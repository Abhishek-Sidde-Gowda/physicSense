/// Time-Difference-of-Arrival (TDOA) localizer for multi-microphone setups.
///
/// Given N microphone positions and the arrival time difference of a reflected
/// acoustic pulse at each pair, we solve for the 2-D target position using
/// hyperbolic multilateration. A minimum of 3 microphones (e.g. laptop corners
/// or phone + earbuds) gives an unambiguous 2-D fix.
use anyhow::{ensure, Result};
use thiserror::Error;

pub const SPEED_OF_SOUND: f32 = 343.0;

#[derive(Debug, Clone, Copy)]
pub struct MicPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TdoaMeasurement {
    /// Index of reference microphone.
    pub ref_mic: usize,
    /// Index of secondary microphone.
    pub sec_mic: usize,
    /// Arrival time difference in seconds (positive = ref arrived earlier).
    pub delta_t: f32,
}

#[derive(Debug, Error)]
pub enum TdoaError {
    #[error("need at least 3 microphones, got {0}")]
    TooFewMics(usize),
    #[error("need at least 2 TDOA measurements, got {0}")]
    TooFewMeasurements(usize),
    #[error("microphone index {0} out of range")]
    BadMicIndex(usize),
    #[error("solver did not converge after {0} iterations")]
    NoConvergence(usize),
}

pub struct TdoaLocalizer {
    mics: Vec<MicPosition>,
}

impl TdoaLocalizer {
    pub fn new(mics: Vec<MicPosition>) -> Result<Self> {
        ensure!(mics.len() >= 3, TdoaError::TooFewMics(mics.len()));
        Ok(Self { mics })
    }

    /// Estimate 2-D position via iterative least-squares (Gauss-Newton).
    /// Initial guess: centroid of microphone positions.
    pub fn locate(&self, measurements: &[TdoaMeasurement]) -> Result<(f32, f32)> {
        ensure!(
            measurements.len() >= 2,
            TdoaError::TooFewMeasurements(measurements.len())
        );
        for m in measurements {
            ensure!(m.ref_mic < self.mics.len(), TdoaError::BadMicIndex(m.ref_mic));
            ensure!(m.sec_mic < self.mics.len(), TdoaError::BadMicIndex(m.sec_mic));
        }

        // Start at the centroid
        let mut x: f32 = self.mics.iter().map(|m| m.x).sum::<f32>() / self.mics.len() as f32;
        let mut y: f32 = self.mics.iter().map(|m| m.y).sum::<f32>() / self.mics.len() as f32;

        const MAX_ITER: usize = 50;
        const TOL: f32 = 1e-5;

        for iter in 0..MAX_ITER {
            let (dx, dy, step) = self.gauss_newton_step(x, y, measurements);
            x += dx;
            y += dy;
            if step < TOL {
                return Ok((x, y));
            }
            if iter == MAX_ITER - 1 {
                return Err(TdoaError::NoConvergence(MAX_ITER).into());
            }
        }
        Ok((x, y))
    }

    fn gauss_newton_step(
        &self,
        x: f32,
        y: f32,
        measurements: &[TdoaMeasurement],
    ) -> (f32, f32, f32) {
        let mut jtj = [[0.0f32; 2]; 2];
        let mut jtr = [0.0f32; 2];

        for m in measurements {
            let ref_pos = &self.mics[m.ref_mic];
            let sec_pos = &self.mics[m.sec_mic];

            let d_ref = ((x - ref_pos.x).powi(2) + (y - ref_pos.y).powi(2)).sqrt().max(1e-6);
            let d_sec = ((x - sec_pos.x).powi(2) + (y - sec_pos.y).powi(2)).sqrt().max(1e-6);

            let predicted_tdoa = (d_ref - d_sec) / SPEED_OF_SOUND;
            let residual = m.delta_t - predicted_tdoa;

            // Jacobian row
            let jx = (x - ref_pos.x) / (SPEED_OF_SOUND * d_ref)
                - (x - sec_pos.x) / (SPEED_OF_SOUND * d_sec);
            let jy = (y - ref_pos.y) / (SPEED_OF_SOUND * d_ref)
                - (y - sec_pos.y) / (SPEED_OF_SOUND * d_sec);

            jtj[0][0] += jx * jx;
            jtj[0][1] += jx * jy;
            jtj[1][0] += jy * jx;
            jtj[1][1] += jy * jy;
            jtr[0] += jx * residual;
            jtr[1] += jy * residual;
        }

        // Solve 2×2 system
        let det = jtj[0][0] * jtj[1][1] - jtj[0][1] * jtj[1][0];
        if det.abs() < 1e-10 {
            return (0.0, 0.0, 0.0);
        }
        let dx = (jtj[1][1] * jtr[0] - jtj[0][1] * jtr[1]) / det;
        let dy = (jtj[0][0] * jtr[1] - jtj[1][0] * jtr[0]) / det;
        let step = (dx * dx + dy * dy).sqrt();
        (dx, dy, step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_mics() -> Vec<MicPosition> {
        vec![
            MicPosition { x: 0.0, y: 0.0 },
            MicPosition { x: 1.0, y: 0.0 },
            MicPosition { x: 0.5, y: 0.866 },
        ]
    }

    #[test]
    fn locates_centre_of_triangle() {
        let localizer = TdoaLocalizer::new(triangle_mics()).unwrap();
        // Target at centroid (0.5, 0.289) — all distances equal → all TDOA = 0
        let measurements = vec![
            TdoaMeasurement { ref_mic: 0, sec_mic: 1, delta_t: 0.0 },
            TdoaMeasurement { ref_mic: 0, sec_mic: 2, delta_t: 0.0 },
        ];
        let (x, y) = localizer.locate(&measurements).unwrap();
        assert!((x - 0.5).abs() < 0.05, "x={x}");
        assert!(y >= 0.0 && y < 0.5, "y={y}");
    }

    #[test]
    fn rejects_too_few_mics() {
        let result = TdoaLocalizer::new(vec![
            MicPosition { x: 0.0, y: 0.0 },
            MicPosition { x: 1.0, y: 0.0 },
        ]);
        assert!(result.is_err());
    }
}
