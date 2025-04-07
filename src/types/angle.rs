use std::ops;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Angle {
    degrees: f64,
}

impl Angle {
    #[allow(dead_code)]
    pub fn degrees(degrees: f64) -> Self {
        Self { degrees }
    }

    #[allow(dead_code)]
    pub fn radians(radians: f64) -> Self {
        Self {
            degrees: radians.to_degrees(),
        }
    }

    #[allow(dead_code)]
    pub fn as_degrees(&self) -> f64 {
        self.degrees
    }

    #[allow(dead_code)]
    pub fn as_radians(&self) -> f64 {
        self.degrees.to_radians()
    }

    pub fn clamp(mut self, min: Self, max: Self) -> Self {
        self.degrees = self.degrees.clamp(min.degrees, max.degrees);
        self
    }
}

impl From<f64> for Angle {
    fn from(value: f64) -> Self {
        Self { degrees: value }
    }
}

impl AsRef<f64> for Angle {
    fn as_ref(&self) -> &f64 {
        &self.degrees
    }
}

impl AsMut<f64> for Angle {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.degrees
    }
}

impl ops::Add for Angle {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            degrees: self.degrees + rhs.degrees,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use super::*;

    const DEGREES_0: f64 = 0_f64;
    const DEGREES_45: f64 = 45_f64;
    const DEGREES_90: f64 = 90_f64;
    const DEGREES_180: f64 = 180_f64;

    #[test]
    fn test_angle_to_radians() {
        let angle = Angle::degrees(DEGREES_90);
        assert_eq!(angle.as_radians(), FRAC_PI_2);
    }

    #[test]
    fn test_angle_to_degrees() {
        let angle = Angle::radians(FRAC_PI_2);
        assert_eq!(angle.as_degrees(), DEGREES_90);
    }

    #[test]
    fn test_angle_add() {
        let lhs = Angle::degrees(DEGREES_90);
        let rhs = Angle::degrees(DEGREES_90);
        let result = lhs + rhs;
        assert_eq!(result.as_degrees(), DEGREES_180);
    }

    #[test]
    fn test_angle_clamp_max() {
        let angle = Angle::degrees(DEGREES_90);
        let min = Angle::degrees(DEGREES_0);
        let max = Angle::degrees(DEGREES_45);
        let result = angle.clamp(min, max);
        assert_eq!(result.as_degrees(), DEGREES_45);
    }

    #[test]
    fn test_angle_clamp_min() {
        let angle = Angle::degrees(-DEGREES_90);
        let min = Angle::degrees(DEGREES_0);
        let max = Angle::degrees(DEGREES_45);
        let result = angle.clamp(min, max);
        assert_eq!(result.as_degrees(), DEGREES_0);
    }
}
