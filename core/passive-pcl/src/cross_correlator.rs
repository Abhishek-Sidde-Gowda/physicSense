/// Passive coherent location relies on cross-correlating a surveillance channel
/// (reflected signal) against a reference channel (direct-path signal from the
/// ambient transmitter). The peak in the cross-correlation gives bistatic delay,
/// which maps directly to bistatic range.
///
/// This implements the frequency-domain cross-correlation via FFT for O(N log N)
/// complexity instead of the naive O(N²) time-domain approach.
use anyhow::{ensure, Result};
use num_complex::Complex32;
use rustfft::{FftPlanner, num_complex::Complex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PclError {
    #[error("reference and surveillance buffers must have equal length, got {0} vs {1}")]
    LengthMismatch(usize, usize),
    #[error("buffer length must be a power of two, got {0}")]
    NotPowerOfTwo(usize),
}

pub struct CrossCorrelator {
    planner: FftPlanner<f32>,
    n: usize,
}

impl CrossCorrelator {
    pub fn new(n: usize) -> Result<Self> {
        ensure!(n.is_power_of_two(), PclError::NotPowerOfTwo(n));
        Ok(Self {
            planner: FftPlanner::new(),
            n,
        })
    }

    /// Compute the circular cross-correlation of `reference` and `surveillance`.
    /// Returns a vector of length `n` where index `k` is the correlation at lag `k`.
    pub fn correlate(
        &mut self,
        reference: &[Complex32],
        surveillance: &[Complex32],
    ) -> Result<Vec<f32>> {
        ensure!(
            reference.len() == surveillance.len(),
            PclError::LengthMismatch(reference.len(), surveillance.len())
        );
        ensure!(
            reference.len() == self.n,
            PclError::LengthMismatch(reference.len(), self.n)
        );

        let fft = self.planner.plan_fft_forward(self.n);
        let ifft = self.planner.plan_fft_inverse(self.n);

        // Convert to rustfft Complex (same layout, just re-cast)
        let mut ref_buf: Vec<Complex<f32>> = reference
            .iter()
            .map(|c| Complex::new(c.re, c.im))
            .collect();
        let mut sur_buf: Vec<Complex<f32>> = surveillance
            .iter()
            .map(|c| Complex::new(c.re, c.im))
            .collect();

        fft.process(&mut ref_buf);
        fft.process(&mut sur_buf);

        // Cross-power spectrum: normalise to unit magnitude per bin to avoid
        // frequency-dependent weighting (GCC-PHAT: generalised cross-correlation
        // with phase transform). This makes the peak location robust to the
        // spectral shape of the ambient WiFi signal.
        let mut cross: Vec<Complex<f32>> = ref_buf
            .iter()
            .zip(sur_buf.iter())
            .map(|(r, s)| {
                let raw = s * r.conj();
                let mag = raw.norm().max(1e-10);
                Complex::new(raw.re / mag, raw.im / mag)
            })
            .collect();

        ifft.process(&mut cross);

        let scale = 1.0 / self.n as f32;
        Ok(cross.iter().map(|c| c.norm() * scale).collect())
    }

    /// Return the lag index with the highest correlation (peak bistatic delay).
    pub fn peak_lag(&mut self, reference: &[Complex32], surveillance: &[Complex32]) -> Result<usize> {
        let corr = self.correlate(reference, surveillance)?;
        Ok(corr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn complex_sine(n: usize, freq_bin: usize) -> Vec<Complex32> {
        (0..n)
            .map(|k| {
                let phase = 2.0 * PI * freq_bin as f32 * k as f32 / n as f32;
                Complex32::new(phase.cos(), phase.sin())
            })
            .collect()
    }

    #[test]
    fn zero_lag_identical_signals() {
        let n = 256;
        let mut cc = CrossCorrelator::new(n).unwrap();
        let sig = complex_sine(n, 10);
        let peak = cc.peak_lag(&sig, &sig).unwrap();
        assert_eq!(peak, 0, "identical signals should peak at lag 0");
    }

    #[test]
    fn detects_known_delay() {
        let n = 512;
        let delay = 17usize;
        let mut cc = CrossCorrelator::new(n).unwrap();

        let reference = complex_sine(n, 8);
        // Circular shift reference by `delay` samples to simulate surveillance
        let surveillance: Vec<Complex32> = (0..n)
            .map(|k| reference[(k + n - delay) % n])
            .collect();

        let peak = cc.peak_lag(&reference, &surveillance).unwrap();
        assert_eq!(peak, delay, "should detect injected delay of {delay} samples");
    }

    #[test]
    fn rejects_mismatched_lengths() {
        let mut cc = CrossCorrelator::new(256).unwrap();
        let a = vec![Complex32::new(0.0, 0.0); 256];
        let b = vec![Complex32::new(0.0, 0.0); 128];
        assert!(cc.correlate(&a, &b).is_err());
    }
}
