use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CameraComponent, CameraMode, ControllerComponent, HumanInputComponent,
};
use crate::runtime_assets::map::MapRenderData;
use crate::systems::SystemVariables;
use sdl2::keyboard::Scancode;
use specs::prelude::*;

// TODO: singleton
pub struct CameraSystem;

impl<'a> System<'a> for CameraSystem {
    type SystemData = (
        ReadStorage<'a, CharacterStateComponent>,
        ReadStorage<'a, HumanInputComponent>,
        ReadStorage<'a, ControllerComponent>,
        WriteStorage<'a, CameraComponent>,
        ReadExpect<'a, SystemVariables>,
        ReadExpect<'a, MapRenderData>,
    );

    fn run(
        &mut self,
        (
            char_state_storage,
            input_storage,
            controller_storage,
            mut camera_storage,
            sys_vars,
            map_render_data,
        ): Self::SystemData,
    ) {
        for (input, camera) in (&input_storage, &mut camera_storage).join() {
            match input.camera_movement_mode {
                CameraMode::Free => {
                    if !input.is_console_open {
                        CameraSystem::free_movement(camera, input);
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
                        camera.camera.rotate(camera.pitch, camera.yaw);
                    }
                }
                CameraMode::FollowChar => {
                    if let Some(followed_controller) = camera.followed_controller {
                        if let Some(followed_char) = controller_storage
                            .get(followed_controller.0)
                            .map(|it| it.controlled_entity)
                        {
                            if input.mouse_wheel != 0 {
                                camera.camera.move_forward(input.mouse_wheel as f32 * 2.0);
                                camera.camera.update_visible_z_range(
                                    &sys_vars.matrices.projection,
                                    sys_vars.resolution_w,
                                    sys_vars.resolution_h,
                                );
                            };
                            if let Some(char_state) = char_state_storage.get(followed_char.0) {
                                let pos = char_state.pos();
                                camera.camera.set_x(pos.x);
                                let z_range = camera.camera.visible_z_range;
                                camera.camera.set_z(pos.y + z_range);
                            }
                        }
                    }
                }
                CameraMode::FreeMoveButFixedAngle => {
                    if input.mouse_wheel != 0 {
                        camera.camera.move_forward(input.mouse_wheel as f32 * 2.0);
                        camera.camera.update_visible_z_range(
                            &sys_vars.matrices.projection,
                            sys_vars.resolution_w,
                            sys_vars.resolution_h,
                        );
                    }
                    if !input.is_console_open {
                        CameraSystem::axis_aligned_movement(camera, input);
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
