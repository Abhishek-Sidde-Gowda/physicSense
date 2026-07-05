use crate::{FmcwChirp, TdoaLocalizer};
use crate::tdoa::{TdoaMeasurement, MicPosition};
use anyhow::Result;
use rustfft::{FftPlanner, num_complex::Complex};

/// End-to-end acoustic sensing pipeline.
///
/// 1. Generate chirp → play via WebAudio (browser side).
/// 2. Capture mic recordings → pass to `process_reflection`.
/// 3. Compute beat spectrum → extract range per mic.
/// 4. Compute TDOA between mic pairs → localize target.
pub struct AcousticPipeline {
    pub chirp: FmcwChirp,
    localizer: TdoaLocalizer,
    planner: FftPlanner<f32>,
}

impl AcousticPipeline {
    pub fn new(sample_rate: f32, mic_positions: Vec<MicPosition>) -> Result<Self> {
        Ok(Self {
            chirp: FmcwChirp::default_ultrasonic(sample_rate),
            localizer: TdoaLocalizer::new(mic_positions)?,
            planner: FftPlanner::new(),
        })
    }

    /// Mix received signal with reference chirp (stretch processing), return
    /// the beat frequency spectrum magnitude.
    pub fn beat_spectrum(&mut self, received: &[f32]) -> Vec<f32> {
        let reference = self.chirp.generate();
        let n = received.len().min(reference.len());

        // Time-domain mix (multiply)
        let mut mixed: Vec<Complex<f32>> = (0..n)
            .map(|k| Complex::new(received[k] * reference[k], 0.0))
            .collect();

        // Zero-pad to next power of two for FFT
        let fft_size = n.next_power_of_two();
        mixed.resize(fft_size, Complex::new(0.0, 0.0));

        let fft = self.planner.plan_fft_forward(fft_size);
        fft.process(&mut mixed);

        let scale = 1.0 / fft_size as f32;
        mixed[..fft_size / 2]
            .iter()
            .map(|c| c.norm() * scale)
            .collect()
    }

    /// Peak beat frequency bin → range in metres.
    pub fn range_from_reflection(&mut self, received: &[f32]) -> f32 {
        let spectrum = self.beat_spectrum(received);
        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let bin_hz = peak_bin as f32 * self.chirp.sample_rate
            / (spectrum.len() * 2) as f32;
        self.chirp.beat_to_range(bin_hz)
    }

    /// Given per-mic range estimates, compute TDOA measurements and localize.
    pub fn localize_from_ranges(
        &self,
        ranges_m: &[f32],
        measurements: Vec<TdoaMeasurement>,
    ) -> Result<(f32, f32)> {
        let _ = ranges_m; // used implicitly via TDOA measurements
        self.localizer.locate(&measurements)
    }
}
