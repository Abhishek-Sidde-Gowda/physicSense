/// Range-Doppler map builder.
///
/// Accumulates successive cross-correlation vectors (one per CPI) into a 2-D
/// range × Doppler matrix by applying a column-wise FFT across slow-time.
/// This is the standard STAP (Space-Time Adaptive Processing) first stage.
use rustfft::{FftPlanner, num_complex::Complex};

pub struct RangeDopplerMap {
    range_bins: usize,
    doppler_bins: usize,
    slow_time_buffer: Vec<Vec<f32>>,
    planner: FftPlanner<f32>,
}

impl RangeDopplerMap {
    pub fn new(range_bins: usize, doppler_bins: usize) -> Self {
        Self {
            range_bins,
            doppler_bins,
            slow_time_buffer: Vec::with_capacity(doppler_bins),
            planner: FftPlanner::new(),
        }
    }

    /// Push one CPI correlation vector. Returns the full map once `doppler_bins`
    /// CPIs have been accumulated.
    pub fn push_cpi(&mut self, correlation: Vec<f32>) -> Option<Vec<Vec<f32>>> {
        assert_eq!(
            correlation.len(),
            self.range_bins,
            "correlation length must equal range_bins"
        );
        self.slow_time_buffer.push(correlation);

        if self.slow_time_buffer.len() < self.doppler_bins {
            return None;
        }

        let map = self.compute_map();
        self.slow_time_buffer.clear();
        Some(map)
    }

    fn compute_map(&mut self) -> Vec<Vec<f32>> {
        let fft = self.planner.plan_fft_forward(self.doppler_bins);
        let mut map = vec![vec![0.0f32; self.doppler_bins]; self.range_bins];

        for r in 0..self.range_bins {
            let mut column: Vec<Complex<f32>> = (0..self.doppler_bins)
                .map(|d| Complex::new(self.slow_time_buffer[d][r], 0.0))
                .collect();

            fft.process(&mut column);

            let scale = 1.0 / self.doppler_bins as f32;
            for (d, c) in column.iter().enumerate() {
                map[r][d] = c.norm() * scale;
            }
        }

        map
    }

    /// Find the (range_bin, doppler_bin) of the strongest target.
    pub fn peak(map: &[Vec<f32>]) -> (usize, usize) {
        let mut best = (0usize, 0usize);
        let mut best_val = 0.0f32;
        for (r, row) in map.iter().enumerate() {
            for (d, &v) in row.iter().enumerate() {
                if v > best_val {
                    best_val = v;
                    best = (r, d);
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_before_emitting() {
        let mut rdm = RangeDopplerMap::new(64, 8);
        for i in 0..7 {
            let cpi = vec![if i == 0 { 1.0 } else { 0.0 }; 64];
            assert!(rdm.push_cpi(cpi).is_none());
        }
        let cpi = vec![0.0f32; 64];
        let map = rdm.push_cpi(cpi);
        assert!(map.is_some());
        let m = map.unwrap();
        assert_eq!(m.len(), 64);
        assert_eq!(m[0].len(), 8);
    }
}
