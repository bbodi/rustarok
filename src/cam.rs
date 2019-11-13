use crate::common::{v3, Mat4, Vec2, Vec3};
use crate::systems::input_sys::InputConsumerSystem;
use nalgebra::Point3;

#[derive(Clone)]
pub struct Camera {
    pos: Vec3,
    front: Vec3,
    up: Vec3,
    right: Vec3,
    pub visible_z_range: f32,
    pub top_z_world_coord_offset: f32,
}

#[allow(dead_code)]
impl Camera {
    pub fn new(pos: Vec3) -> Camera {
        let front = v3(0.0, 0.0, -1.0);
        let up = Vec3::y();
        Camera {
            pos,
            front,
            up,
            right: front.cross(&up).normalize(),
            visible_z_range: 0.0,
            top_z_world_coord_offset: 0.0,
        }
    }

    pub fn is_visible(&self, pos: Vec2) -> bool {
        return (pos.x >= self.pos.x - 40.0 && pos.x <= self.pos.x + 40.0)
            && (pos.y >= self.pos.z - self.top_z_world_coord_offset && pos.y <= self.pos.z + 5.0);
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    pub fn set_x(&mut self, x: f32) {
        self.pos.x = x;
    }

    pub fn set_y(&mut self, y: f32) {
        self.pos.y = y;
    }

    pub fn set_z(&mut self, z: f32) {
        self.pos.z = z;
    }

    pub fn update_visible_z_range(
        &mut self,
        projection: &Mat4,
        resolution_w: u32,
        resolution_h: u32,
    ) {
        let view = self.create_view_matrix();
        let center = InputConsumerSystem::project_screen_pos_to_world_pos(
            0,
            (resolution_h / 2) as u16,
            &self.pos(),
            projection,
            &view,
            resolution_w,
            resolution_h,
        );
        self.visible_z_range = (self.pos.z - center.y).abs();
        self.top_z_world_coord_offset = self.pos.z
            - InputConsumerSystem::project_screen_pos_to_world_pos(
                0,
                0,
                &self.pos(),
                projection,
                &view,
                resolution_w,
                resolution_h,
            )
            .y;
    }

    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        self.front = v3(
            pitch.to_radians().cos() * yaw.to_radians().cos(),
            pitch.to_radians().sin(),
            pitch.to_radians().cos() * yaw.to_radians().sin(),
        )
        .normalize();
        self.right = self.front.cross(&Vec3::y()).normalize();
        self.up = self.right.cross(&self.front).normalize();
    }

    pub fn move_forward(&mut self, speed: f32) {
        self.pos += speed * self.front;
    }

    pub fn move_side(&mut self, speed: f32) {
        self.pos += self.front.cross(&self.up).normalize() * speed;
    }

    pub fn move_along_z(&mut self, speed: f32) {
        self.pos.z += speed;
    }

    pub fn move_along_x(&mut self, speed: f32) {
        self.pos.x += speed;
    }

    pub fn create_view_matrix(&self) -> Mat4 {
        let forward = self.pos + self.front;
        Mat4::look_at_rh(
            &Point3::new(self.pos.x, self.pos.y, self.pos.z),
            &Point3::new(forward.x, forward.y, forward.z),
            &self.up,
        )
    }

    pub fn look_at(&mut self, p: Vec3) {
        self.front = (p - self.pos).normalize();
        self.right = self.front.cross(&Vec3::y()).normalize();
        self.up = self.right.cross(&self.front).normalize();
    }
}
