use crate::{TremorResult, GaitResult};
use crate::tremor::TremorClass;
use serde::{Deserialize, Serialize};

/// UPDRS (Unified Parkinson's Disease Rating Scale) proxy scorer.
///
/// This is NOT a clinical diagnosis. It maps PhysicSense signal features to
/// UPDRS motor subscale items 20 (rest tremor) and 29–30 (gait) using
/// published regression coefficients from:
///
///   Tzallas et al. "Automated Evaluation of Abnormal Involuntary Movements."
///   Sensors, 2014.
///
/// Output should be interpreted as a screening indicator only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdrsScore {
    /// Tremor subscore (0–4, mirrors UPDRS item 20).
    pub tremor_subscore: f32,
    /// Gait subscore (0–4, mirrors UPDRS items 29–30 combined).
    pub gait_subscore: f32,
    /// Composite motor score (0–8).
    pub composite: f32,
    /// Screening flag: true if composite ≥ 3.0.
    pub flag_for_review: bool,
}

pub struct UpdrsProxy;

impl UpdrsProxy {
    pub fn score(tremor: &TremorResult, gait: &GaitResult) -> UpdrsScore {
        let tremor_subscore = Self::score_tremor(tremor);
        let gait_subscore   = Self::score_gait(gait);
        let composite       = tremor_subscore + gait_subscore;

        UpdrsScore {
            tremor_subscore,
            gait_subscore,
            composite,
            flag_for_review: composite >= 3.0,
        }
    }

    fn score_tremor(t: &TremorResult) -> f32 {
        match t.classification {
            TremorClass::None         => 0.0,
            TremorClass::Physiological => 0.5,
            TremorClass::Indeterminate => 1.0,
            TremorClass::Essential     => 1.5,
            TremorClass::Parkinsonian  => {
                // Scale 1–4 based on Parkinsonian band power
                1.0 + (t.parkinsonian_power * 3.0).min(3.0)
            }
        }
    }

    fn score_gait(g: &GaitResult) -> f32 {
        if g.cadence_spm < 1.0 {
            return 0.0;
        }

        let mut score = 0.0f32;

        // Asymmetry contribution (0–1.5)
        score += (g.asymmetry_index * 1.5).min(1.5);

        // Stride variability contribution (0–1.5)
        score += ((g.stride_cv_pct / 30.0) * 1.5).min(1.5);

        // FOG risk contribution (0–1.0)
        score += g.fog_risk;

        score.min(4.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tremor::TremorClass;

    fn healthy_tremor() -> TremorResult {
        TremorResult {
            dominant_hz: None,
            parkinsonian_power: 0.0,
            essential_power: 0.0,
            physiological_power: 0.1,
            classification: TremorClass::None,
        }
    }

    fn healthy_gait() -> GaitResult {
        GaitResult {
            cadence_spm: 110.0,
            asymmetry_index: 0.02,
            stride_cv_pct: 3.0,
            fog_risk: 0.05,
        }
    }

    #[test]
    fn healthy_profile_does_not_flag() {
        let score = UpdrsProxy::score(&healthy_tremor(), &healthy_gait());
        assert!(!score.flag_for_review, "healthy profile should not be flagged");
        assert!(score.composite < 3.0);
    }

    #[test]
    fn parkinsonian_profile_flags() {
        let tremor = TremorResult {
            dominant_hz: Some(4.5),
            parkinsonian_power: 0.85,
            essential_power: 0.10,
            physiological_power: 0.05,
            classification: TremorClass::Parkinsonian,
        };
        let gait = GaitResult {
            cadence_spm: 95.0,
            asymmetry_index: 0.35,
            stride_cv_pct: 22.0,
            fog_risk: 0.45,
        };
        let score = UpdrsProxy::score(&tremor, &gait);
        assert!(score.flag_for_review, "Parkinsonian profile should be flagged");
        assert!(score.composite >= 3.0);
    }
}
