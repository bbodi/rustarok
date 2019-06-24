use websocket::{OwnedMessage, WebSocketError};
use std::time::SystemTime;
use sdl2::keyboard::Scancode;
use std::io::ErrorKind;
use crate::components::{InputProducerComponent, CameraComponent, BrowserClient};
use specs::prelude::*;

pub struct BrowserInputProducerSystem;

impl<'a> specs::System<'a> for BrowserInputProducerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, InputProducerComponent>,
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
        specs::WriteStorage<'a, InputProducerComponent>,
        specs::WriteStorage<'a, CameraComponent>,
    );

    fn run(&mut self, (
        mut input_storage,
        mut camera_storage,
    ): Self::SystemData) {
        for (client, input_producer) in (&mut camera_storage, &mut input_storage).join() {
            let events: Vec<_> = input_producer.inputs.drain(..).collect();
            for event in events {
                match event {
                    sdl2::event::Event::MouseButtonDown { .. } => {
                        client.mouse_down = true;
                    }
                    sdl2::event::Event::MouseButtonUp { .. } => {
                        client.mouse_down = false;
                    }
                    sdl2::event::Event::MouseMotion {
                        timestamp: _,
                        window_id: _,
                        which: _,
                        mousestate: _,
                        x,
                        y,
                        xrel: _,
                        yrel: _
                    } => {
                        if client.mouse_down {
                            let x_offset = x - client.last_mouse_x as i32;
                            let y_offset = client.last_mouse_y as i32 - y; // reversed since y-coordinates go from bottom to top
                            client.yaw += x_offset as f32;
                            client.pitch += y_offset as f32;
                            if client.pitch > 89.0 {
                                client.pitch = 89.0;
                            }
                            if client.pitch < -89.0 {
                                client.pitch = -89.0;
                            }
                            if client.yaw > 360.0 {
                                client.yaw -= 360.0;
                            } else if client.yaw < 0.0 {
                                client.yaw += 360.0;
                            }
                            client.camera.rotate(client.pitch, client.yaw);
                        }
                        client.last_mouse_x = x as u16;
                        client.last_mouse_y = y as u16;
                    }
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        if scancode.is_some() {
                            input_producer.keys.insert(scancode.unwrap());
                        }
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        if scancode.is_some() {
                            input_producer.keys.remove(&scancode.unwrap());
                        }
                    }
                    _ => {}
                }
            }
            let camera_speed = if input_producer.keys.contains(&Scancode::LShift) { 6.0 } else { 2.0 };
            if input_producer.keys.contains(&Scancode::W) {
                client.camera.move_forward(camera_speed);
            } else if input_producer.keys.contains(&Scancode::S) {
                client.camera.move_forward(-camera_speed);
            }
            if input_producer.keys.contains(&Scancode::A) {
                client.camera.move_side(-camera_speed);
            } else if input_producer.keys.contains(&Scancode::D) {
                client.camera.move_side(camera_speed);
            }
        }
    }
}