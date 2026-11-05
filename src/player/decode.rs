use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Clone, Debug)]
pub struct Decoded {
    pub rate: u32,
    pub channels: usize,
    pub samples: Vec<f32>,
}

pub fn decode_file(path: &Path) -> Result<Decoded, String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    decode_reader(Box::new(file), &ext)
}

pub fn decode_bytes(data: Vec<u8>, ext: &str) -> Result<Decoded, String> {
    decode_reader(Box::new(Cursor::new(data)), ext)
}

fn decode_reader(source: Box<dyn symphonia::core::io::MediaSource>, ext: &str) -> Result<Decoded, String> {
    let mss = MediaSourceStream::new(source, Default::default());
    let mut hint = Hint::new();
    if !ext.is_empty() {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("无法识别音频: {e}"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "没有音频轨".to_string())?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("解码器不支持该格式: {e}"))?;
    let mut samples = Vec::<f32>::new();
    let mut rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(0);
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(_)) => break,
            Err(_) => break,
        };
        if packet.track_id() != track.id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                channels = spec.channels.count();
                if sample_buf.is_none() {
                    sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                }
                let buf = sample_buf.as_mut().unwrap();
                buf.copy_interleaved_ref(decoded);
                samples.extend_from_slice(buf.samples());
            }
            Err(SymError::DecodeError(_)) => continue,
            Err(SymError::ResetRequired) => decoder.reset(),
            Err(_) => break,
        }
    }
    if samples.is_empty() || rate == 0 || channels == 0 {
        return Err("没有解出采样".into());
    }
    Ok(Decoded {
        rate,
        channels,
        samples,
    })
}

pub fn to_device_stereo(input: &[f32], channels: usize, in_rate: u32, out_rate: u32) -> Vec<f32> {
    let channels = channels.max(1);
    if input.is_empty() || in_rate == 0 || out_rate == 0 {
        return Vec::new();
    }
    let in_frames = input.len() / channels;
    if in_frames == 0 {
        return Vec::new();
    }
    let out_frames = ((in_frames as u64 * out_rate as u64) / in_rate as u64).max(1) as usize;
    let mut out = vec![0.0f32; out_frames * 2];
    let ch_r = if channels == 1 { 0 } else { 1.min(channels - 1) };
    for i in 0..out_frames {
        let src = i as f64 * in_rate as f64 / out_rate as f64;
        let i0 = (src.floor() as usize).min(in_frames - 1);
        let i1 = (i0 + 1).min(in_frames - 1);
        let t = (src - i0 as f64) as f32;
        let sample = |idx: usize, ch: usize| input[idx * channels + ch];
        let l = sample(i0, 0) * (1.0 - t) + sample(i1, 0) * t;
        let r = sample(i0, ch_r) * (1.0 - t) + sample(i1, ch_r) * t;
        out[i * 2] = l;
        out[i * 2 + 1] = r;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_wav() -> Vec<u8> {
        let rate = 8000u32;
        let n = 1600u32;
        let mut pcm = Vec::new();
        for i in 0..n {
            let s = ((i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 8000.0) as i16;
            pcm.extend_from_slice(&s.to_le_bytes());
        }
        let data_len = pcm.len() as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend_from_slice(&pcm);
        v
    }

    #[test]
    fn decodes_wav_and_resamples() {
        let decoded = decode_bytes(tone_wav(), "wav").unwrap();
        assert_eq!(decoded.rate, 8000);
        assert_eq!(decoded.channels, 1);
        assert!(decoded.samples.iter().any(|s| s.abs() > 0.05));
        let stereo = to_device_stereo(&decoded.samples, 1, 8000, 16000);
        assert_eq!(stereo.len(), decoded.samples.len() * 2 * 2);
    }
}
