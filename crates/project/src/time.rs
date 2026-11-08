//! Rational time and frame-accurate timing primitives.
//!
//! NodaVideo never uses floating-point seconds internally for timeline or media timing.
//! Time is stored as rational numbers (`TimeRational`) or integer frames.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Sub};

/// Greatest common divisor via Euclidean algorithm.
const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Rational time representation storing exact integer numerator and denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeRational {
    /// Time numerator (in timebase units, e.g. ticks or microseconds).
    pub numerator: i64,
    /// Time denominator (ticks per second). Must be non-zero.
    pub denominator: u32,
}

impl TimeRational {
    /// Zero duration.
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    /// Create a new rational time, reduced to simplest form.
    pub fn new(numerator: i64, denominator: u32) -> Self {
        let denom = if denominator == 0 { 1 } else { denominator };
        let g = gcd(numerator.unsigned_abs(), denom as u64) as u32;
        let g = if g == 0 { 1 } else { g };

        Self {
            numerator: numerator / (g as i64),
            denominator: denom / g,
        }
    }

    /// Convert from frames at a specified frame rate.
    pub fn from_frames(frames: i64, fps: FrameRate) -> Self {
        Self::new(frames * (fps.denominator as i64), fps.numerator)
    }

    /// Calculate the nearest integer frame count at a specified frame rate.
    pub fn to_frames(&self, fps: FrameRate) -> i64 {
        if self.denominator == 0 {
            return 0;
        }
        // frames = (num / den) * (fps.num / fps.den)
        let total_num = (self.numerator as i128) * (fps.numerator as i128);
        let total_den = (self.denominator as i128) * (fps.denominator as i128);
        if total_den == 0 {
            return 0;
        }
        (total_num / total_den) as i64
    }

    /// Convert from boundary floating-point seconds (e.g. from FFprobe JSON output).
    pub fn from_seconds_f64(sec: f64, timebase: u32) -> Self {
        if !sec.is_finite() || sec < 0.0 {
            return Self::ZERO;
        }
        let ticks = (sec * f64::from(timebase)).round() as i64;
        Self::new(ticks, timebase)
    }

    /// Convert to floating-point seconds for boundary presentation / diagnostics.
    pub fn to_seconds_f64(&self) -> f64 {
        if self.denominator == 0 {
            0.0
        } else {
            (self.numerator as f64) / f64::from(self.denominator)
        }
    }

    /// Format as SMPTE non-drop-frame timecode `HH:MM:SS:FF`.
    pub fn to_timecode(&self, fps: FrameRate) -> String {
        let total_frames = self.to_frames(fps).max(0);
        let fps_int = fps.rounded_int().max(1);

        let frames = total_frames % fps_int;
        let total_seconds = total_frames / fps_int;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = total_minutes / 60;

        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    }
}

impl Default for TimeRational {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialOrd for TimeRational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeRational {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = (self.numerator as i128) * (other.denominator as i128);
        let rhs = (other.numerator as i128) * (self.denominator as i128);
        lhs.cmp(&rhs)
    }
}

impl Add for TimeRational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let num = (self.numerator as i128) * (rhs.denominator as i128)
            + (rhs.numerator as i128) * (self.denominator as i128);
        let den = (self.denominator as u64) * (rhs.denominator as u64);
        if den > u64::from(u32::MAX) {
            // Scale down if denominator exceeds u32
            let scaled_num = num / 1000;
            let scaled_den = (den / 1000).max(1) as u32;
            Self::new(scaled_num as i64, scaled_den)
        } else {
            Self::new(num as i64, den as u32)
        }
    }
}

impl Sub for TimeRational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let num = (self.numerator as i128) * (rhs.denominator as i128)
            - (rhs.numerator as i128) * (self.denominator as i128);
        let den = (self.denominator as u64) * (rhs.denominator as u64);
        if den > u64::from(u32::MAX) {
            let scaled_num = num / 1000;
            let scaled_den = (den / 1000).max(1) as u32;
            Self::new(scaled_num as i64, scaled_den)
        } else {
            Self::new(num as i64, den as u32)
        }
    }
}

impl fmt::Display for TimeRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}s", self.numerator, self.denominator)
    }
}

/// Rational video frame rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub const FPS_24: Self = Self {
        numerator: 24,
        denominator: 1,
    };
    pub const FPS_25: Self = Self {
        numerator: 25,
        denominator: 1,
    };
    pub const FPS_29_97: Self = Self {
        numerator: 30000,
        denominator: 1001,
    };
    pub const FPS_30: Self = Self {
        numerator: 30,
        denominator: 1,
    };
    pub const FPS_50: Self = Self {
        numerator: 50,
        denominator: 1,
    };
    pub const FPS_59_94: Self = Self {
        numerator: 60000,
        denominator: 1001,
    };
    pub const FPS_60: Self = Self {
        numerator: 60,
        denominator: 1,
    };

    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator: if denominator == 0 { 1 } else { denominator },
        }
    }

    /// Compute nominal frame rate as floating point for display.
    pub fn to_f64(&self) -> f64 {
        if self.denominator == 0 {
            0.0
        } else {
            f64::from(self.numerator) / f64::from(self.denominator)
        }
    }

    /// Rounded integer frames per second for standard calculations.
    pub fn rounded_int(&self) -> i64 {
        self.to_f64().round() as i64
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::FPS_24
    }
}

impl fmt::Display for FrameRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{} fps", self.numerator)
        } else {
            write!(f, "{:.2} fps", self.to_f64())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_rational_simplification() {
        let t = TimeRational::new(100, 200);
        assert_eq!(t.numerator, 1);
        assert_eq!(t.denominator, 2);
    }

    #[test]
    fn test_timecode_formatting() {
        let fps = FrameRate::FPS_24;
        let t = TimeRational::from_frames(24, fps); // exactly 1 second
        assert_eq!(t.to_timecode(fps), "00:00:01:00");

        let t2 = TimeRational::from_frames(1440, fps); // exactly 1 minute
        assert_eq!(t2.to_timecode(fps), "00:01:00:00");
    }

    #[test]
    fn test_rational_ordering_and_math() {
        let a = TimeRational::new(1, 2);
        let b = TimeRational::new(1, 4);
        assert!(a > b);

        let sum = a + b;
        assert_eq!(sum.numerator, 3);
        assert_eq!(sum.denominator, 4);
    }
}
