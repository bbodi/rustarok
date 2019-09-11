use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CameraComponent, CameraMode, ControllerEntityId, HumanInputComponent, PlayerIntention,
    SkillKey, WorldCoords,
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
use std::iter::Enumerate;
use std::slice::Iter;
use std::time::SystemTime;
use websocket::{OwnedMessage, WebSocketError};

pub struct BrowserInputProducerSystem;

const PACKET_MOUSE_MOVE: i32 = 1;
const PACKET_MOUSE_DOWN: i32 = 2;
const PACKET_MOUSE_UP: i32 = 3;
const PACKET_KEY_DOWN: i32 = 4;
const PACKET_KEY_UP: i32 = 5;
const PACKET_MOUSE_WHEEL: i32 = 6;

fn read_u16(iter: &mut Enumerate<Iter<u8>>) -> u16 {
    let (_, upper_byte) = iter.next().unwrap();
    let (_, lower_byte) = iter.next().unwrap();
    return ((*upper_byte as u16) << 8) | *lower_byte as u16;
}

fn read_i16(iter: &mut Enumerate<Iter<u8>>) -> i16 {
    let (_, upper_byte) = iter.next().unwrap();
    let (_, lower_byte) = iter.next().unwrap();
    return ((*upper_byte as i16) << 8) | *lower_byte as i16;
}

impl<'a> specs::System<'a> for BrowserInputProducerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, HumanInputComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (entities, mut input_storage, mut browser_client_storage, char_state_storage, _updater): Self::SystemData,
    ) {
        for (controller_id, client, input_producer) in
            (&entities, &mut browser_client_storage, &mut input_storage).join()
        {
            let controller_id = ControllerEntityId(controller_id);
            match client.receive() {
                Ok(msg) => match msg {
                    OwnedMessage::Pong(buf) => {
                        // TODO
                        //                        let ping_time = u128::from_le_bytes([
                        //                            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
                        //                            buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
                        //                        ]);
                        //                        let now_ms = SystemTime::now()
                        //                            .duration_since(SystemTime::UNIX_EPOCH)
                        //                            .unwrap()
                        //                            .as_millis();
                        //                        client.ping = (now_ms - ping_time) as u16;
                    }
                    OwnedMessage::Binary(buf) => {
                        let mut iter = buf.iter().enumerate();
                        while let Some((index, header)) = iter.next() {
                            let read_bytes = index + 1;
                            let remaining_bytes = buf.len() - read_bytes;
                            let header = *header as i32;
                            match header & 0b1111 {
                                PACKET_MOUSE_MOVE => {
                                    let expected_size = 2 * 2;
                                    if remaining_bytes < expected_size {
                                        continue;
                                    }
                                    let mouse_x: u16 = read_u16(&mut iter);
                                    let mouse_y: u16 = read_u16(&mut iter);
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
                                    let expected_size = 1 + 2;
                                    if remaining_bytes < expected_size {
                                        continue;
                                    }
                                    let scancode = *iter.next().unwrap().1;
                                    let modifiers: u8 = *iter.next().unwrap().1;
                                    let input_char: u16 = read_u16(&mut iter);
                                    log::trace!(
                                        "Message arrived: KeyDown({}, {}, mod: {})",
                                        scancode,
                                        input_char,
                                        modifiers
                                    );
                                    let keymod = match modifiers {
                                        0b11 => {
                                            sdl2::keyboard::Mod::LALTMOD
                                                | sdl2::keyboard::Mod::LCTRLMOD
                                        }
                                        0b10 => sdl2::keyboard::Mod::LALTMOD,
                                        0b01 => sdl2::keyboard::Mod::LCTRLMOD,
                                        _ => sdl2::keyboard::Mod::NOMOD,
                                    };
                                    input_producer.inputs.push(sdl2::event::Event::KeyDown {
                                        timestamp: 0,
                                        window_id: 0,
                                        keycode: None,
                                        scancode: Scancode::from_i32(scancode as i32),
                                        keymod,
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
                                    let scancode = *iter.next().unwrap().1;
                                    let modifiers = *iter.next().unwrap().1;
                                    log::trace!("Message arrived: KeyUp({})", scancode);
                                    let keymod = match modifiers {
                                        0b11 => {
                                            sdl2::keyboard::Mod::LALTMOD
                                                | sdl2::keyboard::Mod::LCTRLMOD
                                        }
                                        0b10 => sdl2::keyboard::Mod::LALTMOD,
                                        0b01 => sdl2::keyboard::Mod::LCTRLMOD,
                                        _ => sdl2::keyboard::Mod::NOMOD,
                                    };
                                    input_producer.inputs.push(sdl2::event::Event::KeyUp {
                                        timestamp: 0,
                                        window_id: 0,
                                        keycode: None,
                                        scancode: Scancode::from_i32(scancode as i32),
                                        keymod,
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
                                    entities.delete(controller_id.0).unwrap();
                                }
                            };
                        }
                    }
                    _ => {
                        log::warn!("Unknown msg: {:?}", msg);
                        BrowserInputProducerSystem::remove_browser_client(
                            &entities,
                            &char_state_storage,
                            controller_id,
                            &input_producer.username,
                        );
                    }
                },
                Err(WebSocketError::IoError(e)) => {
                    if e.kind() == ErrorKind::ConnectionAborted {
                        // 10053, ConnectionAborted
                        log::info!("Client '{:?}' has disconnected", controller_id);
                        BrowserInputProducerSystem::remove_browser_client(
                            &entities,
                            &char_state_storage,
                            controller_id,
                            &input_producer.username,
                        );
                    }
                }
                Err(WebSocketError::ProtocolError(_)) => {}
                Err(WebSocketError::RequestError(_)) => {}
                Err(WebSocketError::ResponseError(_)) => {}
                Err(WebSocketError::DataFrameError(_)) => {}
                Err(WebSocketError::NoDataAvailable) => {}
                Err(WebSocketError::HttpError(_)) => {}
                Err(WebSocketError::UrlError(_)) => {}
                Err(WebSocketError::WebSocketUrlError(_)) => {}
                Err(WebSocketError::TlsError(_)) => {}
                Err(WebSocketError::TlsHandshakeFailure) => {}
                Err(WebSocketError::TlsHandshakeInterruption) => {}
                Err(WebSocketError::Utf8Error(_)) => {}
            }
        }
    }
}

pub struct InputConsumerSystem;

impl<'a> specs::System<'a> for InputConsumerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, HumanInputComponent>,
        specs::WriteStorage<'a, CameraComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (entities, mut input_storage, mut camera_storage, browser_storage, system_vars): Self::SystemData,
    ) {
        for (controller_id, input, camera) in
            (&entities, &mut input_storage, &mut camera_storage).join()
        {
            // for autocompletion...
            let input: &mut HumanInputComponent = input;

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
                        }
                    }
                    sdl2::event::Event::TextInput { text, .. } => {
                        input.text = text;
                    }
                    _ => {}
                }
            }

            if input.is_key_just_pressed(Scancode::Grave)
                && input.alt_down
                && browser_storage.get(controller_id).is_none()
            {
                input.is_console_open = !input.is_console_open;
            }

            if input.is_key_just_released(Scancode::L) && !input.is_console_open {
                match input.camera_movement_mode {
                    CameraMode::Free => {
                        input.camera_movement_mode = CameraMode::FollowChar;
                        camera.reset_y_and_angle(&system_vars.matrices.projection);
                    }
                    CameraMode::FollowChar => {
                        input.camera_movement_mode = CameraMode::FreeMoveButFixedAngle
                    }
                    CameraMode::FreeMoveButFixedAngle => {
                        input.camera_movement_mode = CameraMode::Free
                    }
                }
            }

            let mouse_world_pos = InputConsumerSystem::picking_2d_3d(
                input.last_mouse_x,
                input.last_mouse_y,
                &camera.camera.pos(),
                &system_vars.matrices.projection,
                &camera.view_matrix,
            );
            input.mouse_world_pos = mouse_world_pos;

            if input.is_key_just_released(Scancode::F12) {
                match input.get_skill_for_key(SkillKey::Q) {
                    Some(Skills::FireWall) => {
                        input.assign_skill(SkillKey::Q, Skills::Poison);
                        input.assign_skill(SkillKey::W, Skills::FireBomb);
                        input.assign_skill(SkillKey::E, Skills::Cure);
                        input.assign_skill(SkillKey::R, Skills::Lightning);
                    }
                    Some(Skills::Poison) => {
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
}

impl InputConsumerSystem {
    pub fn target_selection_or_casting(
        skill: Skills,
        mouse_pos: WorldCoords,
    ) -> Option<PlayerIntention> {
        // NoTarget skills have to be casted immediately without selecting target
        if skill.get_skill_target_type() == SkillTargetType::NoTarget {
            log::debug!("Skill '{:?}' is no target, so cast it", skill);
            Some(PlayerIntention::Casting(skill, true, mouse_pos))
        } else {
            None
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

impl BrowserInputProducerSystem {
    fn remove_browser_client(
        entities: &Entities,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        controller_id: ControllerEntityId,
        username: &str,
    ) {
        for (char_id, char_state) in (entities, char_state_storage).join() {
            if char_state.name == username {
                entities.delete(char_id).unwrap();
                break;
            }
        }
        entities.delete(controller_id.0).unwrap();
    }
}
