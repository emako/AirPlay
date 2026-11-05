use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

pub fn spectrum(samples: &[f32; 1024], planner: &mut FftPlanner<f32>) -> [f32; 48] {
    let fft = planner.plan_fft_forward(1024);
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 1023.0).cos();
            Complex { re: s * w, im: 0.0 }
        })
        .collect();
    fft.process(&mut buf);
    let mut bars = [0.0f32; 48];
    for (b, bar) in bars.iter_mut().enumerate() {
        let t0 = b as f32 / 48.0;
        let t1 = (b + 1) as f32 / 48.0;
        let i0 = (1.0 * 511.0f32.powf(t0)).round() as usize;
        let i1 = (1.0 * 511.0f32.powf(t1)).round() as usize;
        let i0 = i0.clamp(1, 511);
        let i1 = i1.max(i0).min(511);
        let mut mag = 0.0f32;
        for bin in i0..=i1 {
            let c = buf[bin];
            mag = mag.max((c.re * c.re + c.im * c.im).sqrt());
        }
        *bar = (1.0 + mag).ln().clamp(0.0, 6.0) / 6.0;
    }
    bars
}
