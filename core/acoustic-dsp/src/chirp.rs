/// FMCW (Frequency-Modulated Continuous Wave) chirp generator.
///
/// Emits an ultrasonic sweep from f_start to f_end over `duration_s` seconds.
/// The browser WebAudio API plays this through the speaker. Reflections captured
/// by the microphone are mixed against the transmitted chirp (stretch processing)
/// to produce a beat frequency proportional to target range.
///
/// Default sweep: 18 kHz → 22 kHz (inaudible to most adults, well within
/// standard laptop speaker + microphone bandwidth).
use std::f32::consts::PI;

pub const DEFAULT_F_START: f32 = 18_000.0;
pub const DEFAULT_F_END:   f32 = 22_000.0;
pub const SPEED_OF_SOUND:  f32 = 343.0; // m/s at 20°C

#[derive(Debug, Clone)]
pub struct FmcwChirp {
    pub f_start: f32,
    pub f_end: f32,
    pub duration_s: f32,
    pub sample_rate: f32,
}

impl FmcwChirp {
    pub fn new(f_start: f32, f_end: f32, duration_s: f32, sample_rate: f32) -> Self {
        Self { f_start, f_end, duration_s, sample_rate }
    }

    pub fn default_ultrasonic(sample_rate: f32) -> Self {
        Self::new(DEFAULT_F_START, DEFAULT_F_END, 0.02, sample_rate)
    }

    /// Generate the chirp waveform as a Vec of f32 samples in [-1, 1].
    pub fn generate(&self) -> Vec<f32> {
        let n = (self.duration_s * self.sample_rate) as usize;
        let bandwidth = self.f_end - self.f_start;
        let chirp_rate = bandwidth / self.duration_s;

        (0..n)
            .map(|k| {
                let t = k as f32 / self.sample_rate;
                let phase = 2.0 * PI * (self.f_start * t + 0.5 * chirp_rate * t * t);
                phase.sin()
            })
            .collect()
    }

    /// Range resolution in metres: c / (2 * bandwidth).
    pub fn range_resolution_m(&self) -> f32 {
        SPEED_OF_SOUND / (2.0 * (self.f_end - self.f_start))
    }

    /// Maximum unambiguous range in metres: c * T_chirp / 2.
    pub fn max_range_m(&self) -> f32 {
        SPEED_OF_SOUND * self.duration_s / 2.0
    }

    /// Convert beat frequency (Hz) to range (metres).
    pub fn beat_to_range(&self, beat_hz: f32) -> f32 {
        let chirp_rate = (self.f_end - self.f_start) / self.duration_s;
        (beat_hz * SPEED_OF_SOUND) / (2.0 * chirp_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chirp_length_matches_duration() {
        let c = FmcwChirp::default_ultrasonic(44_100.0);
        let samples = c.generate();
        let expected = (c.duration_s * c.sample_rate) as usize;
        assert_eq!(samples.len(), expected);
    }

    #[test]
    fn range_resolution_is_correct() {
        let c = FmcwChirp::new(18_000.0, 22_000.0, 0.02, 44_100.0);
        let res = c.range_resolution_m();
        // c / (2 * 4000) = 343 / 8000 ≈ 0.0429 m
        assert!((res - 343.0 / 8000.0).abs() < 0.001);
    }

    #[test]
    fn all_samples_in_range() {
        let c = FmcwChirp::default_ultrasonic(44_100.0);
        let s = c.generate();
        assert!(s.iter().all(|&x| x >= -1.001 && x <= 1.001));
    }
}
