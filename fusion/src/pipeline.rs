use neuromotor::{TremorDetector, GaitAnalyzer, UpdrsProxy, TremorResult, GaitResult, UpdrsScore};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionOutput {
    pub tremor: TremorResult,
    pub gait: GaitResult,
    pub updrs: UpdrsScore,
    /// Estimated target range from acoustic FMCW (metres).
    pub range_m: Option<f32>,
    /// Estimated target Doppler velocity from WiFi PCL (m/s).
    pub velocity_ms: Option<f32>,
}

pub struct FusionPipeline {
    tremor_detector: TremorDetector,
    gait_analyzer: GaitAnalyzer,
}

impl FusionPipeline {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            tremor_detector: TremorDetector::new(sample_rate),
            gait_analyzer: GaitAnalyzer::new(sample_rate),
        }
    }

    pub fn process(
        &mut self,
        phase_signal: &[f32],
        range_m: Option<f32>,
        velocity_ms: Option<f32>,
    ) -> FusionOutput {
        let tremor = self.tremor_detector.analyse(phase_signal);
        let gait   = self.gait_analyzer.analyse(phase_signal);
        let updrs  = UpdrsProxy::score(&tremor, &gait);

        FusionOutput { tremor, gait, updrs, range_m, velocity_ms }
    }
}
