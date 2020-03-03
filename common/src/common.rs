use crate::char_attr::CharAttributeModifier;
use nalgebra::{Matrix3, Matrix4, Point2, Point3, Rotation3, Vector2, Vector3};
use serde::Deserialize;
use serde::Serialize;

use std::time::{Duration, Instant};

pub type Mat3 = Matrix3<f32>;
pub type Mat4 = Matrix4<f32>;

pub type Vec2 = Vector2<f32>;
pub type Vec3 = Vector3<f32>;
pub type Vec2i = Vector2<i16>;
pub type Vec2u = Vector2<u16>;

pub const ALLOWED_F32_DIFF: f32 = 0.01;
pub fn float_cmp(a: f32, b: f32) -> bool {
    (a - b).abs() < ALLOWED_F32_DIFF
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
pub struct SimulationTick(u64);

impl SimulationTick {
    pub fn new() -> SimulationTick {
        SimulationTick(0)
    }
    pub fn inc(&mut self) {
        self.0 += 1;
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn prev(&self) -> SimulationTick {
        SimulationTick(self.0 - 1)
    }

    pub fn revert(&mut self, by_tick: usize) {
        self.0 -= by_tick as u64;
    }
}

// TODO: does this struct make any sense?
#[derive(Clone)]
pub struct EngineTime {
    pub time: LocalTime,
    /// seconds the previous frame required
    // TODO: #[cfg(test)]
    pub fix_dt_for_test: Duration,
}

impl EngineTime {
    pub fn new(time: u32) -> EngineTime {
        EngineTime {
            fix_dt_for_test: Duration::from_millis(1),
            time: LocalTime::from(time),
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(fix_dt_for_test: Duration) -> EngineTime {
        EngineTime {
            fix_dt_for_test,
            time: LocalTime::from(0.0),
        }
    }

    pub fn tick(&mut self, dt: Duration) {
        let dt = if cfg!(test) { self.fix_dt_for_test } else { dt };
        self.time.0 += dt.as_millis() as u32;
    }

    pub fn reverted(
        &self,
        repredict_this_many_frames: usize,
        one_frame_dt: Duration,
    ) -> EngineTime {
        EngineTime::new(
            self.time.0 - (one_frame_dt.as_millis() as u32 * repredict_this_many_frames as u32),
        )
    }

    #[inline]
    pub fn now(&self) -> LocalTime {
        self.time
    }
}

pub fn measure_time<T, F: FnOnce() -> T>(f: F) -> (Duration, T) {
    let start = Instant::now();
    let r = f();
    (start.elapsed(), r)
}

#[inline]
pub fn v2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

#[inline]
pub fn v2u(x: u16, y: u16) -> Vec2u {
    Vec2u::new(x, y)
}

#[inline]
pub fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[inline]
pub fn p3(x: f32, y: f32, z: f32) -> Point3<f32> {
    Point3::<f32>::new(x, y, z)
}
#[macro_export]
macro_rules! p2 {
    ($x:expr, $y:expr) => {
        Point2::<f32>::new($x as f32, $y as f32)
    };
}

#[inline]
pub fn p3_to_v2(input: &Point3<f32>) -> Vec2 {
    v2(input.x, input.z)
}

#[inline]
pub fn v2_to_p3(input: &Vec2) -> Point3<f32> {
    p3(input.x, 0.0, input.y)
}

#[inline]
pub fn v2_to_v3(input: &Vec2) -> Vector3<f32> {
    Vector3::new(input.x, 0.0, input.y)
}

#[inline]
pub fn v3_to_v2(input: &Vector3<f32>) -> Vec2 {
    v2(input.x, input.z)
}

#[inline]
pub fn v2_to_p2(input: &Vec2) -> Point2<f32> {
    Point2::new(input.x, input.y)
}

pub fn rotate_vec2(rad: f32, vec: &Vec2) -> Vec2 {
    let rot_matrix = Mat4::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad)
        .to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&v2_to_p3(vec));
    return p3_to_v2(&rotated);
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct ServerTime(pub u32);

impl ServerTime {
    pub fn to_local_time(&self, now: LocalTime, server_to_local_time_diff: i64) -> LocalTime {
        let local_time = (self.0 as i64 + server_to_local_time_diff).max(0);
        #[cfg(debug_assertions)]
        {
            if local_time > std::u32::MAX as i64 {
                panic!(format!(
                    "time from server: {:?}, server_to_local_time_diff: {:?}, local_now: {:?}",
                    self, server_to_local_time_diff, now
                ));
            }
        }
        return LocalTime::from(local_time as u32);
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(from = "f32")]
pub struct LocalTime(u32);

impl From<f32> for LocalTime {
    fn from(value: f32) -> Self {
        LocalTime((value * 1000f32) as u32)
    }
}

impl From<u32> for LocalTime {
    fn from(value: u32) -> Self {
        LocalTime(value)
    }
}

impl LocalTime {
    pub fn add_millis(&self, millis: u32) -> LocalTime {
        LocalTime(self.0 + millis)
    }

    pub fn add_seconds(&self, seconds: f32) -> LocalTime {
        LocalTime(self.0 + (seconds * 1000f32) as u32)
    }

    pub fn minus(&self, other: LocalTime) -> LocalTime {
        LocalTime(self.0 - other.0)
    }

    pub fn percentage_between(&self, from: LocalTime, to: LocalTime) -> f32 {
        let current = self.0 - from.0;
        let range = to.0 - from.0;
        return current as f32 / range as f32;
    }

    pub fn add(&self, other: LocalTime) -> LocalTime {
        LocalTime(self.0 + other.0)
    }

    pub fn sub(&self, other: LocalTime) -> LocalTime {
        LocalTime(self.0 - other.0)
    }

    pub fn elapsed_since(&self, other: LocalTime) -> LocalTime {
        LocalTime(self.0 - other.0)
    }

    pub fn div(&self, other: u32) -> u32 {
        self.0 / other
    }

    pub fn run_at_least_until(&mut self, system_time: LocalTime, millis: u32) {
        self.0 = self.0.max(system_time.0 + millis);
    }

    pub fn has_already_passed(&self, system_time: LocalTime) -> bool {
        self.0 <= system_time.0
    }

    pub fn has_not_passed_yet(&self, other: LocalTime) -> bool {
        self.0 > other.0
    }

    pub fn as_seconds_f32(&self) -> f32 {
        self.0 as f32 / 1000f32
    }

    pub fn as_millis(&self) -> u32 {
        self.0
    }
}

// able to represent numbers in 0.1% discrete steps
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(from = "i32", into = "i32")]
pub struct Percentage {
    value: i32,
}

impl From<i32> for Percentage {
    fn from(value: i32) -> Self {
        percentage(value)
    }
}

impl Into<i32> for Percentage {
    fn into(self) -> i32 {
        self.value / Percentage::PERCENTAGE_FACTOR
    }
}

pub const fn percentage(value: i32) -> Percentage {
    Percentage {
        value: value * Percentage::PERCENTAGE_FACTOR,
    }
}

impl Percentage {
    const PERCENTAGE_FACTOR: i32 = 1000;

    pub fn is_not_zero(&self) -> bool {
        self.value != 0
    }

    pub fn as_i16(&self) -> i16 {
        (self.value / Percentage::PERCENTAGE_FACTOR) as i16
    }

    pub fn limit(&mut self, min: Percentage, max: Percentage) {
        self.value = self.value.min(max.value).max(min.value);
    }

    pub fn apply(&mut self, modifier: &CharAttributeModifier) {
        match modifier {
            CharAttributeModifier::AddPercentage(p) => {
                self.value += p.value;
            }
            CharAttributeModifier::AddValue(_v) => panic!(
                "{:?} += {:?}, you cannot add value to a percentage",
                self, modifier
            ),
            CharAttributeModifier::IncreaseByPercentage(p) => {
                self.value = self.increase_by(*p).value;
            }
        }
    }

    pub fn as_f32(&self) -> f32 {
        (self.value as f32 / Percentage::PERCENTAGE_FACTOR as f32) / 100.0
    }

    pub fn increase_by(&self, p: Percentage) -> Percentage {
        let change = self.value / 100 * p.value;
        Percentage {
            value: self.value + change / Percentage::PERCENTAGE_FACTOR,
        }
    }

    pub fn add_me_to(&self, num: i32) -> i32 {
        let f = Percentage::PERCENTAGE_FACTOR as i64;
        let change = (num as i64) * f / 100 * (self.value as i64) / f / f;
        return num + (change as i32);
    }

    pub fn of(&self, num: i32) -> i32 {
        let f = Percentage::PERCENTAGE_FACTOR as i64;
        let change = (num as i64) * f / 100 * (self.value as i64) / f / f;
        return change as i32;
    }

    pub fn subtract_me_from(&self, num: i32) -> i32 {
        let f = Percentage::PERCENTAGE_FACTOR as i64;
        let change = (num as i64) * f / 100 * (self.value as i64) / f / f;
        return num - (change as i32);
    }

    #[allow(dead_code)]
    pub fn div(&self, other: i32) -> Percentage {
        Percentage {
            value: self.value / other,
        }
    }

    pub fn subtract(&self, other: Percentage) -> Percentage {
        Percentage {
            value: self.value - other.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentages() {
        assert_eq!(percentage(70).increase_by(percentage(10)).as_i16(), 77);
        assert_eq!(percentage(70).increase_by(percentage(0)).as_i16(), 70);
        assert_eq!(percentage(70).increase_by(percentage(-10)).as_i16(), 63);
        assert_eq!(percentage(100).increase_by(percentage(200)).as_i16(), 300);
        assert_eq!(percentage(10).add_me_to(200), 220);
        assert_eq!(percentage(70).add_me_to(600), 1020);
        assert_eq!(percentage(70).div(10).add_me_to(600), 642);
        assert_eq!(percentage(-10).add_me_to(200), 180);
        assert_eq!(percentage(50).add_me_to(76), 114);
        assert_eq!(percentage(50).add_me_to(10_000), 15_000);
        assert_eq!(percentage(10).of(200), 20);
        assert_eq!(percentage(70).of(600), 420);
        assert_eq!(percentage(70).div(10).of(600), 42);
        assert_eq!(percentage(50).of(76), 38);
        assert_eq!(percentage(50).of(10_000), 5_000);
        assert_eq!(percentage(10).subtract_me_from(200), 180);
        assert_eq!(percentage(40).subtract_me_from(10_000), 6_000);
        assert_eq!(percentage(70).subtract_me_from(600), 180);
        assert_eq!(percentage(50).subtract_me_from(76), 38);
        assert_eq!(percentage(100).as_f32(), 1.0);
        assert_eq!(percentage(50).as_f32(), 0.5);
        assert_eq!(percentage(5).as_f32(), 0.05);
        assert_eq!(percentage(5).div(10).as_f32(), 0.005);
        assert_eq!(percentage(-5).div(10).as_f32(), -0.005);
    }
}
