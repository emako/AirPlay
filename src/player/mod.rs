mod decode;
mod eq;
mod fft;

pub use decode::{decode_bytes, decode_file, Decoded};
pub use eq::{EQ_FREQS, PRESETS};
pub use fft::spectrum;

use decode::to_device_stereo;
use eq::{make_chain, Biquad};
use crate::net;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SEEK_NONE: u64 = u64::MAX;
const ST_STOP: u8 = 0;
const ST_PLAY: u8 = 1;
const ST_PAUSE: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub state: PlayState,
    pub position: f64,
    pub duration: f64,
    pub loading: bool,
    pub error: String,
}

struct Shared {
    pcm: Mutex<Arc<Vec<f32>>>,
    pos: AtomicU64,
    start: AtomicU64,
    end: AtomicU64,
    rate: AtomicU32,
    state: AtomicU8,
    volume: AtomicU32,
    seek: AtomicU64,
    ended: AtomicBool,
    loading: AtomicBool,
    repeat_one: AtomicBool,
    eq_on: AtomicBool,
    eq_gen: AtomicU64,
    gains: [AtomicU32; 10],
    wave: Mutex<[f32; 1024]>,
    wave_pos: AtomicU32,
    error: Mutex<String>,
}

impl Shared {
    fn new() -> Self {
        Self {
            pcm: Mutex::new(Arc::new(Vec::new())),
            pos: AtomicU64::new(0),
            start: AtomicU64::new(0),
            end: AtomicU64::new(0),
            rate: AtomicU32::new(0),
            state: AtomicU8::new(ST_STOP),
            volume: AtomicU32::new(0.8f32.to_bits()),
            seek: AtomicU64::new(SEEK_NONE),
            ended: AtomicBool::new(false),
            loading: AtomicBool::new(false),
            repeat_one: AtomicBool::new(false),
            eq_on: AtomicBool::new(false),
            eq_gen: AtomicU64::new(1),
            gains: std::array::from_fn(|_| AtomicU32::new(0f32.to_bits())),
            wave: Mutex::new([0.0; 1024]),
            wave_pos: AtomicU32::new(0),
            error: Mutex::new(String::new()),
        }
    }
}

struct Raw {
    decoded: Decoded,
    start_sec: f64,
    end_sec: Option<f64>,
}

enum Cmd {
    PlayFile {
        path: PathBuf,
        start: f64,
        end: Option<f64>,
    },
    PlayBytes {
        data: Vec<u8>,
        ext: String,
        start: f64,
        end: Option<f64>,
    },
    SetDevice(String),
    SetBuffer { ms: u32, low_latency: bool },
    Fail(String),
    Shutdown,
}

pub struct AudioEngine {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::new());
        let shared_thread = shared.clone();
        std::thread::Builder::new()
            .name("airplay-audio".into())
            .spawn(move || audio_loop(rx, shared_thread))
            .expect("音频线程启动失败");
        Self { tx, shared }
    }

    pub fn play_file(&self, path: PathBuf, start: f64, end: Option<f64>) {
        self.shared.loading.store(true, Ordering::Release);
        self.clear_error();
        let _ = self.tx.send(Cmd::PlayFile { path, start, end });
    }

    pub fn play_url(&self, url: String, start: f64, end: Option<f64>) {
        self.shared.loading.store(true, Ordering::Release);
        self.clear_error();
        let tx = self.tx.clone();
        std::thread::spawn(move || match net::fetch(&url) {
            Ok((data, ext)) => {
                let _ = tx.send(Cmd::PlayBytes {
                    data,
                    ext,
                    start,
                    end,
                });
            }
            Err(e) => {
                let _ = tx.send(Cmd::Fail(e));
            }
        });
    }

    pub fn pause(&self) {
        if self.shared.state.load(Ordering::Relaxed) == ST_PLAY {
            self.shared.state.store(ST_PAUSE, Ordering::Release);
        }
    }

    pub fn resume(&self) {
        if self.shared.state.load(Ordering::Relaxed) == ST_PAUSE {
            self.shared.ended.store(false, Ordering::Release);
            self.shared.state.store(ST_PLAY, Ordering::Release);
        }
    }

    pub fn stop(&self) {
        let start = self.shared.start.load(Ordering::Relaxed);
        self.shared.pos.store(start, Ordering::Release);
        self.shared.state.store(ST_STOP, Ordering::Release);
        self.shared.ended.store(false, Ordering::Release);
        self.shared.loading.store(false, Ordering::Release);
    }

    pub fn seek(&self, rel_sec: f64) {
        let rate = self.shared.rate.load(Ordering::Relaxed);
        if rate == 0 {
            return;
        }
        let start = self.shared.start.load(Ordering::Relaxed);
        let end = self.shared.end.load(Ordering::Relaxed);
        let mut frame = start.saturating_add((rel_sec.max(0.0) * rate as f64) as u64);
        if end > start && frame >= end {
            frame = end - 1;
        }
        self.shared.seek.store(frame, Ordering::Release);
        self.shared.ended.store(false, Ordering::Release);
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn set_eq(&self, on: bool, gains: [f32; 10]) {
        for (slot, gain) in self.shared.gains.iter().zip(gains) {
            slot.store(gain.clamp(-15.0, 15.0).to_bits(), Ordering::Relaxed);
        }
        self.shared.eq_on.store(on, Ordering::Relaxed);
        self.shared.eq_gen.fetch_add(1, Ordering::Release);
    }

    pub fn set_repeat_one(&self, on: bool) {
        self.shared.repeat_one.store(on, Ordering::Relaxed);
    }

    pub fn set_device(&self, name: &str) {
        let _ = self.tx.send(Cmd::SetDevice(name.to_string()));
    }

    pub fn set_buffer(&self, ms: u32, low_latency: bool) {
        let _ = self.tx.send(Cmd::SetBuffer { ms, low_latency });
    }

    pub fn take_ended(&self) -> bool {
        self.shared.ended.swap(false, Ordering::AcqRel)
    }

    pub fn wave(&self) -> [f32; 1024] {
        let Ok(buf) = self.shared.wave.lock() else {
            return [0.0; 1024];
        };
        let pos = self.shared.wave_pos.load(Ordering::Relaxed) as usize % 1024;
        let mut out = [0.0f32; 1024];
        for (i, sample) in out.iter_mut().enumerate() {
            *sample = buf[(pos + i) % 1024];
        }
        out
    }

    pub fn status(&self) -> Status {
        let rate = self.shared.rate.load(Ordering::Relaxed).max(1);
        let start = self.shared.start.load(Ordering::Relaxed);
        let end = self.shared.end.load(Ordering::Relaxed);
        let pos = self.shared.pos.load(Ordering::Relaxed).max(start);
        let position = pos.saturating_sub(start) as f64 / rate as f64;
        let duration = end.saturating_sub(start) as f64 / rate as f64;
        let state = match self.shared.state.load(Ordering::Relaxed) {
            ST_PLAY => PlayState::Playing,
            ST_PAUSE => PlayState::Paused,
            _ => PlayState::Stopped,
        };
        let error = self
            .shared
            .error
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        Status {
            state,
            position,
            duration,
            loading: self.shared.loading.load(Ordering::Relaxed),
            error,
        }
    }

    pub fn devices() -> Vec<String> {
        let host = cpal::default_host();
        host.output_devices()
            .map(|iter| iter.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }

    fn clear_error(&self) {
        if let Ok(mut e) = self.shared.error.lock() {
            e.clear();
        }
    }
}

fn audio_loop(rx: Receiver<Cmd>, sh: Arc<Shared>) {
    let mut raw: Option<Raw> = None;
    let mut stream: Option<cpal::Stream> = None;
    let mut rate = 48_000u32;
    let mut device_name = String::new();
    let mut low_latency = false;
    let mut buffer_ms = 100u32;
    loop {
        let cmd = match rx.recv() {
            Ok(c) => c,
            Err(_) => break,
        };
        let mut cmds = vec![cmd];
        while let Ok(extra) = rx.try_recv() {
            cmds.push(extra);
        }
        for cmd in cmds {
            match cmd {
                Cmd::Shutdown => return,
                Cmd::Fail(msg) => {
                    set_error(&sh, msg);
                    sh.loading.store(false, Ordering::Release);
                    sh.state.store(ST_STOP, Ordering::Release);
                }
                Cmd::SetDevice(name) => {
                    device_name = name;
                    stream = None;
                    if let Some(raw) = raw.as_ref() {
                        match reopen(&device_name, low_latency, buffer_ms, &sh) {
                            Ok((st, r)) => {
                                rate = r;
                                publish(raw, rate, &sh, true);
                                let _ = st.play();
                                stream = Some(st);
                            }
                            Err(e) => set_error(&sh, e),
                        }
                    }
                }
                Cmd::SetBuffer { ms, low_latency: low } => {
                    buffer_ms = ms.clamp(20, 2000);
                    low_latency = low;
                    if raw.is_some() {
                        stream = None;
                        if let Some(raw) = raw.as_ref() {
                            if let Ok((st, r)) = reopen(&device_name, low_latency, buffer_ms, &sh) {
                                rate = r;
                                publish(raw, rate, &sh, true);
                                let _ = st.play();
                                stream = Some(st);
                            }
                        }
                    }
                }
                Cmd::PlayFile { path, start, end } => {
                    sh.loading.store(true, Ordering::Release);
                    match decode_file(&path) {
                        Ok(decoded) => {
                            raw = Some(Raw {
                                decoded,
                                start_sec: start.max(0.0),
                                end_sec: end,
                            });
                            if stream.is_none() {
                                match reopen(&device_name, low_latency, buffer_ms, &sh) {
                                    Ok((st, r)) => {
                                        rate = r;
                                        let _ = st.play();
                                        stream = Some(st);
                                    }
                                    Err(e) => {
                                        set_error(&sh, e);
                                        sh.loading.store(false, Ordering::Release);
                                        continue;
                                    }
                                }
                            }
                            if let Some(raw) = raw.as_ref() {
                                publish(raw, rate, &sh, false);
                            }
                        }
                        Err(e) => {
                            set_error(&sh, e);
                            sh.state.store(ST_STOP, Ordering::Release);
                        }
                    }
                    sh.loading.store(false, Ordering::Release);
                }
                Cmd::PlayBytes {
                    data,
                    ext,
                    start,
                    end,
                } => {
                    sh.loading.store(true, Ordering::Release);
                    match decode_bytes(data, &ext) {
                        Ok(decoded) => {
                            raw = Some(Raw {
                                decoded,
                                start_sec: start.max(0.0),
                                end_sec: end,
                            });
                            if stream.is_none() {
                                match reopen(&device_name, low_latency, buffer_ms, &sh) {
                                    Ok((st, r)) => {
                                        rate = r;
                                        let _ = st.play();
                                        stream = Some(st);
                                    }
                                    Err(e) => {
                                        set_error(&sh, e);
                                        sh.loading.store(false, Ordering::Release);
                                        continue;
                                    }
                                }
                            }
                            if let Some(raw) = raw.as_ref() {
                                publish(raw, rate, &sh, false);
                            }
                        }
                        Err(e) => {
                            set_error(&sh, e);
                            sh.state.store(ST_STOP, Ordering::Release);
                        }
                    }
                    sh.loading.store(false, Ordering::Release);
                }
            }
        }
    }
}

fn set_error(sh: &Shared, msg: String) {
    if let Ok(mut e) = sh.error.lock() {
        *e = msg;
    }
}

fn publish(raw: &Raw, out_rate: u32, sh: &Shared, keep_rel: bool) {
    let rel = if keep_rel {
        let rate = sh.rate.load(Ordering::Relaxed).max(1);
        let pos = sh.pos.load(Ordering::Relaxed);
        let start = sh.start.load(Ordering::Relaxed);
        pos.saturating_sub(start) as f64 / rate as f64
    } else {
        0.0
    };
    let stereo = to_device_stereo(
        &raw.decoded.samples,
        raw.decoded.channels,
        raw.decoded.rate,
        out_rate,
    );
    let frames = stereo.len() / 2;
    let start = ((raw.start_sec * out_rate as f64).round() as u64).min(frames as u64);
    let end = raw
        .end_sec
        .map(|e| (e * out_rate as f64).round() as u64)
        .unwrap_or(frames as u64)
        .min(frames as u64)
        .max(start);
    if let Ok(mut pcm) = sh.pcm.lock() {
        *pcm = Arc::new(stereo);
    }
    sh.rate.store(out_rate, Ordering::Release);
    sh.start.store(start, Ordering::Release);
    sh.end.store(end, Ordering::Release);
    let mut pos = start.saturating_add((rel * out_rate as f64) as u64);
    if end > start && pos >= end {
        pos = end - 1;
    }
    sh.pos.store(pos, Ordering::Release);
    sh.ended.store(false, Ordering::Release);
    sh.state.store(ST_PLAY, Ordering::Release);
    if let Ok(mut e) = sh.error.lock() {
        e.clear();
    }
}

fn reopen(
    device_name: &str,
    low_latency: bool,
    buffer_ms: u32,
    sh: &Arc<Shared>,
) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = if device_name.is_empty() {
        host.default_output_device()
    } else {
        host.output_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.name().ok().as_deref() == Some(device_name)))
            .or_else(|| host.default_output_device())
    }
    .ok_or_else(|| "没有可用的音频输出设备".to_string())?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("无法打开输出设备: {e}"))?;
    let rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let fmt = supported.sample_format();
    let mut config: StreamConfig = supported.into();
    if low_latency {
        let frames = ((rate as u64 * buffer_ms as u64) / 1000).clamp(128, 8192) as u32;
        config.buffer_size = BufferSize::Fixed(frames);
    }
    let built = build_stream(&device, &config, fmt, channels, sh.clone());
    let stream = match built {
        Ok(s) => s,
        Err(_) if low_latency => {
            config.buffer_size = BufferSize::Default;
            build_stream(&device, &config, fmt, channels, sh.clone())
                .map_err(|e| format!("打开音频流失败: {e}"))?
        }
        Err(e) => return Err(format!("打开音频流失败: {e}")),
    };
    Ok((stream, rate))
}

fn build_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    fmt: SampleFormat,
    channels: usize,
    sh: Arc<Shared>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let err = {
        let sh = sh.clone();
        move |e: cpal::StreamError| set_error(&sh, format!("音频输出中断: {e}"))
    };
    match fmt {
        SampleFormat::F32 => stream_of::<f32, _>(device, config, channels, sh, err, |x| x),
        SampleFormat::F64 => stream_of::<f64, _>(device, config, channels, sh, err, |x| x as f64),
        SampleFormat::I16 => {
            stream_of::<i16, _>(device, config, channels, sh, err, |x| {
                (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
            })
        }
        SampleFormat::I32 => {
            stream_of::<i32, _>(device, config, channels, sh, err, |x| {
                (x.clamp(-1.0, 1.0) * i32::MAX as f32) as i32
            })
        }
        SampleFormat::U16 => stream_of::<u16, _>(device, config, channels, sh, err, |x| {
            ((x.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16
        }),
        SampleFormat::U32 => stream_of::<u32, _>(device, config, channels, sh, err, |x| {
            ((x.clamp(-1.0, 1.0) * 0.5 + 0.5) * u32::MAX as f32) as u32
        }),
        SampleFormat::I8 => {
            stream_of::<i8, _>(device, config, channels, sh, err, |x| {
                (x.clamp(-1.0, 1.0) * i8::MAX as f32) as i8
            })
        }
        SampleFormat::U8 => stream_of::<u8, _>(device, config, channels, sh, err, |x| {
            ((x.clamp(-1.0, 1.0) * 0.5 + 0.5) * u8::MAX as f32) as u8
        }),
        _ => stream_of::<f32, _>(device, config, channels, sh, err, |x| x),
    }
}

fn stream_of<T, F>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    sh: Arc<Shared>,
    mut err: impl FnMut(cpal::StreamError) + Send + 'static,
    map: F,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + Send + 'static,
    F: Fn(f32) -> T + Send + 'static,
{
    let mut eqs_l = [Biquad::identity(); 10];
    let mut eqs_r = [Biquad::identity(); 10];
    let mut built = 0u64;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            render(&sh, channels, data, &map, &mut eqs_l, &mut eqs_r, &mut built);
        },
        move |e| err(e),
        Some(Duration::from_millis(200)),
    )
}

fn render<T, F>(
    sh: &Shared,
    channels: usize,
    data: &mut [T],
    map: &F,
    eqs_l: &mut [Biquad; 10],
    eqs_r: &mut [Biquad; 10],
    built: &mut u64,
) where
    F: Fn(f32) -> T,
{
    let channels = channels.max(1);
    let frames = data.len() / channels;
    let seek = sh.seek.swap(SEEK_NONE, Ordering::AcqRel);
    if seek != SEEK_NONE {
        sh.pos.store(seek, Ordering::Release);
    }
    let playing = sh.state.load(Ordering::Relaxed) == ST_PLAY;
    let pcm = sh.pcm.lock().map(|g| g.clone()).unwrap_or_else(|_| Arc::new(Vec::new()));
    let mut pos = sh.pos.load(Ordering::Relaxed) as usize;
    let start = sh.start.load(Ordering::Relaxed) as usize;
    let end = sh.end.load(Ordering::Relaxed) as usize;
    let vol = f32::from_bits(sh.volume.load(Ordering::Relaxed));
    let eq_on = sh.eq_on.load(Ordering::Relaxed);
    let gen = sh.eq_gen.load(Ordering::Relaxed);
    if eq_on && *built != gen {
        let rate = sh.rate.load(Ordering::Relaxed) as f32;
        let mut gains = [0.0f32; 10];
        for (g, slot) in gains.iter_mut().zip(sh.gains.iter()) {
            *g = f32::from_bits(slot.load(Ordering::Relaxed));
        }
        *eqs_l = make_chain(rate, &gains);
        *eqs_r = make_chain(rate, &gains);
        *built = gen;
    }
    let mut mono = [0.0f32; 256];
    let mut mono_n = 0usize;
    for i in 0..frames {
        let (mut l, mut r) = if playing && pos < end && pos * 2 + 1 < pcm.len() {
            let s = (pcm[pos * 2], pcm[pos * 2 + 1]);
            pos += 1;
            s
        } else if playing && pos >= end {
            if sh.repeat_one.load(Ordering::Relaxed) && end > start {
                pos = start;
                if pos * 2 + 1 < pcm.len() {
                    let s = (pcm[pos * 2], pcm[pos * 2 + 1]);
                    pos += 1;
                    s
                } else {
                    (0.0, 0.0)
                }
            } else {
                sh.state.store(ST_STOP, Ordering::Release);
                sh.ended.store(true, Ordering::Release);
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };
        if eq_on {
            for b in eqs_l.iter_mut() {
                l = b.process(l);
            }
            for b in eqs_r.iter_mut() {
                r = b.process(r);
            }
        }
        l = (l * vol).clamp(-1.0, 1.0);
        r = (r * vol).clamp(-1.0, 1.0);
        let base = i * channels;
        if channels == 1 {
            data[base] = map((l + r) * 0.5);
        } else {
            data[base] = map(l);
            data[base + 1] = map(r);
            for c in 2..channels {
                data[base + c] = map(0.0);
            }
        }
        if mono_n < mono.len() {
            mono[mono_n] = (l + r) * 0.5;
            mono_n += 1;
        }
    }
    sh.pos.store(pos as u64, Ordering::Release);
    if mono_n > 0 {
        if let Ok(mut wave) = sh.wave.try_lock() {
            let mut wp = sh.wave_pos.load(Ordering::Relaxed) as usize;
            for s in mono.iter().take(mono_n) {
                wave[wp % 1024] = *s;
                wp += 1;
            }
            sh.wave_pos.store((wp % 1024) as u32, Ordering::Relaxed);
        }
    }
}
