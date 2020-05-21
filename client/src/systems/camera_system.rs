use crate::components::controller::{
    CameraComponent, CameraMode, HumanInputComponent, LocalPlayerController,
};
use crate::runtime_assets::map::MapRenderData;
use crate::systems::SystemVariables;
use rustarok_common::common::{EngineTime, Local};
use rustarok_common::components::char::{EntityId, LocalCharStateComp};
use sdl2::keyboard::Scancode;
use specs::prelude::*;

// TODO: singleton
pub struct CameraSystem;

impl<'a> System<'a> for CameraSystem {
    type SystemData = (
        ReadStorage<'a, LocalCharStateComp<Local>>,
        ReadExpect<'a, LocalPlayerController>,
        ReadExpect<'a, HumanInputComponent>,
        WriteExpect<'a, CameraComponent>,
        ReadExpect<'a, SystemVariables>,
        ReadExpect<'a, MapRenderData>,
        ReadExpect<'a, EngineTime>,
    );

    fn run(
        &mut self,
        (auth_char_state_storage, local_player, input, mut camera, sys_vars, map_render_data, time): Self::SystemData,
    ) {
        match input.camera_movement_mode {
            CameraMode::Free => {
                if !input.is_console_open {
                    CameraSystem::free_movement(&mut camera, &input);
                }
                if input.left_mouse_down {
                    camera.yaw += input.delta_mouse_x as f32;
                    camera.pitch += input.delta_mouse_y as f32;
                    if camera.pitch > 89.0 {
                        camera.pitch = 89.0;
                    }
                    if camera.pitch < -89.0 {
                        camera.pitch = -89.0;
                    }
                    if camera.yaw > 360.0 {
                        camera.yaw -= 360.0;
                    } else if camera.yaw < 0.0 {
                        camera.yaw += 360.0;
                    }
                    let pitch = camera.pitch;
                    let yaw = camera.yaw;
                    camera.camera.rotate(pitch, yaw);
                }
            }
            CameraMode::FollowChar => {
                if let Some(followed_char_id) = local_player.controller.controlled_entity {
                    if let Some(followed_char) =
                        auth_char_state_storage.get(followed_char_id.into())
                    {
                        if input.mouse_wheel != 0 {
                            camera.camera.move_forward(input.mouse_wheel as f32 * 2.0);
                            camera.camera.update_visible_z_range(
                                &sys_vars.matrices.projection,
                                sys_vars.matrices.resolution_w,
                                sys_vars.matrices.resolution_h,
                            );
                        };
                        let pos = followed_char.pos();
                        camera.camera.set_x(pos.x);
                        let z_range = camera.camera.visible_z_range;
                        camera.camera.set_z(pos.y + z_range);
                    }
                }
            }
            CameraMode::FreeMoveButFixedAngle => {
                if input.mouse_wheel != 0 {
                    camera.camera.move_forward(input.mouse_wheel as f32 * 2.0);
                    camera.camera.update_visible_z_range(
                        &sys_vars.matrices.projection,
                        sys_vars.matrices.resolution_w,
                        sys_vars.matrices.resolution_h,
                    );
                }
                if !input.is_console_open {
                    CameraSystem::axis_aligned_movement(&mut camera, &input);
                }
            }
        }

        if camera.camera.pos().x < 0.0 {
            camera.camera.set_x(0.0);
        } else if camera.camera.pos().x > map_render_data.ground_width as f32 * 2.0 {
            camera
                .camera
                .set_x(map_render_data.ground_width as f32 * 2.0);
        }
        if camera.camera.pos().z > 0.0 {
            camera.camera.set_z(0.0);
        } else if camera.camera.pos().z < -(map_render_data.ground_height as f32 * 2.0) {
            camera
                .camera
                .set_z(-(map_render_data.ground_height as f32 * 2.0));
        }

        camera.view_matrix = camera.camera.create_view_matrix();
        camera.normal_matrix = {
            let inverted = camera.view_matrix.try_inverse().unwrap();
            let m3x3 = inverted.fixed_slice::<nalgebra::base::U3, nalgebra::base::U3>(0, 0);
            m3x3.transpose()
        };
    }
}

impl CameraSystem {
    fn axis_aligned_movement(camera: &mut CameraComponent, input: &HumanInputComponent) {
        let camera_speed = if input.is_key_down(Scancode::LShift) {
            6.0
        } else {
            1.0
        };
        if input.is_key_down(Scancode::Left) {
            camera.camera.move_along_x(-camera_speed);
        } else if input.is_key_down(Scancode::Right) {
            camera.camera.move_along_x(camera_speed);
        }
        if input.is_key_down(Scancode::Up) {
            camera.camera.move_along_z(-camera_speed);
        } else if input.is_key_down(Scancode::Down) {
            camera.camera.move_along_z(camera_speed);
        }
    }

    fn free_movement(camera: &mut CameraComponent, input: &HumanInputComponent) {
        let camera_speed = if input.is_key_down(Scancode::LShift) {
            6.0
        } else {
            1.0
        };
        if input.is_key_down(Scancode::Left) {
            camera.camera.move_side(-camera_speed);
        } else if input.is_key_down(Scancode::Right) {
            camera.camera.move_side(camera_speed);
        }
        if input.is_key_down(Scancode::Up) {
            camera.camera.move_forward(-camera_speed);
        } else if input.is_key_down(Scancode::Down) {
            camera.camera.move_forward(camera_speed);
        }
    }
}
