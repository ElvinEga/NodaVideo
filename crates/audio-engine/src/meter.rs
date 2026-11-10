//! Audio metering and peak/RMS level calculation for broadcast standards.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;

/// Measured audio signal levels in decibels relative to full scale (dBFS).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioLevels {
    /// Peak level on the left channel in dBFS (typically `<= 0.0`).
    pub peak_db_left: f32,
    /// Peak level on the right channel in dBFS.
    pub peak_db_right: f32,
    /// Root Mean Square (RMS) energy on the left channel in dBFS.
    pub rms_db_left: f32,
    /// Root Mean Square (RMS) energy on the right channel in dBFS.
    pub rms_db_right: f32,
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            peak_db_left: -100.0,
            peak_db_right: -100.0,
            rms_db_left: -100.0,
            rms_db_right: -100.0,
        }
    }
}

/// Convert linear amplitude `[0.0, 1.0]` into decibels relative to full scale (dBFS).
#[must_use]
pub fn linear_to_dbfs(amplitude: f32) -> f32 {
    if amplitude <= 0.000_01 {
        -100.0
    } else {
        (20.0 * amplitude.log10()).max(-100.0)
    }
}

/// Audio meter analyzer computing peak and RMS levels across stereo buffers.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioMeter;

impl AudioMeter {
    /// Calculate peak and RMS levels for the given audio buffer.
    #[must_use]
    pub fn compute_levels(buffer: &AudioBuffer) -> AudioLevels {
        let frame_count = buffer.frame_count();
        if frame_count == 0 {
            return AudioLevels::default();
        }

        let mut max_abs_left: f32 = 0.0;
        let mut max_abs_right: f32 = 0.0;
        let mut sum_sq_left: f64 = 0.0;
        let mut sum_sq_right: f64 = 0.0;

        let channels = buffer.format.channels;
        for i in 0..frame_count {
            let left = buffer.get_sample(i, 0);
            let right = if channels > 1 {
                buffer.get_sample(i, 1)
            } else {
                left
            };

            let abs_l = left.abs();
            let abs_r = right.abs();

            if abs_l > max_abs_left {
                max_abs_left = abs_l;
            }
            if abs_r > max_abs_right {
                max_abs_right = abs_r;
            }

            sum_sq_left = (left as f64).mul_add(left as f64, sum_sq_left);
            sum_sq_right = (right as f64).mul_add(right as f64, sum_sq_right);
        }

        let rms_left = (sum_sq_left / frame_count as f64).sqrt() as f32;
        let rms_right = (sum_sq_right / frame_count as f64).sqrt() as f32;

        AudioLevels {
            peak_db_left: linear_to_dbfs(max_abs_left),
            peak_db_right: linear_to_dbfs(max_abs_right),
            rms_db_left: linear_to_dbfs(rms_left),
            rms_db_right: linear_to_dbfs(rms_right),
        }
    }
}
