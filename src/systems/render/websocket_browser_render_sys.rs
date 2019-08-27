use crate::components::BrowserClient;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::SystemFrameDurations;
use byteorder::{LittleEndian, WriteBytesExt};
use specs::prelude::*;

pub struct WebSocketBrowserRenderSystem {}

impl WebSocketBrowserRenderSystem {
    pub fn new() -> WebSocketBrowserRenderSystem {
        WebSocketBrowserRenderSystem {}
    }
}

impl<'a> specs::System<'a> for WebSocketBrowserRenderSystem {
    type SystemData = (
        specs::ReadStorage<'a, RenderCommandCollectorComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(
        &mut self,
        (render_commands_storage, browser_client_storage, mut system_benchmark): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("WebSocketBrowserRenderSystem");

        for (render_commands, browser) in (&render_commands_storage, &browser_client_storage).join()
        {
            let render_commands: &RenderCommandCollectorComponent = render_commands;

            let mut send_buffer = Vec::<u8>::with_capacity(1024);
            /////////////////////////////////
            // 2D Trimesh
            /////////////////////////////////

            /////////////////////////////////
            // 2D Texture
            /////////////////////////////////

            /////////////////////////////////
            // 2D Rectangle
            /////////////////////////////////

            /////////////////////////////////
            // 3D Rectangle
            /////////////////////////////////

            /////////////////////////////////
            // 3D Circles
            /////////////////////////////////

            /////////////////////////////////
            // 3D Sprites
            /////////////////////////////////
            {
                for v in render_commands.view_matrix.as_slice() {
                    send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                }

                for command in &render_commands.billboard_commands {
                    send_buffer
                        .write_f32::<LittleEndian>(command.texture_width)
                        .unwrap();
                    send_buffer
                        .write_f32::<LittleEndian>(command.texture_height)
                        .unwrap();

                    for v in &command.common.color {
                        send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                    }
                    for v in &command.common.offset {
                        send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                    }
                    for v in &command.common.matrix {
                        send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                    }
                    send_buffer
                        .write_u16::<LittleEndian>(command.texture.0 as u16)
                        .unwrap();
                    send_buffer.write_u16::<LittleEndian>(0).unwrap();
                }

                let message = websocket::Message::binary(send_buffer.as_slice());
                //sent_bytes_per_second_counter += client.offscreen.len();
                // it is ok if it fails, the client might have disconnected but
                // ecs_world.maintain has not been executed yet to remove it from the world
                let _result = browser.websocket.lock().unwrap().send_message(&message);
            }

            /////////////////////////////////
            // NUMBERS
            /////////////////////////////////
            {}

            /////////////////////////////////
            // EFFECTS
            /////////////////////////////////
            {}

            /////////////////////////////////
            // MODELS
            /////////////////////////////////
            {}
        }
    }
}
