use nalgebra::{Matrix3, Matrix4, Point2, Point3, Rotation3, Vector2, Vector3};
use serde::Deserialize;
use std::time::{Duration, Instant};

pub type Mat3 = Matrix3<f32>;
pub type Mat4 = Matrix4<f32>;

pub type Vec2 = Vector2<f32>;
pub type Vec3 = Vector3<f32>;
pub type Vec2i = Vector2<i16>;
pub type Vec2u = Vector2<u16>;

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

#[macro_export]
macro_rules! p2 {
    ($x:expr, $y:expr) => {
        Point2::<f32>::new($x as f32, $y as f32)
    };
}

#[macro_export]
macro_rules! p3 {
    ($x:expr, $y:expr, $z:expr) => {
        Point3::<f32>::new($x as f32, $y as f32, $z as f32)
    };
}

#[inline]
pub fn p3_to_v2(input: &Point3<f32>) -> Vec2 {
    v2(input.x, input.z)
}

#[inline]
pub fn v2_to_p3(input: &Vec2) -> Point3<f32> {
    p3!(input.x, 0.0, input.y)
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

#[derive(Copy, Clone, Debug)]
pub struct DeltaTime(pub f32);

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(from = "f32")]
pub struct ElapsedTime(pub f32);

impl From<f32> for ElapsedTime {
    fn from(value: f32) -> Self {
        ElapsedTime(value)
    }
}

impl PartialEq for ElapsedTime {
    fn eq(&self, other: &Self) -> bool {
        (self.0 * 1000.0) as u32 == (other.0 * 1000.0) as u32
    }
}

impl Eq for ElapsedTime {}

impl ElapsedTime {
    pub fn add_seconds(&self, seconds: f32) -> ElapsedTime {
        ElapsedTime(self.0 + seconds as f32)
    }

    pub fn minus(&self, other: ElapsedTime) -> ElapsedTime {
        ElapsedTime(self.0 - other.0)
    }

    pub fn percentage_between(&self, from: ElapsedTime, to: ElapsedTime) -> f32 {
        let current = self.0 - from.0;
        let range = to.0 - from.0;
        return current / range;
    }

    pub fn add(&self, other: ElapsedTime) -> ElapsedTime {
        ElapsedTime(self.0 + other.0)
    }

    pub fn elapsed_since(&self, other: ElapsedTime) -> ElapsedTime {
        ElapsedTime(self.0 - other.0)
    }

    pub fn div(&self, other: f32) -> f32 {
        self.0 / other
    }

    pub fn run_at_least_until_seconds(&mut self, system_time: ElapsedTime, seconds: f32) {
        self.0 = self.0.max(system_time.0 + seconds);
    }

    pub fn has_already_passed(&self, system_time: ElapsedTime) -> bool {
        self.0 <= system_time.0
    }

    pub fn has_not_passed_yet(&self, other: ElapsedTime) -> bool {
        self.0 > other.0
    }

    pub fn as_f32(&self) -> f32 {
        self.0
    }
}
