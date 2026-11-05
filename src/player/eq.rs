pub const EQ_FREQS: [f32; 10] = [
    80.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 10000.0, 12000.0, 14000.0,
];

pub const PRESETS: &[(&str, [f32; 10])] = &[
    ("自定义", [0.0; 10]),
    ("流行", [-1.0, 2.0, 4.0, 5.0, 3.0, 0.0, -1.0, -1.0, 1.0, 2.0]),
    ("摇滚", [5.0, 3.0, 2.0, 0.0, -1.0, 1.0, 3.0, 4.0, 4.0, 4.0]),
    ("古典", [4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]),
    ("低音", [7.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("人声", [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0]),
    ("柔和", [2.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0, 2.0]),
    ("平坦", [0.0; 10]),
];

#[derive(Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn peaking(sample_rate: f32, freq: f32, q: f32, gain_db: f32) -> Self {
        if sample_rate <= 0.0 || freq <= 0.0 || gain_db.abs() < 0.05 {
            return Self::identity();
        }
        let freq = freq.min(sample_rate * 0.45);
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = std::f32::consts::TAU * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q.max(0.05));
        let cos = w0.cos();
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / a;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

pub fn make_chain(sample_rate: f32, gains: &[f32; 10]) -> [Biquad; 10] {
    std::array::from_fn(|i| Biquad::peaking(sample_rate, EQ_FREQS[i], 1.0, gains[i]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_db_is_near_unity() {
        let mut chain = make_chain(48_000.0, &[0.0; 10]);
        let mut err = 0.0f32;
        for i in 0..4000 {
            let x = (i as f32 * 1000.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.5;
            let mut y = x;
            for b in chain.iter_mut() {
                y = b.process(y);
            }
            if i > 1000 {
                err = err.max((y - x).abs());
            }
        }
        assert!(err < 0.02, "err {err}");
    }
}
