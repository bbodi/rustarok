use crate::asset::database::AssetDatabase;
use crate::asset::texture::TextureId;
use crate::common::{v3, Mat3, Mat4, Vec2};
use crate::components::char::SpriteBoundingRect;
use crate::effect::StrEffectId;
use crate::systems::render::opengl_render_sys::VERTEX_ARRAY_COUNT;
use crate::systems::render_sys::ONE_SPRITE_PIXEL_SIZE_IN_3D;
use nalgebra::{Rotation3, Vector2, Vector3, Vector4};
use specs::prelude::*;
use std::collections::{HashMap, VecDeque};

pub fn create_2d_pos_rot_matrix(pos: &[i16; 2], rotation_rad: f32) -> Mat4 {
    let mut matrix = Mat4::identity();
    matrix.prepend_translation_mut(&v3(pos[0] as f32, pos[1] as f32, 0.0));

    let rotation =
        Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::z()), rotation_rad)
            .to_homogeneous();
    matrix * rotation
}

pub fn create_3d_pos_rot_matrix(pos: &Vector3<f32>, rotation_rad: &(Vector3<f32>, f32)) -> Mat4 {
    let mut matrix = Mat4::identity();
    matrix.prepend_translation_mut(&pos);
    let rotation = Rotation3::from_axis_angle(
        &nalgebra::Unit::new_normalize(rotation_rad.0),
        rotation_rad.1,
    )
    .to_homogeneous();
    return matrix * rotation;
}

pub fn create_3d_rot_matrix(rotation_rad: &(Vector3<f32>, f32)) -> Mat4 {
    let matrix = Mat4::identity();
    let rotation = Rotation3::from_axis_angle(
        &nalgebra::Unit::new_normalize(rotation_rad.0),
        rotation_rad.1,
    )
    .to_homogeneous();
    return matrix * rotation;
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct EffectFrameCacheKey {
    pub effect_id: StrEffectId,
    pub layer_index: usize,
    pub key_index: i32,
}

#[derive(Component)]
pub struct RenderCommandCollector {
    pub(super) partial_circle_2d_commands: Vec<PartialCircle2dRenderCommand>,
    pub(super) texture_2d_commands: Vec<Texture2dRenderCommand>,
    pub(super) trimesh_3d_commands: [Vec<Trimesh3dRenderCommand>; VERTEX_ARRAY_COUNT],
    pub(super) rectangle_3d_commands: Vec<Rectangle3dRenderCommand>,
    pub(super) rectangle_2d_commands: VecDeque<Rectangle2dRenderCommand>,
    pub(super) point_2d_commands: Vec<Point2dRenderCommand>,
    pub(super) text_2d_commands: Vec<Text2dRenderCommand>,
    pub(super) circle_3d_commands: Vec<Circle3dRenderCommand>,
    pub(super) sprite_3d_commands: Vec<Sprite3dRenderCommand>,
    pub(super) horizontal_texture_3d_commands: Vec<HorizontalTexture3dRenderCommand>,
    pub(super) number_3d_commands: Vec<Number3dRenderCommand>,
    pub(super) model_commands: Vec<ModelRenderCommand>,
    pub(super) effect_commands: HashMap<EffectFrameCacheKey, Vec<Vector2<f32>>>,
    pub(super) effect_commands2: Vec<(StrEffectId, i32, Vec2)>,
    pub view_matrix: Mat4,
    pub normal_matrix: Mat3,
    pub yaw: f32,
}

impl<'a> RenderCommandCollector {
    pub fn new() -> RenderCommandCollector {
        RenderCommandCollector {
            partial_circle_2d_commands: Vec::with_capacity(128),
            texture_2d_commands: Vec::with_capacity(128),
            text_2d_commands: Vec::with_capacity(128),
            rectangle_3d_commands: Vec::with_capacity(128),
            trimesh_3d_commands: [
                Vec::with_capacity(128),
                Vec::with_capacity(128),
                Vec::with_capacity(128),
            ],
            rectangle_2d_commands: VecDeque::with_capacity(128),
            point_2d_commands: Vec::with_capacity(128),
            circle_3d_commands: Vec::with_capacity(128),
            sprite_3d_commands: Vec::with_capacity(128),
            horizontal_texture_3d_commands: Vec::with_capacity(128),
            number_3d_commands: Vec::with_capacity(128),
            effect_commands: HashMap::with_capacity(128),
            effect_commands2: Vec::with_capacity(128),
            model_commands: Vec::with_capacity(128),
            view_matrix: Mat4::identity(),
            normal_matrix: Mat3::identity(),
            yaw: 0.0,
        }
    }

    pub fn set_view_matrix(&mut self, view_matrix: &Mat4, normal_matrix: &Mat3, yaw: f32) {
        self.view_matrix = *view_matrix;
        self.normal_matrix = *normal_matrix;
        self.yaw = yaw;
    }

    pub fn clear(&mut self) {
        self.partial_circle_2d_commands.clear();
        self.texture_2d_commands.clear();
        self.text_2d_commands.clear();
        self.rectangle_3d_commands.clear();
        for commands in &mut self.trimesh_3d_commands {
            commands.clear();
        }
        self.rectangle_2d_commands.clear();
        self.point_2d_commands.clear();
        self.circle_3d_commands.clear();
        self.sprite_3d_commands.clear();
        self.horizontal_texture_3d_commands.clear();
        self.number_3d_commands.clear();
        self.effect_commands2.clear();
        self.effect_commands
            .iter_mut()
            .for_each(|(_key, vec)| vec.clear());
        self.model_commands.clear();
    }

    pub fn add_model_command_3d(&'a mut self, model_instance_index: usize, is_transparent: bool) {
        self.model_commands.push(ModelRenderCommand {
            model_instance_index,
            is_transparent,
        });
    }

    pub fn partial_circle_2d(&'a mut self) -> PartialCircl2dBuilder {
        PartialCircl2dBuilder::new(self)
    }

    pub fn circle_3d(&'a mut self) -> Circle3dRenderCommandBuilder {
        Circle3dRenderCommandBuilder::new(self)
    }

    pub fn trimesh3d(&'a mut self) -> Trimesh3dRenderCommandBuilder {
        Trimesh3dRenderCommandBuilder::new(self)
    }

    pub fn rectangle_3d(&'a mut self) -> Rectangle3dRenderCommandBuilder {
        Rectangle3dRenderCommandBuilder::new(self)
    }

    pub fn text_2d(&'a mut self) -> Text2dRenderCommandBuilder {
        Text2dRenderCommandBuilder::new(self)
    }

    pub fn number_3d(&'a mut self) -> Number3dRenderCommandBuilder {
        Number3dRenderCommandBuilder::new(self)
    }

    pub fn horizontal_texture_3d(&'a mut self) -> HorizontalTexture3dRenderCommandBuilder {
        HorizontalTexture3dRenderCommandBuilder::new(self)
    }

    pub fn sprite_3d(&'a mut self) -> Sprite3dRenderCommandBuilder {
        Sprite3dRenderCommandBuilder::new(self)
    }

    pub fn rectangle_2d(&'a mut self) -> Rectangle2dCommandBuilder {
        Rectangle2dCommandBuilder::new(self)
    }

    pub fn point_2d(&'a mut self) -> Point2dCommandBuilder {
        Point2dCommandBuilder::new(self)
    }

    pub fn sprite_2d(&'a mut self) -> Texture2dRenderCommandCommandBuilder {
        Texture2dRenderCommandCommandBuilder::new(self)
    }

    pub fn add_effect_command2(&'a mut self, pos: &Vec2, effect_id: StrEffectId, key_index: i32) {
        self.effect_commands2.push((effect_id, key_index, *pos));
    }

    pub fn add_effect_command(
        &'a mut self,
        pos: &Vec2,
        effect_id: StrEffectId,
        key_index: i32,
        layer_index: usize,
    ) {
        let frame_cache_key = EffectFrameCacheKey {
            effect_id,
            layer_index,
            key_index,
        };
        self.effect_commands
            .entry(frame_cache_key.clone())
            .or_insert(Vec::with_capacity(128))
            .push(*pos);
    }

    pub fn get_last_billboard_command(&'a self) -> Option<&'a Sprite3dRenderCommand> {
        // TODO: remove this code, check the comment at 'let render_command = render_commands.get_last_billboard_command();'
        return self
            .sprite_3d_commands
            .get((self.sprite_3d_commands.len() as i32 - 1).max(0) as usize);
    }
}

pub struct Rectangle2dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) screen_pos: [i16; 2],
    pub(super) rotation_rad: f32,
    pub(super) layer: UiLayer2d,
}

pub struct Rectangle2dCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    screen_pos: [i16; 2],
    rotation_rad: f32,
    width: u16,
    height: u16,
    layer: UiLayer2d,
}

#[allow(dead_code)]
impl<'a> Rectangle2dCommandBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollector) -> Rectangle2dCommandBuilder {
        Rectangle2dCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            screen_pos: [0, 0],
            width: 0,
            height: 0,
            rotation_rad: 0.0,
            layer: UiLayer2d::HealthBars,
        }
    }

    pub fn add(&mut self) {
        if self.color[3] < 255 {
            self.collector
                .rectangle_2d_commands
                .push_front(Rectangle2dRenderCommand {
                    color: self.color,
                    width: self.width,
                    height: self.height,
                    screen_pos: self.screen_pos,
                    layer: self.layer,
                    rotation_rad: self.rotation_rad,
                });
        } else {
            self.collector
                .rectangle_2d_commands
                .push_back(Rectangle2dRenderCommand {
                    color: self.color,
                    width: self.width,
                    height: self.height,
                    screen_pos: self.screen_pos,
                    layer: self.layer,
                    rotation_rad: self.rotation_rad,
                });
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Rectangle2dCommandBuilder {
        self.color = *color;
        self
    }

    pub fn rotation_rad(&mut self, rotation_rad: f32) -> &'a mut Rectangle2dCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Rectangle2dCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn screen_pos(&mut self, x: i32, y: i32) -> &'a mut Rectangle2dCommandBuilder {
        self.screen_pos = [x as i16, y as i16];
        self
    }

    pub fn width(&mut self, w: u16) -> &'a mut Rectangle2dCommandBuilder {
        self.width = w;
        self
    }

    pub fn layer(&mut self, layer: UiLayer2d) -> &'a mut Rectangle2dCommandBuilder {
        self.layer = layer;
        self
    }

    pub fn size(&mut self, w: u16, h: u16) -> &'a mut Rectangle2dCommandBuilder {
        self.width = w;
        self.height = h;
        self
    }

    pub fn height(&mut self, h: u16) -> &'a mut Rectangle2dCommandBuilder {
        self.height = h;
        self
    }
}

pub struct Point2dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) screen_pos: [i16; 2],
    pub(super) layer: UiLayer2d,
}

pub struct Point2dCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    screen_pos: [i16; 2],
    layer: UiLayer2d,
}

#[allow(dead_code)]
impl<'a> Point2dCommandBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollector) -> Point2dCommandBuilder {
        Point2dCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            screen_pos: [0, 0],
            layer: UiLayer2d::HealthBars,
        }
    }

    pub fn add(&mut self) {
        self.collector.point_2d_commands.push(Point2dRenderCommand {
            color: self.color,
            screen_pos: self.screen_pos,
            layer: self.layer,
        });
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Point2dCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Point2dCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn screen_pos(&mut self, x: i32, y: i32) -> &'a mut Point2dCommandBuilder {
        self.screen_pos = [x as i16, y as i16];
        self
    }

    pub fn layer(&mut self, layer: UiLayer2d) -> &'a mut Point2dCommandBuilder {
        self.layer = layer;
        self
    }
}

pub struct PartialCircle2dRenderCommand {
    pub(super) circumference_index: usize,
    pub(super) color: [u8; 4],
    pub(super) screen_pos: [i16; 2],
    pub(super) layer: UiLayer2d,
}

pub struct PartialCircl2dBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    screen_pos: [i16; 2],
    circumference_percentage: usize,
    layer: UiLayer2d,
}

#[allow(dead_code)]
impl<'a> PartialCircl2dBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollector) -> PartialCircl2dBuilder {
        PartialCircl2dBuilder {
            collector,
            color: [255, 255, 255, 255],
            screen_pos: [0, 0],
            layer: UiLayer2d::HealthBars,
            circumference_percentage: 100,
        }
    }

    pub fn add(&mut self) {
        self.collector
            .partial_circle_2d_commands
            .push(PartialCircle2dRenderCommand {
                circumference_index: self.circumference_percentage - 1,
                color: self.color,
                screen_pos: self.screen_pos,
                layer: self.layer,
            });
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut PartialCircl2dBuilder {
        self.color = *color;
        self
    }

    pub fn circumference_percentage(
        &mut self,
        circumference_percentage: usize,
    ) -> &'a mut PartialCircl2dBuilder {
        self.circumference_percentage = circumference_percentage;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut PartialCircl2dBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn screen_pos(&mut self, x: i32, y: i32) -> &'a mut PartialCircl2dBuilder {
        self.screen_pos = [x as i16, y as i16];
        self
    }

    pub fn layer(&mut self, layer: UiLayer2d) -> &'a mut PartialCircl2dBuilder {
        self.layer = layer;
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum UiLayer2d {
    HealthBars,
    StatusIndicators,
    SelfCastingBar,
    SkillBar,
    SkillBarIcon,
    SkillBarKey,
    HoveringSkillBarName,
    Minimap,
    MinimapSimpleEntities,
    MinimapImportantEntities,
    MinimapVisibleRegionRectangle,
    SelectingTargetSkillName,
    Console,
    ConsoleTexts,
    ConsoleAutocompletion,
    Cursor,
}

pub struct Texture2dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) offset: [i16; 2],
    pub(super) screen_pos: [i16; 2],
    pub(super) rotation_rad: f32,
    pub(super) scale: f32,
    pub(super) texture: TextureId,
    pub(super) layer: UiLayer2d,
}

pub struct Texture2dRenderCommandCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    screen_pos: [i16; 2],
    offset: [i16; 2],
    rotation_rad: f32,
    layer: UiLayer2d,
    scale: f32,
    flip_vertically: bool,
}

#[allow(dead_code)]
impl<'a> Texture2dRenderCommandCommandBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollector) -> Texture2dRenderCommandCommandBuilder {
        Texture2dRenderCommandCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            screen_pos: [0, 0],
            offset: [0, 0],
            rotation_rad: 0.0,
            scale: 1.0,
            layer: UiLayer2d::HealthBars,
            flip_vertically: false,
        }
    }

    pub fn add(&mut self, texture: TextureId) {
        self.collector
            .texture_2d_commands
            .push(Texture2dRenderCommand {
                color: self.color,
                scale: if self.flip_vertically {
                    -self.scale
                } else {
                    self.scale
                },
                texture,
                offset: self.offset,
                //                texture_width: (1 - self.flip_vertically as i32 * 2) * texture.width,
                //                texture_height: texture.height,
                screen_pos: self.screen_pos,
                rotation_rad: self.rotation_rad,
                layer: self.layer,
            });
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.color = *color;
        self
    }

    pub fn flip_vertically(&mut self, flip: bool) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.flip_vertically = flip;
        self
    }

    pub fn rotation_rad(
        &mut self,
        rotation_rad: f32,
    ) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn screen_pos(&mut self, x: i32, y: i32) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.screen_pos = [x as i16, y as i16];
        self
    }

    pub fn offset(&mut self, x: i16, y: i16) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.offset = [x, y];
        self
    }

    pub fn layer(&mut self, layer: UiLayer2d) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.layer = layer;
        self
    }

    pub fn scale(&mut self, scale: f32) -> &'a mut Texture2dRenderCommandCommandBuilder {
        self.scale = scale;
        self
    }
}

pub struct Text2dRenderCommand {
    pub(super) text: String,
    pub(super) color: [u8; 4],
    pub(super) screen_pos: [i16; 2],
    pub(super) font: Font,
    pub(super) outline: bool,
    pub(super) layer: UiLayer2d,
}

pub struct Text2dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    screen_pos: [i16; 2],
    layer: UiLayer2d,
    font: Font,
    outline: bool,
}

#[allow(dead_code)]
impl<'a> Text2dRenderCommandBuilder<'a> {
    pub fn new(collector: &mut RenderCommandCollector) -> Text2dRenderCommandBuilder {
        Text2dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            font: Font::Normal,
            screen_pos: [0, 0],
            outline: false,
            layer: UiLayer2d::HealthBars,
        }
    }

    pub fn add(&mut self, text: &str) {
        self.collector.text_2d_commands.push(Text2dRenderCommand {
            text: text.to_owned(),
            color: self.color,
            screen_pos: self.screen_pos,
            font: self.font,
            outline: self.outline,
            layer: self.layer,
        })
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Text2dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Text2dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn screen_pos(&mut self, x: i32, y: i32) -> &'a mut Text2dRenderCommandBuilder {
        self.screen_pos = [x as i16, y as i16];
        self
    }

    pub fn layer(&mut self, layer: UiLayer2d) -> &'a mut Text2dRenderCommandBuilder {
        self.layer = layer;
        self
    }

    pub fn font(&mut self, font: Font) -> &'a mut Text2dRenderCommandBuilder {
        self.font = font;
        self
    }

    pub fn outline(&mut self, outline: bool) -> &'a mut Text2dRenderCommandBuilder {
        self.outline = outline;
        self
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Font {
    Small,
    SmallBold,
    Normal,
    NormalBold,
    Big,
    BigBold,
}

#[allow(dead_code)]
pub enum Trimesh3dType {
    Sanctuary,
    Cylinder,
    Sphere,
}

pub struct Trimesh3dRenderCommand {
    pub(super) pos: Vector3<f32>,
    pub(super) color: [u8; 4],
    pub(super) scale: f32,
    pub(super) texture: Option<TextureId>,
    pub(super) rotation_rad: f32,
}

pub struct Trimesh3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    pos: Vector3<f32>,
    color: [u8; 4],
    texture: Option<TextureId>,
    scale: f32,
    rotation_rad: f32,
}

#[allow(dead_code)]
impl<'a> Trimesh3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> Trimesh3dRenderCommandBuilder {
        Trimesh3dRenderCommandBuilder {
            collector,
            pos: Vector3::zeros(),
            color: [255, 255, 255, 255],
            texture: None,
            scale: 1.0,
            rotation_rad: 0.0,
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn texture(&mut self, texture_id: TextureId) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.texture = Some(texture_id);
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn scale(&mut self, scale: f32) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.scale = scale;
        self
    }

    pub fn rotation_rad(&mut self, rotation_rad: f32) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vec2) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Trimesh3dRenderCommandBuilder {
        self.pos.y = y;
        self
    }

    pub fn add(&'a mut self, typ: Trimesh3dType) {
        self.collector.trimesh_3d_commands[typ as usize].push(Trimesh3dRenderCommand {
            pos: self.pos,
            color: self.color,
            scale: self.scale,
            texture: self.texture,
            rotation_rad: self.rotation_rad,
        });
    }
}

pub struct Rectangle3dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) pos: Vector3<f32>,
    pub(super) rotation_rad: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

pub struct Rectangle3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    pos: Vector3<f32>,
    rotation_rad: f32,
    width: f32,
    height: f32,
}

#[allow(dead_code)]
impl<'a> Rectangle3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> Rectangle3dRenderCommandBuilder {
        Rectangle3dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            pos: Vector3::zeros(),
            rotation_rad: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: u8) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.color[3] = a;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vec2) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.pos.y = y;
        self
    }

    pub fn size(&mut self, w: f32, h: f32) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.width = w;
        self.height = h;
        self
    }

    pub fn rotation_rad(
        &'a mut self,
        rotation_rad: f32,
    ) -> &'a mut Rectangle3dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }

    pub fn add(&'a mut self) {
        self.collector
            .rectangle_3d_commands
            .push(Rectangle3dRenderCommand {
                color: self.color,
                pos: self.pos,
                width: self.width,
                height: self.height,
                rotation_rad: self.rotation_rad,
            });
    }
}

pub struct Circle3dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) pos: Vector3<f32>,
    pub(super) radius: f32,
}

pub struct Circle3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    pos: Vector3<f32>,
    radius: f32,
}

#[allow(dead_code)]
impl<'a> Circle3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> Circle3dRenderCommandBuilder {
        Circle3dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            pos: Vector3::zeros(),
            radius: 1.0,
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Circle3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Circle3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: u8) -> &'a mut Circle3dRenderCommandBuilder {
        self.color[3] = a;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vec2) -> &'a mut Circle3dRenderCommandBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Circle3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Circle3dRenderCommandBuilder {
        self.pos.y = y;
        self
    }

    pub fn radius(&'a mut self, radius: f32) -> &'a mut Circle3dRenderCommandBuilder {
        self.radius = radius;
        self
    }

    pub fn add(&'a mut self) {
        self.collector
            .circle_3d_commands
            .push(Circle3dRenderCommand {
                color: self.color,
                pos: self.pos,
                radius: self.radius,
            });
    }
}

pub struct HorizontalTexture3dRenderCommand {
    pub color: [u8; 4],
    pub size: TextureSizeSetting,
    pub pos: Vec2,
    pub rotation_rad: f32,
    pub texture_id: TextureId,
}

#[derive(Clone, Copy)]
pub enum TextureSizeSetting {
    Scale(f32),
    FixSize(f32),
}

pub struct HorizontalTexture3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    pos: Vec2,
    size: TextureSizeSetting,
    rotation_rad: f32,
}

#[allow(dead_code)]
impl<'a> HorizontalTexture3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> HorizontalTexture3dRenderCommandBuilder {
        HorizontalTexture3dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            pos: Vector2::zeros(),
            size: TextureSizeSetting::Scale(1.0),
            rotation_rad: 0.0,
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(
        &mut self,
        color: &[u8; 3],
    ) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: u8) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.color[3] = a;
        self
    }

    pub fn rotation_rad(
        &mut self,
        rotation_rad: f32,
    ) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.rotation_rad = rotation_rad;
        self
    }

    pub fn pos(&mut self, pos: &Vec2) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn scale(&'a mut self, scale: f32) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.size = TextureSizeSetting::Scale(scale);
        self
    }

    pub fn fix_size(&'a mut self, size: f32) -> &'a mut HorizontalTexture3dRenderCommandBuilder {
        self.size = TextureSizeSetting::FixSize(size);
        self
    }

    pub fn add(&'a mut self, texture_id: TextureId) {
        let command = HorizontalTexture3dRenderCommand {
            color: self.color,
            size: self.size,
            texture_id,
            pos: self.pos,
            rotation_rad: self.rotation_rad,
        };
        self.collector.horizontal_texture_3d_commands.push(command);
    }
}

pub struct Sprite3dRenderCommand {
    pub color: [u8; 4],
    pub scale: f32,
    pub pos: Vector3<f32>,
    pub rot_radian: f32,
    pub texture_id: TextureId,
    pub offset: [i16; 2],
    //    pub texture_width: u16,
    //    pub texture_height: u16,
    pub is_vertically_flipped: bool,
}

impl Sprite3dRenderCommand {
    pub fn project_to_screen(
        &self,
        view: &Mat4,
        projection: &Mat4,
        asset_db: &AssetDatabase,
        resolution_w: u32,
        resolution_h: u32,
    ) -> SpriteBoundingRect {
        let texture = asset_db.get_texture(self.texture_id);
        let width = texture.width as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D * self.scale;
        let height = texture.height as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D * self.scale;
        let offset_in_3d_space = [
            self.offset[0] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D * self.scale,
            self.offset[1] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D * self.scale,
        ];
        let mut top_right = Vector4::new(0.5 * width, 0.5 * height, 0.0, 1.0);
        top_right.x += offset_in_3d_space[0];
        top_right.y -= offset_in_3d_space[1];

        let mut bottom_left = Vector4::new(-0.5 * width, -0.5 * height, 0.0, 1.0);
        bottom_left.x += offset_in_3d_space[0];
        bottom_left.y -= offset_in_3d_space[1];
        if bottom_left.y.is_nan() || self.pos.y.is_nan() {
            dbg!(bottom_left);
            dbg!(width);
            dbg!(height);
            dbg!(self.offset);
            dbg!(self.scale);
            dbg!(self.pos);
        }

        let mut model_view = view * Mat4::new_translation(&self.pos);
        Sprite3dRenderCommand::set_spherical_billboard(&mut model_view);
        fn sh(v: Vector4<f32>, resolution_w: u32, resolution_h: u32) -> [i32; 2] {
            let s = if v[3] == 0.0 { 1.0 } else { 1.0 / v[3] };
            let s2 = (v[1] * s / 2.0 + 0.5) * resolution_h as f32;
            if s2.is_nan() {
                dbg!(v[1]);
                dbg!(v);
                dbg!(s);
            }
            [
                ((v[0] * s / 2.0 + 0.5) * resolution_w as f32) as i32,
                resolution_h as i32 - s2 as i32,
            ]
        }
        let bottom_left = sh(
            projection * model_view * bottom_left,
            resolution_w,
            resolution_h,
        );
        let top_right = sh(
            projection * model_view * top_right,
            resolution_w,
            resolution_h,
        );
        return SpriteBoundingRect {
            bottom_left,
            top_right,
        };
    }

    fn set_spherical_billboard(model_view: &mut Mat4) {
        model_view[0] = 1.0;
        model_view[1] = 0.0;
        model_view[2] = 0.0;
        model_view[4] = 0.0;
        model_view[5] = 1.0;
        model_view[6] = 0.0;
        model_view[8] = 0.0;
        model_view[9] = 0.0;
        model_view[10] = 1.0;
    }
}

pub struct Sprite3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    pos: Vector3<f32>,
    rot_radian: f32,
    offset: [i16; 2],
    scale: f32,
    flip_vertically: bool,
}

#[allow(dead_code)]
impl<'a> Sprite3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> Sprite3dRenderCommandBuilder {
        Sprite3dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            pos: Vector3::zeros(),
            offset: [0, 0],
            scale: 1.0,
            flip_vertically: false,
            rot_radian: 0.0,
        }
    }

    pub fn rot_radian(&mut self, rot_radian: f32) -> &'a mut Sprite3dRenderCommandBuilder {
        self.rot_radian = rot_radian;
        self
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Sprite3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn flip_vertically(&mut self, flip: bool) -> &'a mut Sprite3dRenderCommandBuilder {
        self.flip_vertically = flip;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Sprite3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: u8) -> &'a mut Sprite3dRenderCommandBuilder {
        self.color[3] = a;
        self
    }

    pub fn offset(&mut self, offset: [i16; 2]) -> &'a mut Sprite3dRenderCommandBuilder {
        self.offset = offset;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vec2) -> &'a mut Sprite3dRenderCommandBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Sprite3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Sprite3dRenderCommandBuilder {
        self.pos.y = y;
        self
    }

    pub fn scale(&'a mut self, scale: f32) -> &'a mut Sprite3dRenderCommandBuilder {
        self.scale = scale;
        self
    }

    pub fn add(&'a mut self, texture: TextureId) {
        let command = Sprite3dRenderCommand {
            color: self.color,
            scale: self.scale,
            offset: self.offset,
            texture_id: texture,
            is_vertically_flipped: self.flip_vertically,
            pos: self.pos,
            rot_radian: self.rot_radian,
        };
        self.collector.sprite_3d_commands.push(command);
    }
}

pub struct Number3dRenderCommand {
    pub(super) color: [u8; 4],
    pub(super) scale: f32,
    pub(super) pos: Vector3<f32>,
    pub(super) value: u32,
}

pub struct Number3dRenderCommandBuilder<'a> {
    collector: &'a mut RenderCommandCollector,
    color: [u8; 4],
    pos: Vector3<f32>,
    offset: [i16; 2],
    scale: f32,
}

#[allow(dead_code)]
impl<'a> Number3dRenderCommandBuilder<'a> {
    fn new(collector: &mut RenderCommandCollector) -> Number3dRenderCommandBuilder {
        Number3dRenderCommandBuilder {
            collector,
            color: [255, 255, 255, 255],
            pos: Vector3::zeros(),
            offset: [0, 0],
            scale: 1.0,
        }
    }

    pub fn color(&mut self, color: &[u8; 4]) -> &'a mut Number3dRenderCommandBuilder {
        self.color = *color;
        self
    }

    pub fn color_rgb(&mut self, color: &[u8; 3]) -> &'a mut Number3dRenderCommandBuilder {
        self.color[0] = color[0];
        self.color[1] = color[1];
        self.color[2] = color[2];
        self
    }

    pub fn alpha(&mut self, a: u8) -> &'a mut Number3dRenderCommandBuilder {
        self.color[3] = a;
        self
    }

    pub fn offset(&mut self, offset: [i16; 2]) -> &'a mut Number3dRenderCommandBuilder {
        self.offset = offset;
        self
    }

    pub fn pos_2d(&mut self, pos: &Vec2) -> &'a mut Number3dRenderCommandBuilder {
        self.pos.x = pos.x;
        self.pos.z = pos.y;
        self
    }

    pub fn pos(&mut self, pos: &Vector3<f32>) -> &'a mut Number3dRenderCommandBuilder {
        self.pos = *pos;
        self
    }

    pub fn y(&mut self, y: f32) -> &'a mut Number3dRenderCommandBuilder {
        self.pos.y = y;
        self
    }

    pub fn scale(&'a mut self, scale: f32) -> &'a mut Number3dRenderCommandBuilder {
        self.scale = scale;
        self
    }

    pub fn add(&'a mut self, value: u32) {
        self.collector
            .number_3d_commands
            .push(Number3dRenderCommand {
                color: self.color,
                scale: self.scale,
                pos: self.pos,
                value,
            });
    }
}

pub struct ModelRenderCommand {
    pub(super) is_transparent: bool,
    pub(super) model_instance_index: usize,
}
