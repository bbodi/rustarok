use crate::video::{GlTexture, VertexArray};
use nalgebra::{Matrix4, Rotation3, Vector3};
use specs::prelude::*;

#[derive(Component)]
pub struct RenderCommandCollectorComponent {
    pub(super) trimesh_2d_commands: Vec<Trimesh2dRenderCommand>,
    pub(super) texture_2d_commands: Vec<TopLeftTexture2dRenderCommand>,
}

impl<'a> RenderCommandCollectorComponent {
    pub fn new() -> RenderCommandCollectorComponent {
        RenderCommandCollectorComponent {
            trimesh_2d_commands: Vec::with_capacity(128),
            texture_2d_commands: Vec::with_capacity(128),
        }
    }

    pub fn clear(&mut self) {
        self.trimesh_2d_commands.clear();
        self.texture_2d_commands.clear();
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

    pub fn top_left_texture_2d(
        &'a mut self,
        texture: &GlTexture,
    ) -> TopLeftTexture2dRenderCommandBuilder {
        TopLeftTexture2dRenderCommandBuilder {
            collector: self,
            color: [1.0, 1.0, 1.0, 1.0],
            screen_pos: [0.0, 0.0],
            offset: [0.0, 0.0],
            size: 1.0,
            rotation_rad: 0.0,
            texture: texture.id(),
            texture_width: texture.width,
            texture_height: texture.height,
        }
    }
}

#[derive(Debug)]
pub struct Trimesh2dRenderCommand {
    pub(super) vao: VertexArray,
    pub(super) color: [f32; 4],
    pub(super) size: [f32; 2],
    pub(super) matrix: Matrix4<f32>,
    pub(super) layer: Layer2d,
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
    pub fn add(&mut self, layer: Layer2d) {
        self.collector
            .trimesh_2d_commands
            .push(Trimesh2dRenderCommand {
                vao: self.vao.clone(),
                color: self.color,
                size: self.size,
                matrix: create_2d_matrix(&self.screen_pos, self.rotation_rad),
                layer,
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

    pub fn size(&'a mut self, x: f32, y: f32) -> &'a mut Trimesh2dRenderCommandBuilder {
        self.size = [x, y];
        self
    }

    pub fn rotation_rad(&'a mut self, rotation_rad: f32) -> &'a mut Trimesh2dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Layer2d {
    Layer0,
    Layer1,
    Layer2,
    Layer3,
    Layer4,
    Layer5,
    Layer6,
    Layer7,
    Layer8,
    Layer9,
}

#[derive(Debug)]
pub struct TopLeftTexture2dRenderCommand {
    pub(super) color: [f32; 4],
    pub(super) offset: [f32; 2],
    pub(super) size: f32,
    pub(super) matrix: Matrix4<f32>,
    pub(super) texture: gl::types::GLuint,
    pub(super) texture_width: i32,
    pub(super) texture_height: i32,
    pub(super) layer: Layer2d,
}

pub struct TopLeftTexture2dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollectorComponent,
    color: [f32; 4],
    screen_pos: [f32; 2],
    offset: [f32; 2],
    size: f32,
    rotation_rad: f32,
    texture: gl::types::GLuint,
    texture_width: i32,
    texture_height: i32,
}

pub fn create_2d_matrix(pos: &[f32; 2], rotation_rad: f32) -> Matrix4<f32> {
    let mut matrix = Matrix4::<f32>::identity();
    matrix.prepend_translation_mut(&v3!(pos[0], pos[1], 0));

    let rotation =
        Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::z()), rotation_rad)
            .to_homogeneous();
    matrix * rotation
}

impl<'a> TopLeftTexture2dRenderCommandBuilder<'a> {
    pub fn add(&mut self, layer: Layer2d) {
        self.collector
            .texture_2d_commands
            .push(TopLeftTexture2dRenderCommand {
                color: self.color,
                size: self.size,
                texture: self.texture,
                offset: self.offset,
                texture_width: self.texture_width,
                texture_height: self.texture_height,
                matrix: create_2d_matrix(&self.screen_pos, self.rotation_rad),
                layer,
            });
    }

    pub fn color(&'a mut self, color: &[f32; 4]) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn offset(&'a mut self, offset: [f32; 2]) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.offset = offset;
        self
    }

    pub fn flip_vertically(
        &'a mut self,
        flip: bool,
    ) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.texture_width = (1 - flip as i32 * 2) * self.texture_width;
        self
    }

    pub fn screen_pos(
        &'a mut self,
        x: f32,
        y: f32,
    ) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.screen_pos = [x, y];
        self
    }

    pub fn size(&'a mut self, size: f32) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.size = size;
        self
    }

    pub fn rotation_rad(
        &'a mut self,
        rotation_rad: f32,
    ) -> &'a mut TopLeftTexture2dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }
}
