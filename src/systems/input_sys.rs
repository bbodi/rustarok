use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CameraComponent, CameraMode, HumanInputComponent, PlayerIntention, SkillKey, WorldCoords,
};
use crate::components::skills::skill::{SkillTargetType, Skills};
use crate::components::BrowserClient;
use crate::cursor::{CursorFrame, CURSOR_CLICK, CURSOR_NORMAL, CURSOR_STOP, CURSOR_TARGET};
use crate::systems::SystemVariables;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::ElapsedTime;
use nalgebra::{Matrix4, Point2, Point3, Vector2, Vector3, Vector4};
use sdl2::keyboard::Scancode;
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use specs::prelude::*;
use std::io::ErrorKind;
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
        specs::WriteStorage<'a, HumanInputComponent>,
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
                                    entities.delete(entity_id).unwrap();
                                }
                            };
                        }
                    }
                    _ => {
                        log::warn!("Unknown msg: {:?}", msg);
                        entities.delete(entity_id).unwrap();
                    }
                }
            } else if let Err(WebSocketError::IoError(e)) = sh {
                if e.kind() == ErrorKind::ConnectionAborted {
                    // 10053, ConnectionAborted
                    log::info!("Client '{:?}' has disconnected", entity_id);
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
        specs::WriteStorage<'a, HumanInputComponent>,
        specs::WriteStorage<'a, CameraComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (entities, char_state_storage, mut input_storage, mut camera_storage, system_vars): Self::SystemData,
    ) {
        for (entity_id, input, camera) in
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
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        if let Some(scancode) = scancode {
                            input.key_pressed(scancode);
                        }
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        if let Some(scancode) = scancode {
                            input.key_released(scancode);
                        }
                    }
                    _ => {}
                }
            }

            if input.is_key_just_released(Scancode::L) {
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
            input.calc_entity_below_cursor();

            input.cell_below_cursor_walkable = system_vars.map_render_data.gat.is_walkable(
                mouse_world_pos.x.max(0.0) as usize,
                mouse_world_pos.y.abs() as usize,
            );
            input.mouse_world_pos = mouse_world_pos;

            let (cursor_frame, cursor_color) = InputConsumerSystem::determine_cursor(
                system_vars.time,
                entity_id,
                input,
                &char_state_storage,
            );

            input.cursor_anim_descr.action_index = cursor_frame.1;
            input.cursor_color = cursor_color;

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
    pub fn determine_cursor(
        now: ElapsedTime,
        self_id: Entity,
        input: &HumanInputComponent,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
    ) -> (CursorFrame, [f32; 3]) {
        return if let Some((_skill_key, skill)) = input.select_skill_target {
            let is_castable = char_state_storage
                .get(self_id)
                .unwrap()
                .skill_cast_allowed_at
                .get(&skill)
                .unwrap_or(&ElapsedTime(0.0))
                .is_earlier_than(now);
            if !is_castable {
                (CURSOR_STOP, [1.0, 1.0, 1.0])
            } else if skill.get_skill_target_type() != SkillTargetType::Area {
                (CURSOR_TARGET, [1.0, 1.0, 1.0])
            } else {
                (CURSOR_CLICK, [1.0, 1.0, 1.0])
            }
        } else if let Some(entity_below_cursor) = input.entity_below_cursor {
            let self_team = char_state_storage.get(self_id).unwrap().team;
            let ent_is_dead_or_friend = char_state_storage
                .get(entity_below_cursor)
                .map(|it| !it.state().is_alive() || it.team == self_team)
                .unwrap_or(false);
            if entity_below_cursor == self_id || ent_is_dead_or_friend {
                // self or dead
                (CURSOR_NORMAL, [0.2, 0.46, 0.9])
            } else {
                (CURSOR_NORMAL, [1.0, 0.0, 0.0])
            }
        } else if !input.cell_below_cursor_walkable {
            (CURSOR_STOP, [1.0, 1.0, 1.0])
        } else {
            (CURSOR_NORMAL, [1.0, 1.0, 1.0])
        };
    }

    pub fn target_selection_or_casting(
        skill: Skills,
        mouse_pos: WorldCoords,
        entity_below_cursor: Option<Entity>,
    ) -> Option<PlayerIntention> {
        // NoTarget skills have to be casted immediately without selecting target
        if skill.get_skill_target_type() == SkillTargetType::NoTarget {
            log::debug!("Skill '{:?}' is no target, so cast it", skill);
            Some(PlayerIntention::Casting(
                skill,
                true,
                mouse_pos,
                entity_below_cursor,
            ))
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
