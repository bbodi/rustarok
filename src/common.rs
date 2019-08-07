use nalgebra::{Matrix4, Point2, Point3, Rotation3, Vector2, Vector3};

#[macro_export]
macro_rules! v2 {
    ($x:expr, $y:expr) => {
        Vector2::<f32>::new($x as f32, $y as f32)
    };
}

#[macro_export]
macro_rules! v3 {
    ($x:expr, $y:expr, $z:expr) => {
        Vector3::<f32>::new($x as f32, $y as f32, $z as f32)
    };
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

pub fn p3_to_p2(input: &Point3<f32>) -> Point2<f32> {
    Point2::new(input.x, input.z)
}

pub fn p3_to_v2(input: &Point3<f32>) -> Vector2<f32> {
    v2!(input.x, input.z)
}

pub fn v2_to_p3(input: &Vector2<f32>) -> Point3<f32> {
    p3!(input.x, 0.0, input.y)
}

pub fn v2_to_v3(input: &Vector2<f32>) -> Vector3<f32> {
    Vector3::new(input.x, 0.0, input.y)
}

pub fn v2_to_p2(input: &Vector2<f32>) -> Point2<f32> {
    Point2::new(input.x, input.y)
}

pub fn rotate_vec2(rad: f32, vec: &Vector2<f32>) -> Vector2<f32> {
    let rot_matrix = Matrix4::<f32>::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad)
        .to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&v2_to_p3(vec));
    return p3_to_v2(&rotated);
}
