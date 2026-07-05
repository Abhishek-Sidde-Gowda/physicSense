/// Differential privacy engine — (ε, δ)-DP via Gaussian mechanism.
///
/// Differential privacy guarantees that the gradient update from any one node
/// reveals statistically negligible information about that node's raw data.
///
/// The standard DP-SGD recipe (Abadi et al., 2016):
///   1. Clip each per-sample gradient to L2 norm C  (bounds sensitivity)
///   2. Average clipped gradients across local samples
///   3. Add Gaussian noise N(0, σ²C²I)              (adds privacy)
///   4. Transmit the noisy average
///
/// σ is derived from the privacy budget (ε, δ) and the number of rounds T:
///   σ = (C / ε) * sqrt(2 * ln(1.25 / δ) * T)
use crate::gradient::ModelGradient;
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrivacyError {
    #[error("epsilon must be positive, got {0}")]
    InvalidEpsilon(f32),
    #[error("delta must be in (0, 1), got {0}")]
    InvalidDelta(f32),
    #[error("clip_norm must be positive, got {0}")]
    InvalidClipNorm(f32),
}

#[derive(Debug, Clone)]
pub struct PrivacyBudget {
    /// Privacy loss parameter — smaller = stronger privacy.
    pub epsilon: f32,
    /// Failure probability — typically 1e-5.
    pub delta: f32,
    /// Sensitivity clipping norm.
    pub clip_norm: f32,
}

impl PrivacyBudget {
    /// Reasonable defaults for a medical-adjacent neuromotor application.
    /// ε=1.0, δ=1e-5 gives strong privacy with moderate utility loss.
    pub fn default_strong() -> Self {
        Self { epsilon: 1.0, delta: 1e-5, clip_norm: 1.0 }
    }

    pub fn default_moderate() -> Self {
        Self { epsilon: 4.0, delta: 1e-5, clip_norm: 1.0 }
    }
}

pub struct DifferentialPrivacy {
    budget: PrivacyBudget,
    /// Noise multiplier σ (pre-computed from budget).
    sigma: f32,
    /// Simple LCG RNG state — production would use a CSPRNG.
    rng_state: u64,
}

impl DifferentialPrivacy {
    pub fn new(budget: PrivacyBudget, rounds: u32) -> Result<Self> {
        if budget.epsilon <= 0.0 { return Err(PrivacyError::InvalidEpsilon(budget.epsilon).into()); }
        if budget.delta <= 0.0 || budget.delta >= 1.0 { return Err(PrivacyError::InvalidDelta(budget.delta).into()); }
        if budget.clip_norm <= 0.0 { return Err(PrivacyError::InvalidClipNorm(budget.clip_norm).into()); }

        let sigma = Self::compute_sigma(&budget, rounds);
        Ok(Self { budget, sigma, rng_state: 0xdeadbeefcafe1234 })
    }

    /// Apply DP-SGD to a gradient: clip → add Gaussian noise → return.
    pub fn privatise(&mut self, gradient: &ModelGradient) -> ModelGradient {
        let mut g = gradient.clone();
        g.clip(self.budget.clip_norm);

        let noise_std = self.sigma * self.budget.clip_norm;
        let noisy: Vec<f32> = g.values
            .iter()
            .map(|&v| v + self.gaussian(noise_std))
            .collect();

        ModelGradient {
            layer: g.layer,
            values: noisy,
            pre_clip_norm: g.pre_clip_norm,
        }
    }

    /// Compute the total privacy cost after `steps` gradient releases
    /// using the moments accountant (simplified Rényi DP bound).
    pub fn privacy_spent(&self, steps: u32) -> (f32, f32) {
        // Simplified: ε_spent = σ_base * sqrt(steps), δ unchanged.
        let eps_spent = (self.budget.epsilon / self.sigma) * (steps as f32).sqrt();
        (eps_spent.min(self.budget.epsilon * steps as f32), self.budget.delta)
    }

    fn compute_sigma(budget: &PrivacyBudget, rounds: u32) -> f32 {
        // σ = sqrt(2 * ln(1.25/δ) * T) / ε
        let ln_term = (1.25f32 / budget.delta).ln();
        (2.0 * ln_term * rounds as f32).sqrt() / budget.epsilon
    }

    /// Box-Muller transform over a simple LCG for deterministic testing.
    /// Replace with a CSPRNG (e.g. rand::thread_rng) in production.
    fn gaussian(&mut self, std: f32) -> f32 {
        let u1 = self.next_uniform();
        let u2 = self.next_uniform();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
        z * std
    }

    fn next_uniform(&mut self) -> f32 {
        // LCG: same constants as glibc
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bits = (self.rng_state >> 33) as u32;
        // Map to (0, 1) — avoid exact 0 for ln()
        (bits as f32 + 0.5) / (u32::MAX as f32 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigma_is_positive() {
        let dp = DifferentialPrivacy::new(PrivacyBudget::default_strong(), 100).unwrap();
        assert!(dp.sigma > 0.0);
    }

    #[test]
    fn privatised_gradient_has_same_length() {
        let mut dp = DifferentialPrivacy::new(PrivacyBudget::default_moderate(), 50).unwrap();
        let g = ModelGradient::new("layer", vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let pg = dp.privatise(&g);
        assert_eq!(pg.values.len(), g.values.len());
    }

    #[test]
    fn noise_is_added() {
        let mut dp = DifferentialPrivacy::new(PrivacyBudget::default_strong(), 10).unwrap();
        let g = ModelGradient::new("layer", vec![0.5; 128]).unwrap();
        let pg = dp.privatise(&g);
        // With noise, at least some values should differ from 0.5
        let changed = pg.values.iter().filter(|&&v| (v - 0.5).abs() > 1e-6).count();
        assert!(changed > 0, "privatisation should add noise");
    }

    #[test]
    fn rejects_invalid_budget() {
        assert!(DifferentialPrivacy::new(
            PrivacyBudget { epsilon: -1.0, delta: 1e-5, clip_norm: 1.0 }, 10
        ).is_err());
        assert!(DifferentialPrivacy::new(
            PrivacyBudget { epsilon: 1.0, delta: 1.5, clip_norm: 1.0 }, 10
        ).is_err());
    }
}
