/// Bistatic Doppler extraction from a sequence of cross-correlation peaks.
///
/// In passive coherent location, Doppler shift is measured by computing the
/// phase rotation of the surveillance signal relative to the reference over
/// successive CPI (coherent processing intervals). A moving person in the
/// bistatic footprint creates a Doppler signature proportional to their
/// velocity component along the bistatic bisector.
use num_complex::Complex32;
use std::f32::consts::PI;

/// One coherent processing interval result.
#[derive(Debug, Clone)]
pub struct CpiResult {
    /// Estimated Doppler frequency in Hz.
    pub doppler_hz: f32,
    /// Bistatic delay at peak correlation (samples).
    pub peak_lag: usize,
    /// Correlation magnitude at peak.
    pub peak_magnitude: f32,
}

pub struct DopplerExtractor {
    sample_rate_hz: f32,
    cpi_samples: usize,
    history: Vec<Complex32>,
}

impl DopplerExtractor {
    pub fn new(sample_rate_hz: f32, cpi_samples: usize) -> Self {
        Self {
            sample_rate_hz,
            cpi_samples,
            history: Vec::new(),
        }
    }

    /// Feed a cross-correlation peak complex value from successive CPIs.
    /// Returns Doppler estimate once enough history is accumulated.
    pub fn push_peak(&mut self, peak: Complex32) -> Option<f32> {
        self.history.push(peak);
        if self.history.len() < 2 {
            return None;
        }

        // Instantaneous phase difference between consecutive CPIs
        let prev = self.history[self.history.len() - 2];
        let curr = self.history[self.history.len() - 1];

        // Phase rotation: angle of (curr * conj(prev))
        let rotation = curr * prev.conj();
        let phase_diff = rotation.im.atan2(rotation.re);

        // Doppler = phase_diff / (2π * T_cpi)
        let t_cpi = self.cpi_samples as f32 / self.sample_rate_hz;
        let doppler_hz = phase_diff / (2.0 * PI * t_cpi);

        Some(doppler_hz)
    }

    /// Estimate velocity from bistatic Doppler.
    /// `wavelength_m`: carrier wavelength in metres (2.4 GHz WiFi ≈ 0.125 m).
    /// `bistatic_angle_rad`: angle at the target in the bistatic triangle.
    pub fn doppler_to_velocity(doppler_hz: f32, wavelength_m: f32, bistatic_angle_rad: f32) -> f32 {
        // v = (f_d * λ) / (2 * cos(β/2))
        // where β is the bistatic angle
        let beta_half_cos = (bistatic_angle_rad / 2.0).cos().max(1e-6);
        (doppler_hz * wavelength_m) / (2.0 * beta_half_cos)
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doppler_to_velocity_monostatic() {
        // Monostatic case: bistatic_angle = 0, cos(0) = 1
        // v = f_d * λ / 2
        let wavelength = 0.125; // 2.4 GHz
        let doppler_hz = 16.0;
        let v = DopplerExtractor::doppler_to_velocity(doppler_hz, wavelength, 0.0);
        let expected = doppler_hz * wavelength / 2.0;
        assert!((v - expected).abs() < 1e-4);
    }

    #[test]
    fn push_peak_returns_none_on_first() {
        let mut de = DopplerExtractor::new(2_000_000.0, 1024);
        let result = de.push_peak(Complex32::new(1.0, 0.0));
        assert!(result.is_none());
    }

    #[test]
    fn detects_stationary_target() {
        let mut de = DopplerExtractor::new(2_000_000.0, 1024);
        de.push_peak(Complex32::new(1.0, 0.0));
        // Same phase → zero Doppler
        let d = de.push_peak(Complex32::new(1.0, 0.0)).unwrap();
        assert!(d.abs() < 0.1, "stationary target should yield near-zero Doppler");
    }
}
