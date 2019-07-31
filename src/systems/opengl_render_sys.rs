use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::video::VertexArray;
use nalgebra::{Matrix4, Rotation3, Vector3};
use specs::prelude::*;

#[derive(Component)]
pub struct RenderCommandCollectorComponent {
    pub trimesh_2d_commands: Vec<Trimesh2dRenderCommand>,
}

impl<'a> RenderCommandCollectorComponent {
    pub fn new() -> RenderCommandCollectorComponent {
        RenderCommandCollectorComponent {
            trimesh_2d_commands: Vec::with_capacity(128),
        }
    }

    pub fn clear(&mut self) {
        self.trimesh_2d_commands.clear();
    }

    pub fn trimesh_2d(&'a mut self, vao: &'a VertexArray) -> Trimesh2dRenderCommandBuilder {
        Trimesh2dRenderCommandBuilder {
            collector: self,
            vao,
            color: [1.0, 1.0, 1.0, 1.0],
            screen_pos: [0.0, 0.0],
            size: [1.0, 1.0],
            rotation_rad: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct Trimesh2dRenderCommand {
    pub vao: VertexArray,
    pub color: [f32; 4],
    pub size: [f32; 2],
    pub matrix: Matrix4<f32>,
}

pub struct Trimesh2dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollectorComponent,
    vao: &'a VertexArray,
    color: [f32; 4],
    screen_pos: [f32; 2],
    size: [f32; 2],
    rotation_rad: f32,
}

impl<'a> Trimesh2dRenderCommandBuilder<'a> {
    pub fn add(&mut self) {
        self.collector
            .trimesh_2d_commands
            .push(Trimesh2dRenderCommand {
                vao: self.vao.clone(),
                color: self.color,
                size: self.size,
                matrix: {
                    let mut matrix = Matrix4::<f32>::identity();
                    matrix.prepend_translation_mut(&v3!(self.screen_pos[0], self.screen_pos[1], 0));

                    let rotation = Rotation3::from_axis_angle(
                        &nalgebra::Unit::new_normalize(Vector3::z()),
                        self.rotation_rad,
                    )
                    .to_homogeneous();
                    matrix * rotation
                },
            });
    }

    pub fn color(&'a mut self, color: &[f32; 4]) -> &'a mut Trimesh2dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn screen_pos(&'a mut self, x: f32, y: f32) -> &'a mut Trimesh2dRenderCommandBuilder {
        self.screen_pos = [x, y];
        self
    }

    pub fn rotation_rad(&'a mut self, rotation_rad: f32) -> &'a mut Trimesh2dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }
}

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
                trimesh_2d.vao.bind().draw();
            }
        }
    }
}
