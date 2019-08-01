use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::render_sys::{DamageRenderSystem, ONE_SPRITE_PIXEL_SIZE_IN_3D};
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::video::{VertexArray, VertexAttribDefinition, TEXTURE_0};
use nalgebra::Vector3;
use specs::prelude::*;

pub struct OpenGlRenderSystem {
    centered_rectangle_vao: VertexArray,
    circle_vao: VertexArray,
    // damage rendering
    single_digit_u_coord: f32,
    texture_u_coords: [f32; 10],
}

impl OpenGlRenderSystem {
    pub fn new() -> OpenGlRenderSystem {
        let single_digit_width = 10.0;
        let texture_width = single_digit_width * 10.0;
        let single_digit_u_coord = single_digit_width / texture_width;

        OpenGlRenderSystem {
            single_digit_u_coord,
            texture_u_coords: [
                single_digit_u_coord * 0.0,
                single_digit_u_coord * 1.0,
                single_digit_u_coord * 2.0,
                single_digit_u_coord * 3.0,
                single_digit_u_coord * 4.0,
                single_digit_u_coord * 5.0,
                single_digit_u_coord * 6.0,
                single_digit_u_coord * 7.0,
                single_digit_u_coord * 8.0,
                single_digit_u_coord * 9.0,
            ],
            centered_rectangle_vao: {
                let bottom_left = v3!(-0.5, 0.0, -0.5);
                let top_left = v3!(-0.5, 0.0, 0.5);
                let top_right = v3!(0.5, 0.0, 0.5);
                let bottom_right = v3!(0.5, 0.0, -0.5);
                VertexArray::new(
                    gl::LINE_LOOP,
                    &[bottom_left, top_left, top_right, bottom_right],
                    4,
                    vec![VertexAttribDefinition {
                        number_of_components: 3,
                        offset_of_first_element: 0,
                    }],
                )
            },
            circle_vao: {
                let capsule_mesh = ncollide2d::procedural::circle(&1.0, 32);
                let coords: Vec<[f32; 3]> = capsule_mesh
                    .coords()
                    .iter()
                    .map(|it| [it.x, 0.0, it.y])
                    .collect();
                VertexArray::new(
                    gl::LINE_LOOP,
                    coords.as_slice(),
                    coords.len(),
                    vec![VertexAttribDefinition {
                        number_of_components: 3,
                        offset_of_first_element: 0,
                    }],
                )
            },
        }
    }

    pub fn create_number_vertex_array(&self, number: u32) -> VertexArray {
        let digits = DamageRenderSystem::get_digits(number);
        // create vbo based on the numbers
        let mut width = 0.0;
        let mut vertices = vec![];
        digits.iter().for_each(|&digit| {
            let digit = digit as usize;
            vertices.push([width - 0.5, 0.5, self.texture_u_coords[digit], 0.0]);
            vertices.push([
                width + 0.5,
                0.5,
                self.texture_u_coords[digit] + self.single_digit_u_coord,
                0.0,
            ]);
            vertices.push([width - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
            vertices.push([
                width + 0.5,
                0.5,
                self.texture_u_coords[digit] + self.single_digit_u_coord,
                0.0,
            ]);
            vertices.push([width - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
            vertices.push([
                width + 0.5,
                -0.5,
                self.texture_u_coords[digit] + self.single_digit_u_coord,
                1.0,
            ]);
            width += 1.0;
        });
        return VertexArray::new(
            gl::TRIANGLES,
            &vertices,
            vertices.len(),
            vec![
                VertexAttribDefinition {
                    number_of_components: 2,
                    offset_of_first_element: 0,
                },
                VertexAttribDefinition {
                    // uv
                    number_of_components: 2,
                    offset_of_first_element: 2,
                },
            ],
        );
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
        let _stopwatch = system_benchmark.start_measurement("OpenGlRenderSystem");
        for render_commands in render_commands_storage.join() {
            let render_commands: &RenderCommandCollectorComponent = render_commands;
            let shader = system_vars.assets.shaders.trimesh2d_shader.gl_use();
            shader.set_mat4("projection", &system_vars.matrices.ortho);
            for trimesh_2d in &render_commands.trimesh_2d_commands {
                // TODO: move bind out of the loop by grouping same vaos
                shader.set_mat4("model", &trimesh_2d.matrix);
                shader.set_vec4("color", &trimesh_2d.color);
                shader.set_vec2("size", &trimesh_2d.size);
                shader.set_f32("z", 0.01 * trimesh_2d.layer as usize as f32);
                trimesh_2d.vao.bind().draw();
            }

            {
                let shader = system_vars.assets.shaders.sprite2d_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.ortho);
                shader.set_int("model_texture", 0);
                let vertex_array_bind = system_vars.map_render_data.sprite_vertex_array.bind();
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0);
                }
                for command in &render_commands.texture_2d_commands {
                    let width = command.texture_width as f32;
                    let height = command.texture_height as f32;
                    unsafe {
                        gl::BindTexture(gl::TEXTURE_2D, command.texture);
                    }
                    shader.set_mat4("model", &command.matrix);
                    shader.set_f32("z", 0.01 * command.layer as usize as f32);
                    shader.set_vec2("offset", &command.offset);
                    shader.set_vec2("size", &[width * command.size, height * command.size]);
                    shader.set_vec4("color", &command.color);
                    vertex_array_bind.draw();
                }
            }

            {
                let vertex_array_bind = system_vars.map_render_data.sprite_vertex_array.bind();
                let shader = system_vars.assets.shaders.trimesh2d_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.ortho);
                for command in &render_commands.rectangle_2d_commands {
                    shader.set_vec4("color", &command.color);
                    shader.set_mat4("model", &command.matrix);
                    shader.set_vec2("size", &command.size);
                    shader.set_f32("z", 0.01 * command.layer as usize as f32);
                    vertex_array_bind.draw();
                }
            }

            {
                let centered_rectangle_vao_bind = self.centered_rectangle_vao.bind();
                let shader = system_vars.assets.shaders.trimesh_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.projection);
                shader.set_mat4("view", &render_commands.view_matrix);
                for command in &render_commands.rectangle_3d_commands {
                    shader.set_vec4("color", &command.common.color);
                    shader.set_mat4("model", &command.common.matrix);
                    shader.set_vec2("size", &command.size);
                    centered_rectangle_vao_bind.draw();
                }
            }

            {
                let vao_bind = self.circle_vao.bind();
                let shader = system_vars.assets.shaders.trimesh_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.projection);
                shader.set_mat4("view", &render_commands.view_matrix);
                for command in &render_commands.circle_3d_commands {
                    shader.set_vec4("color", &command.common.color);
                    shader.set_mat4("model", &command.common.matrix);
                    shader.set_vec2(
                        "size",
                        &[command.common.size * 2.0, command.common.size * 2.0],
                    );
                    vao_bind.draw();
                }
            }

            {
                let shader = system_vars.assets.shaders.sprite_shader.gl_use();
                let vao_bind = system_vars
                    .map_render_data
                    .centered_sprite_vertex_array
                    .bind();
                shader.set_mat4("projection", &system_vars.matrices.projection);
                shader.set_mat4("view", &render_commands.view_matrix);
                shader.set_int("model_texture", 0);
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0);
                }
                for command in &render_commands.sprite_commands {
                    shader.set_vec2(
                        "size",
                        &[
                            command.texture_width as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                            command.texture_height as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                        ],
                    );
                    shader.set_mat4("model", &command.common.matrix);
                    shader.set_vec4("color", &command.common.color);
                    shader.set_vec2("offset", &command.common.offset);

                    unsafe {
                        gl::BindTexture(gl::TEXTURE_2D, command.texture);
                    }
                    vao_bind.draw();
                }
            }

            {
                let shader = system_vars.assets.shaders.sprite_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.projection);
                shader.set_mat4("view", &render_commands.view_matrix);
                shader.set_int("model_texture", 0);
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0);
                }
                system_vars.assets.sprites.numbers.bind(TEXTURE_0);
                for command in &render_commands.number_3d_commands {
                    shader.set_vec2("size", &[command.common.size, command.common.size]);
                    shader.set_mat4("model", &command.common.matrix);
                    shader.set_vec4("color", &command.common.color);
                    shader.set_vec2("offset", &command.common.offset);

                    self.create_number_vertex_array(command.value).bind().draw();
                }
            }
        }
    }
}
