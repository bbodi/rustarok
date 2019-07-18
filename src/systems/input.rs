use websocket::{OwnedMessage, WebSocketError};
use std::time::SystemTime;
use sdl2::keyboard::Scancode;
use std::io::ErrorKind;
use crate::components::BrowserClient;
use specs::prelude::*;
use crate::video::{VIDEO_WIDTH, VIDEO_HEIGHT};
use crate::systems::SystemVariables;
use sdl2::mouse::MouseButton;
use crate::components::controller::{ControllerComponent, CastMode, ControllerAction, SkillKey, ALL_SKILL_KEYS, WorldCoords};
use crate::cam::Camera;
use crate::RenderMatrices;
use nalgebra::{Point2, Vector3, Vector4};
use crate::components::char::{PhysicsComponent, CharacterStateComponent};

pub struct BrowserInputProducerSystem;

impl<'a> specs::System<'a> for BrowserInputProducerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
    );

    fn run(&mut self, (
        entities,
        mut input_storage,
        mut browser_client_storage,
    ): Self::SystemData) {
        for (entity, client, input_producer) in (&entities, &mut browser_client_storage, &mut input_storage).join() {
            let sh = client.websocket.lock().unwrap().recv_message();
            if let Ok(msg) = sh {
                match msg {
                    OwnedMessage::Pong(buf) => {
                        let ping_time = u128::from_le_bytes([
                            buf[0], buf[1], buf[2], buf[3],
                            buf[4], buf[5], buf[6], buf[7],
                            buf[8], buf[9], buf[10], buf[11],
                            buf[12], buf[13], buf[14], buf[15],
                        ]);
                        let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
                        client.ping = (now_ms - ping_time) as u16;
                    }
                    OwnedMessage::Binary(buf) => {
                        let mut iter = buf.iter();
                        while let Some(header) = iter.next() {
                            match header {
                                1 => {
                                    let upper_byte = iter.next().unwrap();
                                    let lower_byte = iter.next().unwrap();
                                    let mouse_x: u16 = ((*upper_byte as u16) << 8) | *lower_byte as u16;

                                    let upper_byte = iter.next().unwrap();
                                    let lower_byte = iter.next().unwrap();
                                    let mouse_y: u16 = ((*upper_byte as u16) << 8) | *lower_byte as u16;
                                    trace!("Message arrived: MouseMove({}, {})", mouse_x, mouse_y);
                                    let shit2 = (0 as u32,
                                                 0 as i32,
                                                 0 as i32);
                                    let shit = unsafe { std::mem::transmute(shit2) };
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseMotion {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mousestate: shit,
                                            x: mouse_x as i32,
                                            y: mouse_y as i32,
                                            xrel: 0,
                                            yrel: 0,
                                        }
                                    );
                                }
                                2 => {
                                    trace!("Message arrived: MouseDown");
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseButtonDown {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn: sdl2::mouse::MouseButton::Left,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        }
                                    );
                                }
                                3 => {
                                    trace!("Message arrived: MouseUp");
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseButtonUp {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn: sdl2::mouse::MouseButton::Left,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        });
                                }
                                4 => {
                                    let scancode = *iter.next().unwrap();
                                    let upper_byte = *iter.next().unwrap();
                                    let lower_byte = *iter.next().unwrap();
                                    let input_char: u16 = ((upper_byte as u16) << 8) | lower_byte as u16;
                                    trace!("Message arrived: KeyDown({}, {})", scancode, input_char);
                                    input_producer.inputs.push(
                                        sdl2::event::Event::KeyDown {
                                            timestamp: 0,
                                            window_id: 0,
                                            keycode: None,
                                            scancode: Scancode::from_i32(scancode as i32),
                                            keymod: sdl2::keyboard::Mod::NOMOD,
                                            repeat: false,
                                        });
                                    if let Some(ch) = std::char::from_u32(input_char as u32) {
                                        input_producer.inputs.push(
                                            sdl2::event::Event::TextInput {
                                                timestamp: 0,
                                                window_id: 0,
                                                text: ch.to_string(),
                                            }
                                        );
                                    }
                                }
                                5 => {
                                    let scancode = *iter.next().unwrap();
                                    trace!("Message arrived: KeyUp({})", scancode);
                                    input_producer.inputs.push(
                                        sdl2::event::Event::KeyUp {
                                            timestamp: 0,
                                            window_id: 0,
                                            keycode: None,
                                            scancode: Scancode::from_i32(scancode as i32),
                                            keymod: sdl2::keyboard::Mod::NOMOD,
                                            repeat: false,
                                        });
                                }
                                _ => {
                                    warn!("Unknown header: {}", header);
                                    entities.delete(entity).unwrap();
                                }
                            };
                        }
                    }
                    _ => {
                        warn!("Unknown msg: {:?}", msg);
                        entities.delete(entity).unwrap();
                    }
                }
            } else if let Err(WebSocketError::IoError(e)) = sh {
                if e.kind() == ErrorKind::ConnectionAborted {
                    // 10053, ConnectionAborted
                    info!("Client has disconnected");
                    entities.delete(entity).unwrap();
                }
            }
        }
    }
}

pub struct InputConsumerSystem;

impl<'a> specs::System<'a> for InputConsumerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        entities,
        physics_storage,
        char_state_storage,
        mut controller_storage,
        mut system_vars,
    ): Self::SystemData) {
        for (controller) in (&mut controller_storage).join() {
            // for autocompletion...
            let controller: &mut ControllerComponent = controller;


            let camera_speed = if controller.is_key_down(Scancode::LShift) { 6.0 } else { 1.0 };
            let events: Vec<_> = controller.inputs.drain(..).collect();
            controller.left_mouse_released = false;
            controller.right_mouse_released = false;
            controller.right_mouse_pressed = false;
            controller.left_mouse_pressed = false;
            controller.cleanup_released_keys();
            for event in events {
                match event {
                    sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => {
                        match mouse_btn {
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
                        }
                    }
                    sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => {
                        match mouse_btn {
                            MouseButton::Left => {
                                controller.left_mouse_down = false;
                                controller.left_mouse_released = true;
                            }
                            MouseButton::Right => {
                                controller.right_mouse_down = false;
                                controller.right_mouse_released = true;
                            }
                            _ => {}
                        }
                    }
                    sdl2::event::Event::MouseMotion {
                        timestamp: _,
                        window_id: _,
                        which: _,
                        mousestate: _,
                        x,
                        y,
                        xrel,
                        yrel
                    } => {
                        // SDL generates only one event when the mouse touches the edge of the screen,
                        // so I put this pseudo key into the controller in that case, which will
                        // indicate screen movement
                        if x == 0 {
                            controller.key_pressed(Scancode::Left);
                            controller.camera.move_along_x(-camera_speed);
                        } else if x == (VIDEO_WIDTH as i32) - 1 {
                            controller.key_pressed(Scancode::Right);
                            controller.camera.move_along_x(camera_speed);
                        } else {
                            controller.key_released(Scancode::Left);
                            controller.key_released(Scancode::Right);
                        }
                        if y == 0 {
                            controller.key_pressed(Scancode::Up);
                            controller.camera.move_along_z(-camera_speed);
                        } else if y == (VIDEO_HEIGHT as i32) - 1 {
                            controller.key_pressed(Scancode::Down);
                            controller.camera.move_along_z(camera_speed);
                        } else {
                            controller.key_released(Scancode::Up);
                            controller.key_released(Scancode::Down);
                        }
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
                    sdl2::event::Event::MouseWheel {
                        y,
                        ..
                    } => {
                        controller.camera.move_forward(y as f32);
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
            } else if controller.camera.pos().x > system_vars.map_render_data.gnd.width as f32 * 2.0 {
                controller.camera.set_x(system_vars.map_render_data.gnd.width as f32 * 2.0);
            }
            if controller.camera.pos().z > 0.0 {
                controller.camera.set_z(0.0);
            } else if controller.camera.pos().z < -(system_vars.map_render_data.gnd.height as f32 * 2.0) {
                controller.camera.set_z(-(system_vars.map_render_data.gnd.height as f32 * 2.0));
            }
            // setup next action baed on input
            // TODO: optimize
            let just_pressed_skill_key = ALL_SKILL_KEYS
                .iter()
                .filter(|key| {
                    controller.is_key_just_pressed(key.scancode())
                }).take(1).map(|it| *it).collect::<Vec<SkillKey>>().pop();
            let just_released_skill_key = ALL_SKILL_KEYS
                .iter()
                .filter(|key| {
                    controller.is_key_just_released(key.scancode())
                }).take(1).map(|it| *it).collect::<Vec<SkillKey>>().pop();

            if controller.next_action.is_some() {
                controller.last_action = std::mem::replace(&mut controller.next_action, None);
                ;
            }
            controller.next_action = if let Some(casting_skill_key) = controller.is_casting_selection {
                match controller.cast_mode {
                    CastMode::Normal => {
                        if controller.left_mouse_released {
                            Some(ControllerAction::Casting(casting_skill_key))
                        } else if controller.right_mouse_pressed {
                            Some(ControllerAction::CancelCastingSelectTarget)
                        } else { None }
                    }
                    CastMode::OnKeyRelease => {
                        if controller.is_key_just_released(casting_skill_key.scancode()) {
                            Some(ControllerAction::Casting(casting_skill_key))
                        } else if controller.right_mouse_pressed {
                            Some(ControllerAction::CancelCastingSelectTarget)
                        } else { None }
                    }
                    CastMode::OnKeyPress => {
                        // not possible to get into this state while OnKeyPress is active
                        None
                    }
                }
            } else if let Some(skill_key) = just_pressed_skill_key {
                match controller.cast_mode {
                    CastMode::Normal | CastMode::OnKeyRelease => {
                        Some(ControllerAction::CastingSelectTarget(skill_key))
                    }
                    CastMode::OnKeyPress => {
                        Some(ControllerAction::Casting(skill_key))
                    }
                }
            } else if controller.right_mouse_pressed {
                Some(ControllerAction::MoveTowardsMouse(controller.mouse_pos()))
            } else if controller.right_mouse_down {
                Some(ControllerAction::MoveTowardsMouse(controller.mouse_pos()))
            } else if let Some(ControllerAction::MoveTowardsMouse(pos)) = &controller.last_action {
                // user released the mouse, so it is not a MoveTowardsMouse but a move to command
                Some(ControllerAction::MoveOrAttackTo((*pos).clone()))
            } else {
                None
            };
            controller.is_casting_selection = match controller.next_action {
                Some(ControllerAction::CastingSelectTarget(skill_key)) => Some(skill_key),
                None => controller.is_casting_selection,
                _ => None,
            };

            let mouse_world_pos = InputConsumerSystem::picking_2d_3d(controller.last_mouse_x,
                                                                     controller.last_mouse_y,
                                                                     &controller.camera,
                                                                     &system_vars.matrices);
            let mut entity_below_cursor: Option<Entity> = None;
            for (entity, other_char_state, other_physics) in (&entities, &char_state_storage, &physics_storage).join() {
                let bb = &other_char_state.bounding_rect_2d;
                let mx = controller.last_mouse_x as i32;
                let my = controller.last_mouse_y as i32;
                if mx >= bb.bottom_left[0] && mx <= bb.top_right[0] &&
                    my <= bb.bottom_left[1] && my >= bb.top_right[1] {
                    entity_below_cursor = Some(entity);
                    break;
                }
            }
            controller.entity_below_cursor = entity_below_cursor;
            // TODO: thread '<unnamed>' panicked at 'attempt to add with overflow', src\gat.rs:35:24
            controller.cell_below_cursor_walkable = system_vars.map_render_data.gat.is_walkable(
                mouse_world_pos.x as usize,
                mouse_world_pos.y.abs() as usize,
            );
            controller.mouse_world_pos = mouse_world_pos;

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
    pub fn picking_2d_3d(x2d: u16, y2d: u16, camera: &Camera, matrices: &RenderMatrices) -> WorldCoords {
        let screen_point = Point2::new(x2d as f32, y2d as f32);

        let ray_clip = Vector4::new(2.0 * screen_point.x as f32 / VIDEO_WIDTH as f32 - 1.0,
                                    1.0 - (2.0 * screen_point.y as f32) / VIDEO_HEIGHT as f32,
                                    -1.0,
                                    1.0);
        let ray_eye = matrices.projection.try_inverse().unwrap() * ray_clip;
        let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
        let ray_world = matrices.view.try_inverse().unwrap() * ray_eye;
        let ray_world = Vector3::new(ray_world.x, ray_world.y, ray_world.z).normalize();

        let line_location = camera.pos();
        let line_direction: Vector3<f32> = ray_world;
        let plane_normal = Vector3::new(0.0, 1.0, 0.0);
        let plane_point = Vector3::new(0.0, 0.0, 0.0);
        let t = (plane_normal.dot(&plane_point) - plane_normal.dot(&line_location.coords)) / plane_normal.dot(&line_direction);
        let world_pos = line_location + (line_direction.scale(t));
        return Point2::new(world_pos.x, world_pos.z);
    }
}