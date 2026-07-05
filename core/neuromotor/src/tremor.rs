use crate::bandpass::TremorFilterBank;
use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TremorResult {
    /// Dominant tremor frequency in Hz, or None if below threshold.
    pub dominant_hz: Option<f32>,
    /// Power in the Parkinsonian rest-tremor band (3–6 Hz), normalised.
    pub parkinsonian_power: f32,
    /// Power in the essential tremor band (4–12 Hz), normalised.
    pub essential_power: f32,
    /// Power in the physiological tremor band (8–12 Hz), normalised.
    pub physiological_power: f32,
    /// Likely tremor classification.
    pub classification: TremorClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TremorClass {
    None,
    Physiological,
    Essential,
    Parkinsonian,
    Indeterminate,
}

pub struct TremorDetector {
    sample_rate: f32,
    filter_bank: TremorFilterBank,
    planner: FftPlanner<f32>,
    /// Minimum signal power to attempt classification (avoids noise floor hits).
    power_threshold: f32,
}

impl TremorDetector {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            filter_bank: TremorFilterBank::new(sample_rate),
            planner: FftPlanner::new(),
            power_threshold: 1e-6,
        }
    }

    /// Analyse a block of phase-displacement signal (e.g. unwrapped WiFi CSI
    /// phase, sampled at `sample_rate` Hz). Returns a TremorResult.
    pub fn analyse(&mut self, signal: &[f32]) -> TremorResult {
        let total_power: f32 = signal.iter().map(|&x| x * x).sum::<f32>() / signal.len() as f32;

        if total_power < self.power_threshold {
            return TremorResult {
                dominant_hz: None,
                parkinsonian_power: 0.0,
                essential_power: 0.0,
                physiological_power: 0.0,
                classification: TremorClass::None,
            };
        }

        let park_out = self.filter_bank.parkinsonian.process_block(signal);
        let ess_out  = self.filter_bank.essential.process_block(signal);
        let phys_out = self.filter_bank.physiological.process_block(signal);

        let park_pwr  = band_power(&park_out);
        let ess_pwr   = band_power(&ess_out);
        let phys_pwr  = band_power(&phys_out);

        let norm = (park_pwr + ess_pwr + phys_pwr).max(1e-10);

        let dominant_hz = self.dominant_frequency(signal);
        let classification = classify(park_pwr / norm, ess_pwr / norm, phys_pwr / norm);

        TremorResult {
            dominant_hz,
            parkinsonian_power: park_pwr / norm,
            essential_power: ess_pwr / norm,
            physiological_power: phys_pwr / norm,
            classification,
        }
    }

    fn dominant_frequency(&mut self, signal: &[f32]) -> Option<f32> {
        let n = signal.len().next_power_of_two();
        let mut buf: Vec<Complex<f32>> = signal
            .iter()
            .map(|&x| Complex::new(x, 0.0))
            .collect();
        buf.resize(n, Complex::new(0.0, 0.0));

        let fft = self.planner.plan_fft_forward(n);
        fft.process(&mut buf);

        // Only look at 1–20 Hz (neuromotor range)
        let min_bin = (1.0 * n as f32 / self.sample_rate) as usize;
        let max_bin = (20.0 * n as f32 / self.sample_rate) as usize;

        let (peak_bin, peak_mag) = buf[min_bin..max_bin.min(n / 2)]
            .iter()
            .enumerate()
            .map(|(i, c)| (i + min_bin, c.norm()))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or((0, 0.0));

        if peak_mag < 1e-4 {
            return None;
        }

        Some(peak_bin as f32 * self.sample_rate / n as f32)
    }
}

fn band_power(signal: &[f32]) -> f32 {
    signal.iter().map(|&x| x * x).sum::<f32>() / signal.len().max(1) as f32
}

fn classify(park: f32, ess: f32, phys: f32) -> TremorClass {
    const THRESHOLD: f32 = 0.4;
    if park > THRESHOLD && park > ess && park > phys {
        TremorClass::Parkinsonian
    } else if ess > THRESHOLD && ess > phys {
        TremorClass::Essential
    } else if phys > THRESHOLD {
        TremorClass::Physiological
    } else if park + ess + phys > 0.3 {
        TremorClass::Indeterminate
    } else {
        TremorClass::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn sine_wave(freq_hz: f32, sample_rate: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|k| (2.0 * PI * freq_hz * k as f32 / sample_rate).sin())
            .collect()
    }

    #[test]
    fn detects_parkinsonian_band() {
        let mut det = TremorDetector::new(200.0);
        // 4.5 Hz is centre of parkinsonian band
        let sig = sine_wave(4.5, 200.0, 1024);
        let result = det.analyse(&sig);
        assert_eq!(result.classification, TremorClass::Parkinsonian,
            "4.5 Hz signal should be classified as Parkinsonian");
    }

    #[test]
    fn silent_signal_is_none() {
        let mut det = TremorDetector::new(200.0);
        let sig = vec![0.0f32; 512];
        let result = det.analyse(&sig);
        assert_eq!(result.classification, TremorClass::None);
    }
}
