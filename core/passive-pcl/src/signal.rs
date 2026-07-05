use num_complex::Complex32;
use serde::{Deserialize, Serialize};

/// One IQ sample from a passive WiFi receiver.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct IqSample {
    pub i: f32,
    pub q: f32,
}

impl IqSample {
    pub fn new(i: f32, q: f32) -> Self {
        Self { i, q }
    }

    pub fn to_complex(self) -> Complex32 {
        Complex32::new(self.i, self.q)
    }

    pub fn magnitude(self) -> f32 {
        (self.i * self.i + self.q * self.q).sqrt()
    }
}

/// Rolling buffer of IQ samples with fixed capacity.
pub struct SignalBuffer {
    data: Vec<Complex32>,
    capacity: usize,
    head: usize,
    len: usize,
}

impl SignalBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![Complex32::new(0.0, 0.0); capacity],
            capacity,
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, sample: IqSample) {
        self.data[self.head] = sample.to_complex();
        self.head = (self.head + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    pub fn as_contiguous(&self) -> Vec<Complex32> {
        if self.len < self.capacity {
            return self.data[..self.len].to_vec();
        }
        let mut out = Vec::with_capacity(self.capacity);
        out.extend_from_slice(&self.data[self.head..]);
        out.extend_from_slice(&self.data[..self.head]);
        out
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_wraps_correctly() {
        let mut buf = SignalBuffer::new(4);
        for i in 0..6u32 {
            buf.push(IqSample::new(i as f32, 0.0));
        }
        let out = buf.as_contiguous();
        assert_eq!(out.len(), 4);
        // last 4 values: 2,3,4,5
        assert_eq!(out[0].re, 2.0);
        assert_eq!(out[3].re, 5.0);
    }

    #[test]
    fn magnitude_is_correct() {
        let s = IqSample::new(3.0, 4.0);
        assert!((s.magnitude() - 5.0).abs() < 1e-5);
    }
}
