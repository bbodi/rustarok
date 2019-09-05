use crate::components::BrowserClient;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::{SystemFrameDurations, SystemVariables};
use byteorder::{LittleEndian, WriteBytesExt};
use specs::prelude::*;

pub struct WebSocketBrowserRenderSystem {
    send_buffer: Vec<u8>,
}

impl WebSocketBrowserRenderSystem {
    pub fn new() -> WebSocketBrowserRenderSystem {
        WebSocketBrowserRenderSystem {
            send_buffer: Vec::<u8>::with_capacity(60 * 1024),
        }
    }
}

impl<'a> specs::System<'a> for WebSocketBrowserRenderSystem {
    type SystemData = (
        specs::ReadStorage<'a, RenderCommandCollectorComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (render_commands_storage, mut browser_client_storage, mut system_benchmark, system_vars): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("WebSocketBrowserRenderSystem");
        if system_vars.tick
            % system_vars
                .dev_configs
                .network
                .send_render_data_every_nth_frame
                .max(1)
            != 0
        {
            return;
        }

        for (render_commands, browser) in
            (&render_commands_storage, &mut browser_client_storage).join()
        {
            let render_commands: &RenderCommandCollectorComponent = render_commands;
            self.send_buffer.clear();

            for v in render_commands.view_matrix.as_slice() {
                self.send_buffer.write_f32::<LittleEndian>(*v).unwrap();
            }
            for v in render_commands.normal_matrix.as_slice() {
                self.send_buffer.write_f32::<LittleEndian>(*v).unwrap();
            }
            /////////////////////////////////
            // 2D Trimesh
            /////////////////////////////////

            /////////////////////////////////
            // 2D Texture
            /////////////////////////////////
            //            for command in &render_commands.texture_2d_commands {
            //                let width = command.texture_width as f32;
            //                let height = command.texture_height as f32;
            //                unsafe {
            //                    gl::BindTexture(gl::TEXTURE_2D, command.texture.0);
            //                }
            //                shader.set_mat4("model", &command.matrix);
            //                shader.set_f32("z", 0.01 * command.layer as usize as f32);

            //                shader.set_vec2("offset", &command.offset);
            //                shader.set_vec2("size", &[width * command.size, height * command.size]);
            //                shader.set_vec4u8("color", &command.color);
            //                vertex_array_bind.draw();

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
                self.send_buffer
                    .write_u32::<LittleEndian>(render_commands.billboard_commands.len() as u32)
                    .unwrap();
                for command in &render_commands.billboard_commands {
                    for v in &command.common.color {
                        self.send_buffer.write_u8(*v).unwrap();
                    }
                    for v in &command.offset {
                        self.send_buffer.write_i16::<LittleEndian>(*v).unwrap();
                    }
                    for v in &command.common.matrix {
                        self.send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                    }
                    let packed_int: u32 =
                        ((command.is_vertically_flipped as u32) << 31) | command.texture.0 as u32;

                    self.send_buffer
                        .write_u32::<LittleEndian>(packed_int)
                        .unwrap();
                }
            }

            /////////////////////////////////
            // 3D NUMBERS
            /////////////////////////////////
            {
                self.send_buffer
                    .write_u32::<LittleEndian>(render_commands.number_3d_commands.len() as u32)
                    .unwrap();
                for command in &render_commands.number_3d_commands {
                    self.send_buffer
                        .write_f32::<LittleEndian>(command.common.size)
                        .unwrap();

                    for v in &command.common.color {
                        self.send_buffer.write_u8(*v).unwrap();
                    }
                    for v in &command.common.matrix {
                        self.send_buffer.write_f32::<LittleEndian>(*v).unwrap();
                    }
                    self.send_buffer
                        .write_u32::<LittleEndian>(command.value)
                        .unwrap();
                }
            }

            /////////////////////////////////
            // 3D EFFECTS
            /////////////////////////////////
            {}

            /////////////////////////////////
            // 3D MODELS
            /////////////////////////////////
            {
                self.send_buffer
                    .write_u32::<LittleEndian>(render_commands.model_commands.len() as u32)
                    .unwrap();
                for command in &render_commands.model_commands {
                    let packed_int: u32 = ((command.is_transparent as u32) << 31)
                        | command.model_instance_index as u32;
                    self.send_buffer
                        .write_u32::<LittleEndian>(packed_int)
                        .unwrap();
                }
            }

            browser.send_message(&self.send_buffer);
        }
    }
}
