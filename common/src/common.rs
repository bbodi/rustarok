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

#[derive(Clone)]
pub struct EngineTime {
    pub render_frame: u64,
    pub simulation_frame: u64,
    skip_next_simulation: bool,
    run_simulation_in_this_frame: bool,
    pub end_of_last_frame: Instant,
    pub time: ElapsedTime,
    /// seconds the previous frame required
    pub dt: Duration,
    // TODO: #[cfg(test)]
    pub fix_dt_for_test: Duration,
    last_simulation_at: Instant,
    time_between_simulations: Duration,
}

impl EngineTime {
    pub fn new(simulation_freq: usize) -> EngineTime {
        EngineTime {
            fix_dt_for_test: Duration::from_millis(1),
            simulation_frame: 1,
            render_frame: 1,
            skip_next_simulation: false,
            run_simulation_in_this_frame: true,
            end_of_last_frame: Instant::now(),
            last_simulation_at: Instant::now(),

            time: ElapsedTime(0.0),
            dt: Duration::from_millis(1),
            time_between_simulations: Duration::from_millis((1000 / simulation_freq) as u64),
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(fix_dt_for_test: Duration) -> EngineTime {
        EngineTime {
            fix_dt_for_test,
            last_simulation_at: Instant::now(),
            simulation_frame: 1,
            render_frame: 1,
            run_simulation_in_this_frame: true,
            end_of_last_frame: Instant::now(),
            time: ElapsedTime(0.0),
            skip_next_simulation: false,
            dt: Duration::from_millis(1),
            time_between_simulations: Duration::from_millis(30 as u64),
        }
    }

    pub fn can_simulation_run(&self) -> bool {
        self.run_simulation_in_this_frame
    }

    pub fn get_time_between_simulations(&self) -> Duration {
        self.time_between_simulations
    }

    pub fn adjust_simulation_freq(&mut self, simulation_duration_adjuster: i64) {
        if simulation_duration_adjuster > 0 {
            self.time_between_simulations +=
                Duration::from_millis(simulation_duration_adjuster as u64);
        } else if simulation_duration_adjuster < 0 {
            let tmp = simulation_duration_adjuster.abs();
            if self.time_between_simulations.as_millis() as i64 > tmp {
                self.time_between_simulations -= Duration::from_millis(tmp as u64);
            }
        }
    }

    pub fn render_frame_end(&mut self, dt: Duration, now: Instant) {
        let dt = if cfg!(test) {
            self.fix_dt_for_test
        } else {
            self.end_of_last_frame = now;
            dt
        };
        self.render_frame += 1;
        self.dt = dt;
        self.time.0 += dt.as_millis() as f32 / 1000.0;

        if self.run_simulation_in_this_frame {
            log::debug!(
                "simulation {} -> {}",
                self.simulation_frame,
                self.simulation_frame + 1
            );
            self.simulation_frame += 1;
        }

        self.run_simulation_in_this_frame =
            self.last_simulation_at.elapsed() >= self.time_between_simulations;
        if self.run_simulation_in_this_frame {
            if self.skip_next_simulation {
                self.run_simulation_in_this_frame = false;
                self.skip_next_simulation = false;
            } else {
                self.last_simulation_at = now;
            }
        }
    }

    pub fn update_timers_for_prediction(&mut self) {
        self.simulation_frame += 1;
    }

    pub fn force_simulation(&mut self) {
        self.run_simulation_in_this_frame = true;
    }

    pub fn skip_next_simulation(&mut self) {
        self.skip_next_simulation = true;
    }

    #[inline]
    pub fn now(&self) -> ElapsedTime {
        self.time
    }

    pub fn dt(&self) -> f32 {
        self.dt.as_millis() as f32 / 1000.0
    }

    pub fn reverted(&self, by_tick: u64) -> EngineTime {
        EngineTime {
            simulation_frame: self.simulation_frame - by_tick,
            run_simulation_in_this_frame: true,
            ..*self
        }
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
        ElapsedTime(self.0 + seconds)
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
