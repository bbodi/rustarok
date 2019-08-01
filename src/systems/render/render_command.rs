use crate::video::{GlTexture, VertexArray};
use nalgebra::{Matrix4, Rotation3, Vector2, Vector3};
use specs::prelude::*;

fn create_2d_matrix(pos: &[f32; 2], rotation_rad: f32) -> Matrix4<f32> {
    let mut matrix = Matrix4::<f32>::identity();
    matrix.prepend_translation_mut(&v3!(pos[0], pos[1], 0));

    let rotation =
        Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::z()), rotation_rad)
            .to_homogeneous();
    matrix * rotation
}

fn create_3d_matrix(pos: &Vector3<f32>, rotation_rad: &(Vector3<f32>, f32)) -> Matrix4<f32> {
    let mut matrix = Matrix4::<f32>::identity();
    matrix.prepend_translation_mut(&pos);
    let rotation = Rotation3::from_axis_angle(
        &nalgebra::Unit::new_normalize(rotation_rad.0),
        rotation_rad.1,
    )
    .to_homogeneous();
    return matrix * rotation;
}

#[derive(Component)]
pub struct RenderCommandCollectorComponent {
    pub(super) trimesh_2d_commands: Vec<Trimesh2dRenderCommand>,
    pub(super) texture_2d_commands: Vec<Texture2dRenderCommand>,
    pub(super) rectangle_3d_commands: Vec<Rectangle3dRenderCommand>,
    pub(super) rectangle_2d_commands: Vec<Common2DProperties>,
    pub(super) circle_3d_commands: Vec<Circle3dRenderCommand>,
    pub(super) sprite_commands: Vec<SpriteRenderCommand>,
    pub(super) number_3d_commands: Vec<Number3dRenderCommand>,
    pub(super) view_matrix: Matrix4<f32>,
}

impl<'a> RenderCommandCollectorComponent {
    pub fn new() -> RenderCommandCollectorComponent {
        RenderCommandCollectorComponent {
            trimesh_2d_commands: Vec::with_capacity(128),
            texture_2d_commands: Vec::with_capacity(128),
            rectangle_3d_commands: Vec::with_capacity(128),
            rectangle_2d_commands: Vec::with_capacity(128),
            circle_3d_commands: Vec::with_capacity(128),
            sprite_commands: Vec::with_capacity(128),
            number_3d_commands: Vec::with_capacity(128),
            view_matrix: Matrix4::identity(),
        }
    }

    pub fn set_view_matrix(&mut self, view_matrix: &Matrix4<f32>) {
        self.view_matrix = *view_matrix;
    }

    pub fn clear(&mut self) {
        self.trimesh_2d_commands.clear();
        self.texture_2d_commands.clear();
        self.rectangle_3d_commands.clear();
        self.rectangle_2d_commands.clear();
        self.circle_3d_commands.clear();
        self.sprite_commands.clear();
        self.number_3d_commands.clear();
    }

    pub fn prepare_for_3d(&'a mut self) -> Common3DPropBuilder {
        Common3DPropBuilder::new(self)
    }

    pub fn prepare_for_2d(&'a mut self) -> Common2DPropBuilder {
        Common2DPropBuilder::new(self)
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

pub struct Common2DProperties {
    pub(super) color: [f32; 4],
    pub(super) size: [f32; 2],
    pub(super) matrix: Matrix4<f32>,
    pub(super) layer: Layer2d,
}

pub struct Common2DPropBuilder<'a> {
    collector: &'a mut RenderCommandCollectorComponent,
    color: [f32; 4],
    screen_pos: [f32; 2],
    size: [f32; 2],
    rotation_rad: f32,
}

impl<'a> Common2DPropBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollectorComponent) -> Common2DPropBuilder {
        Common2DPropBuilder {
            collector,
            color: [1.0, 1.0, 1.0, 1.0],
            screen_pos: [0.0, 0.0],
            size: [1.0, 1.0],
            rotation_rad: 0.0,
        }
    }

    pub fn add_trimesh_command(&'a mut self, vao: &'a VertexArray, layer: Layer2d) {
        self.collector
            .trimesh_2d_commands
            .push(Trimesh2dRenderCommand {
                vao: vao.clone(),
                color: self.color,
                size: self.size,
                matrix: create_2d_matrix(&self.screen_pos, self.rotation_rad),
                layer,
            });
    }

    pub fn add_rectangle_command(&'a mut self, layer: Layer2d) {
        self.collector
            .rectangle_2d_commands
            .push(Common2DProperties {
                color: self.color,
                size: self.size,
                matrix: create_2d_matrix(&self.screen_pos, self.rotation_rad),
                layer,
            });
    }

    pub fn add_texture_command(&'a mut self, texture: &GlTexture, layer: Layer2d) {
        self.add_sprite_command(texture, [0.0, 0.0], false, layer);
    }

    pub fn add_sprite_command(
        &'a mut self,
        texture: &GlTexture,
        offset: [f32; 2],
        flip_vertically: bool,
        layer: Layer2d,
    ) {
        self.collector
            .texture_2d_commands
            .push(Texture2dRenderCommand {
                color: self.color,
                size: self.size[0],
                texture: texture.id(),
                offset,
                texture_width: (1 - flip_vertically as i32 * 2) * texture.width,
                texture_height: texture.height,
                matrix: create_2d_matrix(&self.screen_pos, self.rotation_rad),
                layer,
            });
    }

    pub fn color(&'a mut self, color: &[f32; 4]) -> &'a mut Common2DPropBuilder {
        self.color = *color;
        self
    }

    pub fn screen_pos(&'a mut self, x: f32, y: f32) -> &'a mut Common2DPropBuilder {
        self.screen_pos = [x, y];
        self
    }

    pub fn size2(&'a mut self, x: f32, y: f32) -> &'a mut Common2DPropBuilder {
        self.size = [x, y];
        self
    }

    pub fn size(&'a mut self, size: f32) -> &'a mut Common2DPropBuilder {
        self.size = [size, size];
        self
    }

    pub fn rotation_rad(&'a mut self, rotation_rad: f32) -> &'a mut Common2DPropBuilder {
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
pub struct Texture2dRenderCommand {
    pub(super) color: [f32; 4],
    pub(super) offset: [f32; 2],
    pub(super) size: f32,
    pub(super) matrix: Matrix4<f32>,
    pub(super) texture: gl::types::GLuint,
    pub(super) texture_width: i32,
    pub(super) texture_height: i32,
    pub(super) layer: Layer2d,
}

#[derive(Debug)]
pub struct Rectangle3dRenderCommand {
    pub(super) common: Common3DProperties,
    pub(super) size: [f32; 2],
}

#[derive(Debug)]
pub struct Circle3dRenderCommand {
    pub(super) common: Common3DProperties,
}

#[derive(Debug)]
pub struct SpriteRenderCommand {
    pub(super) common: Common3DProperties,
    pub(super) texture: gl::types::GLuint,
    pub(super) texture_width: i32,
    pub(super) texture_height: i32,
}

#[derive(Debug)]
pub struct Common3DProperties {
    pub(super) color: [f32; 4],
    pub(super) offset: [f32; 2],
    pub(super) size: f32,
    pub(super) matrix: Matrix4<f32>,
}

pub struct Common3DPropBuilder<'a> {
    collector: &'a mut RenderCommandCollectorComponent,
    color: [f32; 4],
    pos: Vector3<f32>,
    offset: [f32; 2],
    size: f32,
    rotation_rad: (Vector3<f32>, f32),
}

impl<'a> Common3DPropBuilder<'a> {
    fn new(collector: &mut RenderCommandCollectorComponent) -> Common3DPropBuilder {
        Common3DPropBuilder {
            collector,
            color: [1.0, 1.0, 1.0, 1.0],
            pos: Vector3::zeros(),
            offset: [0.0, 0.0],
            size: 1.0,
            rotation_rad: (Vector3::zeros(), 0.0),
        }
    }

    pub fn color(&mut self, color: &[f32; 4]) -> &'a mut Common3DPropBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[f32; 3]) -> &'a mut Common3DPropBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: f32) -> &'a mut Common3DPropBuilder {
        self.color[3] = a;
        self
    }

    pub fn offset(&mut self, offset: [f32; 2]) -> &'a mut Common3DPropBuilder {
        self.offset = offset;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vector2<f32>) -> &'a mut Common3DPropBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Common3DPropBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Common3DPropBuilder {
        self.pos.y = y;
        self
    }

    pub fn size(&'a mut self, size: f32) -> &'a mut Common3DPropBuilder {
        self.size = size;
        self
    }

    pub fn radius(&'a mut self, size: f32) -> &'a mut Common3DPropBuilder {
        self.size = size;
        self
    }

    pub fn rotation_rad(&mut self, axis: Vector3<f32>, angle: f32) -> &'a mut Common3DPropBuilder {
        self.rotation_rad = (axis, angle);
        self
    }

    fn build(&self) -> Common3DProperties {
        Common3DProperties {
            color: self.color,
            size: self.size,
            offset: self.offset,
            matrix: create_3d_matrix(&self.pos, &self.rotation_rad),
        }
    }

    pub fn add_number_command(&'a mut self, value: u32, digit_count: u8) {
        self.collector
            .number_3d_commands
            .push(Number3dRenderCommand {
                common: self.build(),
                value,
                digit_count,
            });
    }

    pub fn add_sprite_command(&'a mut self, texture: &GlTexture, flip_vertically: bool) {
        self.collector.sprite_commands.push(SpriteRenderCommand {
            common: self.build(),
            texture: texture.id(),
            texture_width: (1 - flip_vertically as i32 * 2) * texture.width,
            texture_height: texture.height,
        });
    }

    pub fn add_circle_command(&'a mut self) {
        self.collector
            .circle_3d_commands
            .push(Circle3dRenderCommand {
                common: self.build(),
            });
    }

    pub fn add_rectangle_command(&'a mut self, size: &Vector2<f32>) {
        self.collector
            .rectangle_3d_commands
            .push(Rectangle3dRenderCommand {
                common: self.build(),
                size: [size.x, size.y],
            });
    }
}

#[derive(Debug)]
pub struct Number3dRenderCommand {
    pub(super) common: Common3DProperties,
    pub(super) value: u32,
    pub(super) digit_count: u8,
}
