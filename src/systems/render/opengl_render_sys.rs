use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::{SystemFrameDurations, SystemVariables};
use specs::prelude::*;

pub struct OpenGlRenderSystem;

impl OpenGlRenderSystem {
    pub fn new() -> OpenGlRenderSystem {
        OpenGlRenderSystem
    }
}

impl<'a> specs::System<'a> for OpenGlRenderSystem {
    type SystemData = (
        specs::ReadStorage<'a, RenderCommandCollectorComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (render_commands_storage, mut system_benchmark, system_vars): Self::SystemData,
    ) {
        let stopwatch = system_benchmark.start_measurement("OpenGlRenderSystem");
        for render_commands in render_commands_storage.join() {
            let render_commands: &RenderCommandCollectorComponent = render_commands;
            let shader = system_vars.shaders.trimesh2d_shader.gl_use();
            shader.set_mat4("projection", &system_vars.matrices.ortho);
            for trimesh_2d in &render_commands.trimesh_2d_commands {
                // TODO: move bind out of the loop by grouping same vaos
                shader.set_mat4("model", &trimesh_2d.matrix);
                shader.set_vec4("color", &trimesh_2d.color);
                shader.set_vec2("size", &trimesh_2d.size);
                shader.set_f32("z", 0.01 * trimesh_2d.layer as usize as f32);
                trimesh_2d.vao.bind().draw();
            }

            for command in &render_commands.texture_2d_commands {
                let width = command.texture_width as f32;
                let height = command.texture_height as f32;
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, command.texture);
                }
                let shader = system_vars.shaders.sprite2d_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.ortho);
                shader.set_mat4("model", &command.matrix);
                shader.set_int("model_texture", 0);
                shader.set_f32("z", 0.01 * command.layer as usize as f32);
                shader.set_vec2("offset", &command.offset);
                shader.set_vec2("size", &[width * command.size, height * command.size]);
                shader.set_vec4("color", &command.color);
                system_vars
                    .map_render_data
                    .sprite_vertex_array
                    .bind()
                    .draw();
            }
        }
    }
}
