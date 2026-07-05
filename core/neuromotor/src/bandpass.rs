/// Second-order IIR bandpass filter (biquad) for neuromotor frequency bands.
///
/// Clinical tremor bands:
///   Physiological tremor:    8–12 Hz
///   Essential tremor:        4–12 Hz
///   Parkinsonian rest tremor: 3–6 Hz
///   Cerebellar tremor:       < 5 Hz
///
/// Filter coefficients computed via bilinear transform of a 2nd-order
/// Butterworth bandpass prototype.
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct BiquadBandpass {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadBandpass {
    /// Design a bandpass biquad at center frequency `f_c` Hz with bandwidth
    /// `bw` Hz at the given sample rate.
    pub fn new(f_c: f32, bw: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * f_c / sample_rate;
        let alpha = w0.sin() * (2.0_f32.ln() / 2.0 * bw / f_c * w0 / w0.sin()).sinh();

        let b0 =  alpha;
        let b1 =  0.0;
        let b2 = -alpha;
        let a0 =  1.0 + alpha;
        let a1 = -2.0 * w0.cos();
        let a2 =  1.0 - alpha;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
              - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x;
        self.y2 = self.y1; self.y1 = y;
        y
    }

    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| self.process(x)).collect()
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0; self.x2 = 0.0;
        self.y1 = 0.0; self.y2 = 0.0;
    }
}

/// Pre-built filters for the three primary clinical tremor bands.
pub struct TremorFilterBank {
    pub parkinsonian: BiquadBandpass, // 3–6 Hz rest tremor
    pub essential:    BiquadBandpass, // 4–12 Hz
    pub physiological: BiquadBandpass, // 8–12 Hz
}

impl TremorFilterBank {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            parkinsonian:  BiquadBandpass::new(4.5, 3.0, sample_rate),
            essential:     BiquadBandpass::new(8.0, 8.0, sample_rate),
            physiological: BiquadBandpass::new(10.0, 4.0, sample_rate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_is_rejected() {
        let mut f = BiquadBandpass::new(5.0, 2.0, 100.0);
        let dc: Vec<f32> = vec![1.0; 500];
        let out = f.process_block(&dc);
        let tail_mean: f32 = out[400..].iter().sum::<f32>() / 100.0;
        assert!(tail_mean.abs() < 0.01, "DC should be rejected: {tail_mean}");
    }
}
