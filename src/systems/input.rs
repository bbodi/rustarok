use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CastMode, ControllerComponent, PlayerIntention, SkillKey, WorldCoords,
};
use crate::components::skills::skill::{SkillTargetType, Skills};
use crate::components::BrowserClient;
use crate::systems::SystemVariables;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use nalgebra::{Matrix4, Point2, Point3, Vector2, Vector3, Vector4};
use sdl2::keyboard::Scancode;
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use specs::prelude::*;
use std::io::ErrorKind;
use std::slice::Iter;
use std::time::SystemTime;
use strum::IntoEnumIterator;
use websocket::{OwnedMessage, WebSocketError};

pub struct BrowserInputProducerSystem;

const PACKET_MOUSE_MOVE: i32 = 1;
const PACKET_MOUSE_DOWN: i32 = 2;
const PACKET_MOUSE_UP: i32 = 3;
const PACKET_KEY_DOWN: i32 = 4;
const PACKET_KEY_UP: i32 = 5;
const PACKET_MOUSE_WHEEL: i32 = 6;

fn read_u16(iter: &mut Iter<u8>) -> u16 {
    let upper_byte = iter.next().unwrap();
    let lower_byte = iter.next().unwrap();
    return ((*upper_byte as u16) << 8) | *lower_byte as u16;
}

fn read_i16(iter: &mut Iter<u8>) -> i16 {
    let upper_byte = iter.next().unwrap();
    let lower_byte = iter.next().unwrap();
    return ((*upper_byte as i16) << 8) | *lower_byte as i16;
}

impl<'a> specs::System<'a> for BrowserInputProducerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (entities, mut input_storage, mut browser_client_storage, _updater): Self::SystemData,
    ) {
        for (entity_id, client, input_producer) in
            (&entities, &mut browser_client_storage, &mut input_storage).join()
        {
            let sh = client.websocket.lock().unwrap().recv_message();
            if let Ok(msg) = sh {
                match msg {
                    OwnedMessage::Pong(buf) => {
                        let ping_time = u128::from_le_bytes([
                            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
                            buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
                        ]);
                        let now_ms = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        client.ping = (now_ms - ping_time) as u16;
                    }
                    OwnedMessage::Binary(buf) => {
                        let mut iter = buf.iter();
                        while let Some(header) = iter.next() {
                            let header = *header as i32;
                            match header & 0b1111 {
                                PACKET_MOUSE_MOVE => {
                                    let mouse_x: u16 = read_u16(&mut iter);
                                    let mouse_y: u16 = read_u16(&mut iter);
                                    log::trace!(
                                        "Message arrived: MouseMove({}, {})",
                                        mouse_x,
                                        mouse_y
                                    );
                                    let mousestate = {
                                        unsafe {
                                            std::mem::transmute((0 as u32, 0 as i32, 0 as i32))
                                        }
                                    };
                                    input_producer.inputs.push(sdl2::event::Event::MouseMotion {
                                        timestamp: 0,
                                        window_id: 0,
                                        which: 0,
                                        mousestate,
                                        x: mouse_x as i32,
                                        y: mouse_y as i32,
                                        xrel: 0,
                                        yrel: 0,
                                    });
                                }
                                PACKET_MOUSE_DOWN => {
                                    let mouse_btn = match (header >> 4) & 0b11 {
                                        0 => sdl2::mouse::MouseButton::Left,
                                        1 => sdl2::mouse::MouseButton::Middle,
                                        _ => sdl2::mouse::MouseButton::Right,
                                    };
                                    log::trace!("Message arrived: MouseDown: {:?}", mouse_btn);
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseButtonDown {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        },
                                    );
                                }
                                PACKET_MOUSE_UP => {
                                    let mouse_btn = match (header >> 4) & 0b11 {
                                        0 => sdl2::mouse::MouseButton::Left,
                                        1 => sdl2::mouse::MouseButton::Middle,
                                        _ => sdl2::mouse::MouseButton::Right,
                                    };
                                    log::trace!("Message arrived: MouseUp: {:?}", mouse_btn);
                                    input_producer
                                        .inputs
                                        .push(sdl2::event::Event::MouseButtonUp {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        });
                                }
                                PACKET_KEY_DOWN => {
                                    let scancode = *iter.next().unwrap();
                                    let input_char: u16 = read_u16(&mut iter);
                                    log::trace!(
                                        "Message arrived: KeyDown({}, {})",
                                        scancode,
                                        input_char
                                    );
                                    input_producer.inputs.push(sdl2::event::Event::KeyDown {
                                        timestamp: 0,
                                        window_id: 0,
                                        keycode: None,
                                        scancode: Scancode::from_i32(scancode as i32),
                                        keymod: sdl2::keyboard::Mod::NOMOD,
                                        repeat: false,
                                    });
                                    if let Some(ch) = std::char::from_u32(input_char as u32) {
                                        input_producer.inputs.push(sdl2::event::Event::TextInput {
                                            timestamp: 0,
                                            window_id: 0,
                                            text: ch.to_string(),
                                        });
                                    }
                                }
                                PACKET_KEY_UP => {
                                    let scancode = *iter.next().unwrap();
                                    log::trace!("Message arrived: KeyUp({})", scancode);
                                    input_producer.inputs.push(sdl2::event::Event::KeyUp {
                                        timestamp: 0,
                                        window_id: 0,
                                        keycode: None,
                                        scancode: Scancode::from_i32(scancode as i32),
                                        keymod: sdl2::keyboard::Mod::NOMOD,
                                        repeat: false,
                                    });
                                }
                                PACKET_MOUSE_WHEEL => {
                                    let delta_y: i32 = read_i16(&mut iter) as i32;
                                    log::trace!("Message arrived: MouseWheel({})", delta_y);
                                    input_producer.inputs.push(sdl2::event::Event::MouseWheel {
                                        which: 0,
                                        x: 0,
                                        y: delta_y as i32,
                                        direction: MouseWheelDirection::Normal,
                                        timestamp: 0,
                                        window_id: 0,
                                    });
                                }
                                _ => {
                                    log::warn!("Unknown header: {}", header);
                                    entities.delete(input_producer.char_entity_id).unwrap();
                                    entities.delete(entity_id).unwrap();
                                }
                            };
                        }
                    }
                    _ => {
                        log::warn!("Unknown msg: {:?}", msg);
                        entities.delete(input_producer.char_entity_id).unwrap();
                        entities.delete(entity_id).unwrap();
                    }
                }
            } else if let Err(WebSocketError::IoError(e)) = sh {
                if e.kind() == ErrorKind::ConnectionAborted {
                    // 10053, ConnectionAborted
                    log::info!("Client '{:?}' has disconnected", entity_id);
                    entities.delete(input_producer.char_entity_id).unwrap();
                    entities.delete(entity_id).unwrap();
                }
            }
        }
    }
}

pub struct InputConsumerSystem;

impl<'a> specs::System<'a> for InputConsumerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (_entities, char_state_storage, mut controller_storage, system_vars): Self::SystemData,
    ) {
        for controller in (&mut controller_storage).join() {
            // for autocompletion...
            let controller: &mut ControllerComponent = controller;

            let camera_speed = if controller.is_key_down(Scancode::LShift) {
                6.0
            } else {
                1.0
            };
            let events: Vec<_> = controller.inputs.drain(..).collect();
            controller.left_mouse_released = false;
            controller.right_mouse_released = false;
            controller.right_mouse_pressed = false;
            controller.left_mouse_pressed = false;
            controller.cleanup_released_keys();
            for event in events {
                match event {
                    sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                        MouseButton::Left => {
                            controller.left_mouse_down = true;
                            controller.left_mouse_pressed = true;
                            controller.left_mouse_released = false;
                        }
                        MouseButton::Right => {
                            controller.right_mouse_down = true;
                            controller.right_mouse_pressed = true;
                            controller.right_mouse_released = false;
                        }
                        _ => {}
                    },
                    sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                        MouseButton::Left => {
                            controller.left_mouse_down = false;
                            controller.left_mouse_released = true;
                        }
                        MouseButton::Right => {
                            controller.right_mouse_down = false;
                            controller.right_mouse_released = true;
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
                        // SDL generates only one event when the mouse touches the edge of the screen,
                        // so I put this pseudo key into the controller in that case, which will
                        // indicate screen movement
                        //                        if x == 0 {
                        //                            controller.key_pressed(Scancode::Left);
                        //                            controller.camera.move_along_x(-camera_speed);
                        //                        } else if x == (VIDEO_WIDTH as i32) - 1 {
                        //                            controller.key_pressed(Scancode::Right);
                        //                            controller.camera.move_along_x(camera_speed);
                        //                        } else {
                        //                            controller.key_released(Scancode::Left);
                        //                            controller.key_released(Scancode::Right);
                        //                        }
                        //                        if y == 0 {
                        //                            controller.key_pressed(Scancode::Up);
                        //                            controller.camera.move_along_z(-camera_speed);
                        //                        } else if y == (VIDEO_HEIGHT as i32) - 1 {
                        //                            controller.key_pressed(Scancode::Down);
                        //                            controller.camera.move_along_z(camera_speed);
                        //                        } else {
                        //                            controller.key_released(Scancode::Up);
                        //                            controller.key_released(Scancode::Down);
                        //                        }
                        // free look
                        //                        if controller.mouse_down {
                        //                            let x_offset = x - controller.last_mouse_x as i32;
                        //                            let y_offset = controller.last_mouse_y as i32 - y; // reversed since y-coordinates go from bottom to top
                        //                            controller.yaw += x_offset as f32;
                        //                            controller.pitch += y_offset as f32;
                        //                            if controller.pitch > 89.0 {
                        //                                controller.pitch = 89.0;
                        //                            }
                        //                            if controller.pitch < -89.0 {
                        //                                controller.pitch = -89.0;
                        //                            }
                        //                            if controller.yaw > 360.0 {
                        //                                controller.yaw -= 360.0;
                        //                            } else if controller.yaw < 0.0 {
                        //                                controller.yaw += 360.0;
                        //                            }
                        //                            controller.camera.rotate(controller.pitch, controller.yaw);
                        //                        }
                        controller.last_mouse_x = x as u16;
                        controller.last_mouse_y = y as u16;
                    }
                    sdl2::event::Event::MouseWheel { y, .. } => {
                        controller.camera.move_forward(y as f32 * 2.0);
                        controller
                            .camera
                            .update_visible_z_range(&system_vars.matrices.projection);
                    }
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        if let Some(scancode) = scancode {
                            controller.key_pressed(scancode);
                        }
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        if let Some(scancode) = scancode {
                            controller.key_released(scancode);
                        }
                    }
                    _ => {}
                }
            }
            if controller.is_key_down(Scancode::Left) {
                controller.camera.move_along_x(-camera_speed);
            } else if controller.is_key_down(Scancode::Right) {
                controller.camera.move_along_x(camera_speed);
            }
            if controller.is_key_down(Scancode::Up) {
                controller.camera.move_along_z(-camera_speed);
            } else if controller.is_key_down(Scancode::Down) {
                controller.camera.move_along_z(camera_speed);
            }
            if controller.camera.pos().x < 0.0 {
                controller.camera.set_x(0.0);
            } else if controller.camera.pos().x > system_vars.map_render_data.gnd.width as f32 * 2.0
            {
                controller
                    .camera
                    .set_x(system_vars.map_render_data.gnd.width as f32 * 2.0);
            }
            if controller.camera.pos().z > 0.0 {
                controller.camera.set_z(0.0);
            } else if controller.camera.pos().z
                < -(system_vars.map_render_data.gnd.height as f32 * 2.0)
            {
                controller
                    .camera
                    .set_z(-(system_vars.map_render_data.gnd.height as f32 * 2.0));
            }
            if controller.is_key_just_released(Scancode::L) {
                controller.camera_follows_char = !controller.camera_follows_char;
            }
            if controller.camera_follows_char {
                let char_state = char_state_storage.get(controller.char_entity_id).unwrap();
                let pos = char_state.pos();
                controller.camera.set_x(pos.x);
                let z_range = controller.camera.visible_z_range;
                controller.camera.set_z(pos.y + z_range);
            }
            controller.view_matrix = controller.camera.create_view_matrix();
            controller.normal_matrix = {
                let inverted = controller.view_matrix.try_inverse().unwrap();
                let m3x3 = inverted.fixed_slice::<nalgebra::base::U3, nalgebra::base::U3>(0, 0);
                m3x3.transpose()
            };
            // setup next action based on input
            // TODO: optimize
            let just_pressed_skill_key = SkillKey::iter()
                .filter(|key| controller.is_key_just_pressed(key.scancode()))
                .take(1)
                .collect::<Vec<SkillKey>>()
                .pop();
            let just_released_skill_key = SkillKey::iter()
                .filter(|key| controller.is_key_just_released(key.scancode()))
                .take(1)
                .collect::<Vec<SkillKey>>()
                .pop();

            if controller.next_action.is_some() {
                // here 'next_action' is the action from the prev frame
                controller.last_action = std::mem::replace(&mut controller.next_action, None);
            }
            let alt_down = controller.is_key_down(Scancode::LAlt);
            controller.next_action = if let Some((casting_skill_key, skill)) =
                controller.is_selecting_target()
            {
                match controller.cast_mode {
                    CastMode::Normal => {
                        if controller.left_mouse_released {
                            log::debug!("Player wants to cast {:?}", skill);
                            Some(PlayerIntention::Casting(skill, false))
                        } else if controller.right_mouse_pressed {
                            Some(PlayerIntention::CancelCastingSelectTarget)
                        } else if let Some((skill_key, skill)) =
                            just_pressed_skill_key.and_then(|skill_key| {
                                controller
                                    .get_skill_for_key(skill_key)
                                    .map(|skill| (skill_key, skill))
                            })
                        {
                            log::debug!("Player select target for casting {:?}", skill);
                            Some(InputConsumerSystem::target_selection_or_casting(
                                skill_key, skill,
                            ))
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyRelease => {
                        if controller.is_key_just_released(casting_skill_key.scancode()) {
                            log::debug!("Player wants to cast {:?}", skill);
                            Some(
                                PlayerIntention::Casting(
                                    controller.get_skill_for_key(casting_skill_key)
                                        .expect("'is_casting_selection' must be Some only if the casting skill is valid! "),
                                    false,
                                )
                            )
                        } else if controller.right_mouse_pressed {
                            Some(PlayerIntention::CancelCastingSelectTarget)
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyPress => {
                        // not possible to get into this state while OnKeyPress is active
                        None
                    }
                }
            } else if let Some((skill_key, skill)) = just_pressed_skill_key.and_then(|skill_key| {
                controller
                    .get_skill_for_key(skill_key)
                    .map(|skill| (skill_key, skill))
            }) {
                match controller.cast_mode {
                    CastMode::Normal | CastMode::OnKeyRelease => {
                        if !alt_down {
                            log::debug!("Player select target for casting {:?}", skill);
                            Some(InputConsumerSystem::target_selection_or_casting(
                                skill_key, skill,
                            ))
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyPress => {
                        log::debug!("Player wants to cast {:?}, alt={:?}", skill, alt_down);
                        Some(PlayerIntention::Casting(skill, alt_down))
                    }
                }
            } else if let Some((_skill_key, skill)) =
                just_released_skill_key.and_then(|skill_key| {
                    controller
                        .get_skill_for_key(skill_key)
                        .map(|skill| (skill_key, skill))
                })
            {
                // can get here only when alt was down and OnKeyRelease
                if alt_down {
                    log::debug!("Player wants to cast {:?}, SELF", skill);
                    Some(PlayerIntention::Casting(skill, true))
                } else {
                    None
                }
            } else if controller.right_mouse_pressed {
                Some(PlayerIntention::MoveTowardsMouse(controller.mouse_pos()))
            } else if controller.right_mouse_down {
                Some(PlayerIntention::MoveTowardsMouse(controller.mouse_pos()))
            } else if let Some(PlayerIntention::MoveTowardsMouse(pos)) = &controller.last_action {
                // user released the mouse, so it is not a MoveTowardsMouse but a move to command
                Some(PlayerIntention::MoveOrAttackTo((*pos).clone()))
            } else {
                None
            };

            let mouse_world_pos = InputConsumerSystem::picking_2d_3d(
                controller.last_mouse_x,
                controller.last_mouse_y,
                &controller.camera.pos(),
                &system_vars.matrices.projection,
                &controller.view_matrix,
            );
            controller.calc_entity_below_cursor();

            controller.cell_below_cursor_walkable = system_vars.map_render_data.gat.is_walkable(
                mouse_world_pos.x.max(0.0) as usize,
                mouse_world_pos.y.abs() as usize,
            );
            controller.mouse_world_pos = mouse_world_pos;

            if controller.is_key_just_released(Scancode::F12) {
                match controller.get_skill_for_key(SkillKey::Q) {
                    Some(Skills::FireWall) => {
                        controller.assign_skill(SkillKey::Q, Skills::Poison);
                        controller.assign_skill(SkillKey::W, Skills::FireBomb);
                        controller.assign_skill(SkillKey::E, Skills::Cure);
                        controller.assign_skill(SkillKey::R, Skills::Lightning);
                    }
                    Some(Skills::Poison) => {
                        controller.assign_skill(SkillKey::Q, Skills::FireWall);
                        controller.assign_skill(SkillKey::W, Skills::AbsorbShield);
                        controller.assign_skill(SkillKey::E, Skills::Heal);
                        controller.assign_skill(SkillKey::R, Skills::BrutalTestSkill);
                    }
                    _ => {}
                }
            }

            //            let camera_speed = if controller.keys.contains(&Scancode::LShift) { 6.0 } else { 2.0 };
            //            if controller.keys.contains(&Scancode::W) {
            //                controller.camera.move_forward(camera_speed);
            //            } else if controller.keys.contains(&Scancode::S) {
            //                controller.camera.move_forward(-camera_speed);
            //            }
            //            if controller.keys.contains(&Scancode::A) {
            //                controller.camera.move_side(-camera_speed);
            //            } else if controller.keys.contains(&Scancode::D) {
            //                controller.camera.move_side(camera_speed);
            //            }
        }
    }
}

impl InputConsumerSystem {
    pub fn target_selection_or_casting(skill_key: SkillKey, skill: Skills) -> PlayerIntention {
        // NoTarget skills have to be casted immediately without selecting target
        if skill.get_skill_target_type() == SkillTargetType::NoTarget {
            log::debug!("Skill '{:?}' is no target, so cast it", skill);
            PlayerIntention::Casting(skill, true)
        } else {
            PlayerIntention::CastingSelectTarget(skill_key, skill)
        }
    }

    pub fn picking_2d_3d(
        x2d: u16,
        y2d: u16,
        camera_pos: &Point3<f32>,
        projection: &Matrix4<f32>,
        view: &Matrix4<f32>,
    ) -> WorldCoords {
        let screen_point = Point2::new(x2d as f32, y2d as f32);

        let ray_clip = Vector4::new(
            2.0 * screen_point.x as f32 / VIDEO_WIDTH as f32 - 1.0,
            1.0 - (2.0 * screen_point.y as f32) / VIDEO_HEIGHT as f32,
            -1.0,
            1.0,
        );
        let ray_eye = projection.try_inverse().unwrap() * ray_clip;
        let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
        let ray_world = view.try_inverse().unwrap() * ray_eye;
        let ray_world = Vector3::new(ray_world.x, ray_world.y, ray_world.z).normalize();

        let line_location = camera_pos;
        let line_direction: Vector3<f32> = ray_world;
        let plane_normal = Vector3::new(0.0, 1.0, 0.0);
        let plane_point = Vector3::new(0.0, 0.0, 0.0);
        let t = (plane_normal.dot(&plane_point) - plane_normal.dot(&line_location.coords))
            / plane_normal.dot(&line_direction);
        let world_pos = line_location + (line_direction.scale(t));
        return v2!(world_pos.x, world_pos.z);
    }
}
