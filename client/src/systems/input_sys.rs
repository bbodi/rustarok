use crate::components::controller::{CameraComponent, CameraMode, HumanInputComponent, SkillKey};
use crate::components::skills::skills::{SkillTargetType, Skills};
use crate::systems::RenderMatrices;
use crate::systems::SystemVariables;
use crate::ConsoleCommandBuffer;
use nalgebra::Vector4;
use rustarok_common::common::{v2, v3, Local, Mat4, Vec2, Vec3};
use rustarok_common::components::controller::PlayerIntention;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use specs::prelude::*;

pub struct InputConsumerSystem;

impl InputConsumerSystem {
    pub fn run(
        &mut self,
        input: &mut HumanInputComponent,
        camera: &mut CameraComponent,
        console_command_buffer: &mut ConsoleCommandBuffer,
        matrices: &RenderMatrices,
    ) {
        let events: Vec<_> = input.inputs.drain(..).collect();
        input.left_mouse_released = false;
        input.right_mouse_released = false;
        input.right_mouse_pressed = false;
        input.left_mouse_pressed = false;
        input.mouse_wheel = 0;
        input.delta_mouse_x = 0;
        input.delta_mouse_y = 0;
        input.alt_down = false;
        input.ctrl_down = false;
        input.shift_down = false;
        input.text = String::new();
        input.cleanup_released_keys();
        for event in events {
            match event {
                sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                    MouseButton::Left => {
                        input.left_mouse_down = true;
                        input.left_mouse_pressed = true;
                        input.left_mouse_released = false;
                    }
                    MouseButton::Right => {
                        input.right_mouse_down = true;
                        input.right_mouse_pressed = true;
                        input.right_mouse_released = false;
                    }
                    _ => {}
                },
                sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    MouseButton::Left => {
                        input.left_mouse_down = false;
                        input.left_mouse_released = true;
                    }
                    MouseButton::Right => {
                        input.right_mouse_down = false;
                        input.right_mouse_released = true;
                    }
                    _ => {}
                },
                sdl2::event::Event::MouseMotion {
                    timestamp: _,
                    window_id: _,
                    which: _,
                    mousestate: _,
                    x,
                    y,
                    xrel: _,
                    yrel: _,
                } => {
                    input.delta_mouse_x = x - input.last_mouse_x as i32;
                    input.delta_mouse_y = input.last_mouse_y as i32 - y; // reversed since y-coordinates go from bottom to top
                    input.last_mouse_x = x as u16;
                    input.last_mouse_y = y as u16;
                }
                sdl2::event::Event::MouseWheel { y, .. } => {
                    input.mouse_wheel = y;
                }
                sdl2::event::Event::KeyDown {
                    scancode, keymod, ..
                } => {
                    if let Some(scancode) = scancode {
                        if scancode == Scancode::LCtrl || scancode == Scancode::LAlt {
                            // It causes problems on the browser because alt-tabbing
                            // does not releases the alt key
                            // So alt and ctrl keys should be registered only together with other keys
                            continue;
                        }
                        input.key_pressed(scancode);
                        if keymod.contains(sdl2::keyboard::Mod::LALTMOD) {
                            input.alt_down = true;
                        }
                        if keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) {
                            input.ctrl_down = true;
                        }
                        if keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) {
                            input.shift_down = true;
                        }
                    }
                }
                sdl2::event::Event::KeyUp {
                    scancode, keymod, ..
                } => {
                    if let Some(scancode) = scancode {
                        if scancode == Scancode::LCtrl || scancode == Scancode::LAlt {
                            // see above
                            continue;
                        }
                        input.key_released(scancode);
                        if keymod.contains(sdl2::keyboard::Mod::LALTMOD) {
                            input.alt_down = true;
                        }
                        if keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) {
                            input.ctrl_down = true;
                        }
                        if keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) {
                            input.shift_down = true;
                        }
                    }
                }
                sdl2::event::Event::TextInput { text, .. } => {
                    input.text = text;
                }
                _ => {}
            }
        }

        {
            let key = input.key_bindings.iter().find_map(|key_binding| {
                let (keys, script) = key_binding;
                let mut need_alt = false;
                let mut need_ctrl = false;
                let mut need_shift = false;
                let mut key = None;
                let mut all_keys_down = keys.iter().take_while(|it| it.is_some()).all(|it| {
                    let it = it.unwrap();
                    match it {
                        Scancode::LAlt => {
                            need_alt = true;
                            true
                        }
                        Scancode::LCtrl => {
                            need_ctrl = true;
                            true
                        }
                        Scancode::LShift => {
                            need_shift = true;
                            true
                        }
                        _ => {
                            key = Some(it);
                            input.is_key_just_pressed(it)
                        }
                    }
                });
                all_keys_down &= need_alt == input.alt_down;
                all_keys_down &= need_ctrl == input.ctrl_down;
                all_keys_down &= need_shift == input.shift_down;
                if all_keys_down {
                    console_command_buffer.commands.push(script.clone());
                    key
                } else {
                    None
                }
            });
            // in case of a binding activation, we remove the key from being
            // "just pressed", so other areas won't register it as a keypress (e.g. console)
            // (calling key_pressed again on the key will set it as "down" but not as "just-pressed")
            if let Some(key) = key {
                input.key_pressed(key);
                input.text.clear();
            }
        }

        if input.is_key_just_released(Scancode::L) && !input.is_console_open {
            match input.camera_movement_mode {
                CameraMode::Free => {
                    input.camera_movement_mode = CameraMode::FollowChar;
                    camera.reset_y_and_angle(
                        &matrices.projection,
                        matrices.resolution_w,
                        matrices.resolution_h,
                    );
                }
                CameraMode::FollowChar => {
                    input.camera_movement_mode = CameraMode::FreeMoveButFixedAngle
                }
                CameraMode::FreeMoveButFixedAngle => input.camera_movement_mode = CameraMode::Free,
            }
        }

        let mouse_world_pos = InputConsumerSystem::project_screen_pos_to_world_pos(
            input.last_mouse_x,
            input.last_mouse_y,
            &camera.camera.pos(),
            &matrices.projection,
            &camera.view_matrix,
            matrices.resolution_w,
            matrices.resolution_h,
        );
        input.mouse_world_pos = mouse_world_pos;

        if input.is_key_just_released(Scancode::F12) {
            match input.get_skill_for_key(SkillKey::Q) {
                Some(Skills::FireWall) => {
                    input.assign_skill(SkillKey::Q, Skills::Poison);
                    input.assign_skill(SkillKey::W, Skills::FireBomb);
                    input.assign_skill(SkillKey::E, Skills::Cure);
                    input.assign_skill(SkillKey::R, Skills::Lightning);
                    input.assign_skill(SkillKey::Num1, Skills::Sanctuary);
                    input.assign_skill(SkillKey::Num2, Skills::ExoSkeleton);
                    input.assign_skill(SkillKey::Num3, Skills::GazBarricade);
                }
                Some(Skills::Poison) => {
                    input.assign_skill(SkillKey::Q, Skills::WizPyroBlast);
                    input.assign_skill(SkillKey::W, Skills::AssaBladeDash);
                    input.assign_skill(SkillKey::E, Skills::AssaPhasePrism);
                    input.assign_skill(SkillKey::R, Skills::GazXplodiumCharge);
                }
                Some(Skills::WizPyroBlast) => {
                    input.assign_skill(SkillKey::Q, Skills::GazTurret);
                    input.assign_skill(SkillKey::D, Skills::GazDestroyTurret);
                    input.assign_skill(SkillKey::Num1, Skills::GazTurretTarget);
                    input.assign_skill(SkillKey::Num2, Skills::FalconCarry);
                    input.assign_skill(SkillKey::Num3, Skills::FalconAttack);
                }
                Some(Skills::GazTurret) => {
                    input.assign_skill(SkillKey::Q, Skills::FireWall);
                    input.assign_skill(SkillKey::W, Skills::AbsorbShield);
                    input.assign_skill(SkillKey::E, Skills::Heal);
                    input.assign_skill(SkillKey::R, Skills::BrutalTestSkill);
                }
                _ => {}
            }
        }
    }
}

impl InputConsumerSystem {
    pub fn target_selection_or_casting(
        skill: Skills,
        mouse_pos: Vec2,
    ) -> Option<PlayerIntention<Local>> {
        // NoTarget skills have to be casted immediately without selecting target
        if skill.get_definition().get_skill_target_type() == SkillTargetType::NoTarget {
            log::debug!("Skill '{:?}' is no target, so cast it", skill);
            // TODO2
            //Some(PlayerIntention::Casting(skill, false, mouse_pos))
            None
        } else {
            None
        }
    }

    pub fn project_screen_pos_to_world_pos(
        x2d: u16,
        y2d: u16,
        camera_pos: &Vec3,
        projection: &Mat4,
        view: &Mat4,
        resolution_w: u32,
        resolution_h: u32,
    ) -> Vec2 {
        let x = x2d as f32;
        let y = y2d as f32;

        let ray_clip = Vector4::new(
            2.0 * x / resolution_w as f32 - 1.0,
            1.0 - (2.0 * y) / resolution_h as f32,
            -1.0,
            1.0,
        );
        let ray_eye = projection.try_inverse().unwrap() * ray_clip;
        let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
        let ray_world = view.try_inverse().unwrap() * ray_eye;
        let ray_world = v3(ray_world.x, ray_world.y, ray_world.z).normalize();

        let line_location = camera_pos;
        let line_direction: Vec3 = ray_world;
        let plane_normal = v3(0.0, 1.0, 0.0);
        let plane_point = v3(0.0, 0.0, 0.0);
        let t = (plane_normal.dot(&plane_point) - plane_normal.dot(&line_location))
            / plane_normal.dot(&line_direction);
        let world_pos = line_location + (line_direction.scale(t));
        return v2(world_pos.x, world_pos.z);
    }
}
