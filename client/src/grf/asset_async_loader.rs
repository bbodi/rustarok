use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};

use crate::strum::IntoEnumIterator;
use encoding::types::Encoding;
use encoding::DecoderTrap;
use std::fmt::Write;

use crate::components::char::CharActionIndex;
use crate::consts::{job_name_table, PLAYABLE_CHAR_SPRITES};
use crate::grf::act::ActionFile;
use crate::grf::asset_loader::GrfEntryLoader;
use crate::grf::gnd::{Gnd, MeshVertex};
use crate::grf::rsm::{BoundingBox, Rsm};
use crate::grf::rsw::RswModelInstance;
use crate::grf::spr::SpriteFile;
use crate::grf::texture::TextureId;
use crate::grf::SpriteResource;
use crate::my_gl::MyGlEnum;
use crate::runtime_assets::map::{ModelInstance, SameTextureNodeFacesRaw};
use crate::systems::{EffectSprites, Sprites};
use nalgebra::{Point3, Rotation3};
use rustarok_common::common::{measure_time, v3, Mat4, Vec2, Vec3};
use rustarok_common::components::char::{JobId, MonsterId};
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::grf::asset_loader::CommonAssetLoader;
use rustarok_common::grf::binary_reader::BinaryReader;
use rustarok_common::grf::gat::{BlockingRectangle, CellType, Gat};
use sdl2::pixels::PixelFormatEnum;

pub(super) struct BackgroundAssetLoader<'a> {
    to_main_thread: Sender<FromBackgroundAssetLoaderMsg<'a>>,
    from_main_thread: Receiver<ToBackgroundAssetLoaderMsg>,
    asset_loader: CommonAssetLoader,
}

pub(super) struct SendableRawSdlSurface<'a>(pub &'a mut sdl2::sys::SDL_Surface);

unsafe impl<'a> Send for SendableRawSdlSurface<'a> {}

impl<'a> SendableRawSdlSurface<'a> {
    fn new(sdl_surface: sdl2::surface::Surface<'a>) -> SendableRawSdlSurface<'a> {
        let ptr = sdl_surface.raw();
        // prevent drop
        std::mem::forget(sdl_surface);
        SendableRawSdlSurface(unsafe { &mut *ptr })
    }
}

pub(super) enum ToBackgroundAssetLoaderMsg {
    StartLoadingSprites(Vec<TextureId>),
    LoadTexture {
        texture_id: TextureId,
        minmag: MyGlEnum,
        filename: String,
    },
    LoadModelPart1 {
        model_id_pool: Vec<usize>,
        texture_id_pool: Vec<TextureId>,
        rsw_model_instances: Vec<RswModelInstance>,
        map_width: u32,
        map_height: u32,
    },
    StartLoadingGnd {
        texture_id_pool: Vec<TextureId>,
        map_name: String,
        rectangles: Vec<BlockingRectangle>,
        gat: Gat,
        water_level: f32,
        water_wave_height: f32,
        colliders: Vec<(Vec2, Vec2)>,
    },
}

pub(super) enum FromBackgroundAssetLoaderMsg<'a> {
    StartLoadingSpritesResponse {
        sprites: Box<Sprites>,
        reserved_textures: Vec<ReservedTexturedata<'a>>,
        texture_id_pool: Vec<TextureId>,
    },
    StartLoadingGroundResponse {
        ground_result: AsyncGroundLoadResult,
        reserved_textures: Vec<ReservedTexturedata<'a>>,
        texture_id_pool: Vec<TextureId>,
    },
    LoadTextureResponse {
        texture_id: TextureId,
        minmag: MyGlEnum,
        content: Vec<u8>,
        filename: String,
    },
    LoadModelsResponse {
        models: HashMap<String, ModelLoadingData>,
        model_instances: Vec<ModelInstance>,
        reserved_textures: Vec<ReservedTexturedata<'a>>,
        texture_id_pool: Vec<TextureId>,
        model_id_pool: Vec<usize>,
    },
}

pub(super) struct ReservedTexturedata<'a> {
    pub texture_id: TextureId,
    pub name: String,
    pub raw_sdl_surface: SendableRawSdlSurface<'a>,
    pub minmag: MyGlEnum,
}

pub(super) struct AsyncGroundLoadResult {
    pub ground_vertex_array: Vec<MeshVertex>,
    pub ground_walkability_mesh: Vec<Point3<f32>>,
    pub ground_walkability_mesh2: Vec<Point3<f32>>,
    pub ground_walkability_mesh3: Vec<Point3<f32>>,
    pub ground_width: u32,
    pub ground_height: u32,
    pub texture_atlas: TextureId,
    pub tile_color_texture: TextureId,
    pub lightmap_texture: TextureId,
}

pub(super) struct ModelLoadingData {
    pub model_id: usize,
    pub data_for_rendering_full_model: Vec<Vec<SameTextureNodeFacesRaw>>,
    pub bbox: BoundingBox,
    pub alpha: u8,
}

impl<'a> BackgroundAssetLoader<'a> {
    pub fn new(
        to_main_thread: Sender<FromBackgroundAssetLoaderMsg>,
        from_main_thread: Receiver<ToBackgroundAssetLoaderMsg>,
        asset_loader: CommonAssetLoader,
    ) -> BackgroundAssetLoader {
        BackgroundAssetLoader {
            to_main_thread,
            from_main_thread,
            asset_loader,
        }
    }

    pub fn run(self) {
        loop {
            let msg = self.from_main_thread.recv();
            if msg.is_err() {
                break;
            }
            match msg.unwrap() {
                ToBackgroundAssetLoaderMsg::LoadTexture {
                    texture_id,
                    minmag,
                    filename,
                } => {
                    let content = self.asset_loader.get_content(&filename).unwrap();
                    self.to_main_thread
                        .send(FromBackgroundAssetLoaderMsg::LoadTextureResponse {
                            texture_id,
                            minmag,
                            content,
                            filename,
                        })
                        .expect("");
                }
                ToBackgroundAssetLoaderMsg::LoadModelPart1 {
                    mut model_id_pool,
                    rsw_model_instances,
                    mut texture_id_pool,
                    map_width,
                    map_height,
                } => {
                    let mut reserved_textures = Vec::<ReservedTexturedata>::with_capacity(1000);
                    let mut texture_map = HashMap::<String, TextureId>::with_capacity(128);
                    let model_names: HashSet<_> = rsw_model_instances
                        .iter()
                        .map(|m| m.filename.clone())
                        .collect();
                    let models: HashMap<String, ModelLoadingData> = model_names
                        .into_iter()
                        .map(|model_name| {
                            let file_name = format!("data\\model\\{}", model_name);
                            let content = self.asset_loader.get_content(&file_name).unwrap();
                            let model_id = model_id_pool.pop().unwrap();
                            let rsm = Rsm::load(BinaryReader::from_vec(content));
                            let textures: Vec<(String, TextureId)> = rsm
                                .texture_names
                                .iter()
                                .map(|texture_name| {
                                    let texture_id = texture_map
                                        .entry(texture_name.to_string())
                                        .or_insert_with(|| {
                                            let path = format!("data\\texture\\{}", texture_name);
                                            self.load_texture(
                                                &path,
                                                MyGlEnum::NEAREST,
                                                &mut texture_id_pool,
                                                &mut reserved_textures,
                                            )
                                            .unwrap()
                                        });
                                    (texture_name.to_string(), *texture_id)
                                })
                                .collect::<Vec<_>>();
                            let (data_for_rendering_full_model, bbox): (
                                Vec<Vec<SameTextureNodeFacesRaw>>,
                                BoundingBox,
                            ) = Rsm::generate_meshes_by_texture_id(
                                &rsm.bounding_box,
                                rsm.shade_type,
                                rsm.nodes.len() == 1,
                                &rsm.nodes,
                                &textures,
                            );
                            (
                                model_name,
                                ModelLoadingData {
                                    model_id,
                                    data_for_rendering_full_model,
                                    bbox,
                                    alpha: rsm.alpha,
                                },
                            )
                        })
                        .collect();
                    //
                    let model_instances = rsw_model_instances
                        .into_iter()
                        .map(|rsw_model_instance| {
                            BackgroundAssetLoader::to_model_instance(
                                rsw_model_instance,
                                &models,
                                map_width,
                                map_height,
                            )
                        })
                        .collect();
                    self.to_main_thread
                        .send(FromBackgroundAssetLoaderMsg::LoadModelsResponse {
                            models,
                            model_instances,
                            reserved_textures,
                            texture_id_pool,
                            model_id_pool,
                        })
                        .expect("");
                }
                ToBackgroundAssetLoaderMsg::StartLoadingSprites(mut texture_id_pool) => {
                    let mut reserved_textures = Vec::<ReservedTexturedata>::with_capacity(8_000);
                    let sprites = self.load_sprites(&mut texture_id_pool, &mut reserved_textures);
                    self.to_main_thread
                        .send(FromBackgroundAssetLoaderMsg::StartLoadingSpritesResponse {
                            sprites: Box::new(sprites),
                            reserved_textures,
                            texture_id_pool,
                        })
                        .expect("");
                }
                ToBackgroundAssetLoaderMsg::StartLoadingGnd {
                    mut texture_id_pool,
                    map_name,
                    rectangles,
                    gat,
                    water_level,
                    water_wave_height: water_height,
                    colliders,
                } => {
                    let mut reserved_textures = Vec::<ReservedTexturedata>::with_capacity(3);
                    let result = self.load_ground(
                        &map_name,
                        &gat,
                        rectangles,
                        water_level,
                        water_height,
                        &colliders,
                        &mut texture_id_pool,
                        &mut reserved_textures,
                    );
                    self.to_main_thread
                        .send(FromBackgroundAssetLoaderMsg::StartLoadingGroundResponse {
                            ground_result: result,
                            reserved_textures,
                            texture_id_pool,
                        })
                        .expect("");
                }
            }
        }
    }

    fn to_model_instance(
        model_instance: RswModelInstance,
        models: &HashMap<String, ModelLoadingData>,
        map_width: u32,
        map_height: u32,
    ) -> ModelInstance {
        let mut only_translation_matrix: vek::Mat4<f32> = vek::Mat4::identity();
        {
            let t = model_instance.pos + Vec3::new(map_width as f32, 0f32, map_height as f32);
            only_translation_matrix.translate_3d((t.x, t.y, t.z));
        }

        let mut instance_matrix: vek::Mat4<f32> = only_translation_matrix.clone();
        instance_matrix *= vek::Mat4::rotation_z(model_instance.rot.z.to_radians());
        instance_matrix *= vek::Mat4::rotation_x(model_instance.rot.x.to_radians());
        instance_matrix *= vek::Mat4::rotation_y(model_instance.rot.y.to_radians());

        instance_matrix *= vek::Mat4::scaling_3d((
            model_instance.scale.x,
            model_instance.scale.y,
            model_instance.scale.z,
        ));
        only_translation_matrix *= vek::Mat4::scaling_3d((
            model_instance.scale.x,
            model_instance.scale.y,
            model_instance.scale.z,
        ));

        instance_matrix.rotate_x(180f32.to_radians());
        only_translation_matrix.rotate_x(180f32.to_radians());

        let model_data = models.get(&model_instance.filename).expect("");
        let model_bb = &model_data.bbox;
        let tmin: vek::Vec3<f32> = only_translation_matrix.mul_point(vek::Vec3::new(
            model_bb.min.x,
            model_bb.min.y,
            model_bb.min.z,
        ));
        let tmax: vek::Vec3<f32> = only_translation_matrix.mul_point(vek::Vec3::new(
            model_bb.max.x,
            model_bb.max.y,
            model_bb.max.z,
        ));
        let min = Vec3::new(
            tmin[0].min(tmax[0]),
            tmin[1].min(tmax[1]),
            tmin[2].max(tmax[2]),
        );
        let max = Vec3::new(
            tmax[0].max(tmin[0]),
            tmax[1].max(tmin[1]),
            tmax[2].min(tmin[2]),
        );
        ModelInstance {
            asset_db_model_index: model_data.model_id,
            matrix: Mat4::new(
                instance_matrix[(0, 0)],
                instance_matrix[(0, 1)],
                instance_matrix[(0, 2)],
                instance_matrix[(0, 3)],
                instance_matrix[(1, 0)],
                instance_matrix[(1, 1)],
                instance_matrix[(1, 2)],
                instance_matrix[(1, 3)],
                instance_matrix[(2, 0)],
                instance_matrix[(2, 1)],
                instance_matrix[(2, 2)],
                instance_matrix[(2, 3)],
                instance_matrix[(3, 0)],
                instance_matrix[(3, 1)],
                instance_matrix[(3, 2)],
                instance_matrix[(3, 3)],
            ),
            bottom_left_front: min,
            top_right_back: max,
        }
    }

    fn load_ground(
        &self,
        map_name: &str,
        gat: &Gat,
        rectangles: Vec<BlockingRectangle>,
        water_level: f32,
        water_wave_height: f32,
        colliders: &Vec<(Vec2, Vec2)>,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> AsyncGroundLoadResult {
        let mut v = v3(0.0, 0.0, 0.0);
        let rot = Rotation3::<f32>::new(Vec3::new(180f32.to_radians(), 0.0, 0.0));
        let mut rotate_around_x_axis = |mut pos: Point3<f32>| {
            v.x = pos[0];
            v.y = pos[1];
            v.z = pos[2];
            v = rot * v;
            pos[0] = v.x;
            pos[1] = v.y;
            pos[2] = v.z;
            pos
        };

        log::info!("vertices");
        let vertices: Vec<Point3<f32>> = rectangles
            .iter()
            .map(|cell| {
                let x = cell.start_x as f32;
                let x2 = (cell.start_x + cell.width) as f32;
                let y = (cell.bottom - cell.height + 1) as f32;
                let y2 = (cell.bottom + 1) as f32;
                vec![
                    rotate_around_x_axis(Point3::new(x, -2.0, y2)),
                    rotate_around_x_axis(Point3::new(x2, -2.0, y2)),
                    rotate_around_x_axis(Point3::new(x, -2.0, y)),
                    rotate_around_x_axis(Point3::new(x, -2.0, y)),
                    rotate_around_x_axis(Point3::new(x2, -2.0, y2)),
                    rotate_around_x_axis(Point3::new(x2, -2.0, y)),
                ]
            })
            .flatten()
            .collect();

        let vertices2: Vec<Point3<f32>> = gat
            .cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let x = (i as u32 % gat.width) as f32;
                let y = (i as u32 / gat.width) as f32;
                if cell.cell_type & CellType::Walkable as u8 == 0 {
                    vec![
                        rotate_around_x_axis(Point3::new(x + 0.0, -1.0, y + 1.0)),
                        rotate_around_x_axis(Point3::new(x + 1.0, -1.0, y + 1.0)),
                        rotate_around_x_axis(Point3::new(x + 0.0, -1.0, y + 0.0)),
                        rotate_around_x_axis(Point3::new(x + 0.0, -1.0, y + 0.0)),
                        rotate_around_x_axis(Point3::new(x + 1.0, -1.0, y + 1.0)),
                        rotate_around_x_axis(Point3::new(x + 1.0, -1.0, y + 0.0)),
                    ]
                } else {
                    vec![]
                }
            })
            .flatten()
            .collect();
        let ground_walkability_mesh = vertices;
        let ground_walkability_mesh2 = vertices2;
        let vertices: Vec<Point3<f32>> = colliders
            .iter()
            .map(|(extents, pos)| {
                let x = pos.x - extents.x;
                let x2 = pos.x + extents.x;
                let y = pos.y - extents.y;
                let y2 = pos.y + extents.y;
                vec![
                    Point3::new(x, 3.0, y2),
                    Point3::new(x2, 3.0, y2),
                    Point3::new(x, 3.0, y),
                    Point3::new(x, 3.0, y),
                    Point3::new(x2, 3.0, y2),
                    Point3::new(x2, 3.0, y),
                ]
            })
            .flatten()
            .collect();
        let ground_walkability_mesh3 = vertices;
        let (elapsed, mut ground) = measure_time(|| {
            self.load_gnd(map_name, water_level, water_wave_height)
                .unwrap()
        });
        log::info!("gnd loaded: {}ms", elapsed.as_millis());
        let (elapsed, texture_atlas) = measure_time(|| {
            self.create_gl_texture_atlas(&ground.texture_names, texture_id_pool, reserved_textures)
        });
        log::info!("gnd texture_atlas loaded: {}ms", elapsed.as_millis());

        let tile_color_texture = BackgroundAssetLoader::create_tile_color_texture(
            &mut ground.tiles_color_image,
            ground.width,
            ground.height,
            texture_id_pool,
            reserved_textures,
        );
        let lightmap_texture = BackgroundAssetLoader::create_lightmap_texture(
            &mut ground.lightmap_image,
            ground.lightmaps.count,
            texture_id_pool,
            reserved_textures,
        );
        let ground_vertex_array = std::mem::replace(&mut ground.mesh, vec![]);

        AsyncGroundLoadResult {
            ground_vertex_array,
            ground_walkability_mesh,
            ground_walkability_mesh2,
            ground_walkability_mesh3,
            ground_width: ground.width,
            ground_height: ground.height,
            texture_atlas,
            tile_color_texture,
            lightmap_texture,
        }
    }

    pub fn create_tile_color_texture(
        tiles_color_buffer: &mut Vec<u8>,
        width: u32,
        height: u32,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> TextureId {
        let tile_color_surface = sdl2::surface::Surface::from_data(
            tiles_color_buffer,
            width,
            height,
            4 * width,
            PixelFormatEnum::BGRA32,
        )
        .unwrap();

        let scaled_w = width.next_power_of_two();
        let scaled_h = height.next_power_of_two();

        let mut scaled_tiles_color_surface =
            sdl2::surface::Surface::new(scaled_w, scaled_h, PixelFormatEnum::BGRA32)
                .unwrap()
                .convert(&tile_color_surface.pixel_format())
                .unwrap();
        tile_color_surface
            .blit_scaled(
                None,
                &mut scaled_tiles_color_surface,
                sdl2::rect::Rect::new(0, 0, scaled_w, scaled_h),
            )
            .unwrap();

        let texture_id = texture_id_pool.pop().unwrap();
        reserved_textures.push(ReservedTexturedata {
            texture_id,
            name: "ground_tile_color_texture".to_string(),
            raw_sdl_surface: SendableRawSdlSurface::new(scaled_tiles_color_surface),
            minmag: MyGlEnum::LINEAR,
        });
        return texture_id;
    }

    pub fn create_lightmap_texture(
        lightmap: &mut Vec<u8>,
        count: u32,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> TextureId {
        let width = ((count as f32).sqrt().round() as u32 * 8).next_power_of_two();
        let height = ((count as f32).sqrt().ceil() as u32 * 8).next_power_of_two();

        let texture_id = texture_id_pool.pop().unwrap();
        let surface = {
            sdl2::surface::Surface::from_data(
                lightmap,
                width,
                height,
                width * 4,
                sdl2::pixels::PixelFormatEnum::BGRA32,
            )
            .unwrap()
        };
        //         clone this surface because the lightmap will be freed when load_ground exits.
        let mut cloned_surface =
            sdl2::surface::Surface::new(width, height, PixelFormatEnum::BGRA32).unwrap();
        surface
            .blit_scaled(None, &mut cloned_surface, None)
            .unwrap();

        reserved_textures.push(ReservedTexturedata {
            texture_id,
            name: "ground_lightmap_texture".to_string(),
            raw_sdl_surface: SendableRawSdlSurface::new(cloned_surface),
            minmag: MyGlEnum::LINEAR,
        });
        return texture_id;
    }

    pub fn create_gl_texture_atlas(
        &self,
        texture_names: &Vec<String>,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> TextureId {
        let texture_surfaces: Vec<sdl2::surface::Surface> = texture_names
            .iter()
            .map(|texture_name| {
                let path = format!("data\\texture\\{}", texture_name);
                let surface = GrfEntryLoader::load_sdl_surface2(
                    self.asset_loader.get_content(&path).unwrap(),
                    path.ends_with(".tga"),
                );
                surface.unwrap()
            })
            .collect();
        let surface_atlas = Gnd::create_texture_atlas(texture_surfaces);
        let texture_id = texture_id_pool.pop().unwrap();
        reserved_textures.push(ReservedTexturedata {
            texture_id,
            name: "ground_texture_atlas".to_string(),
            raw_sdl_surface: SendableRawSdlSurface::new(surface_atlas),
            minmag: MyGlEnum::NEAREST,
        });
        return texture_id;
    }

    pub fn load_gnd(
        &self,
        map_name: &str,
        water_level: f32,
        water_height: f32,
    ) -> Result<Gnd, String> {
        let file_name = format!("data\\{}.gnd", map_name);
        let content = self.asset_loader.get_content(&file_name)?;
        return Ok(Gnd::load(
            BinaryReader::from_vec(content),
            water_level,
            water_height,
        ));
    }

    fn load_sprites(
        &self,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> Sprites {
        let mut string_buffer = String::with_capacity(512);
        let job_sprite_name_table = job_name_table();
        let sprites = Sprites {
            cursors: self
                .load_spr_and_act("data\\sprite\\cursors", texture_id_pool, reserved_textures)
                .unwrap(),
            exoskeleton: {
                let mut exoskeleton = self
                    .load_spr_and_act(
                        "data\\sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\¸¶µµ±â¾î_³²",
                        texture_id_pool,
                        reserved_textures,
                    )
                    .unwrap();
                // for Idle action, character sprites contains head rotating animations, we don't need them
                exoskeleton
                    .action
                    .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                exoskeleton
            },
            ginseng_bullet: self
                .load_spr_and_act(
                    "data\\sprite\\¸ó½ºÅÍ\\ginseng_bullet",
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            arrow: self
                .load_spr_and_act(
                    "data\\sprite\\npc\\skel_archer_arrow",
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            falcon: self
                .load_spr_and_act(
                    "data\\sprite\\ÀÌÆÑÆ®\\¸Å",
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            stun: self
                .load_spr_and_act(
                    "data\\sprite\\ÀÌÆÑÆ®\\status-stun",
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            timefont: self
                .load_spr_and_act(
                    "data\\sprite\\ÀÌÆÑÆ®\\timefont",
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            numbers: {
                let texture_id = texture_id_pool.pop().unwrap();
                let sdl_surface = BackgroundAssetLoader::sdl_surface_from_file("assets/damage.bmp");
                reserved_textures.push(ReservedTexturedata {
                    texture_id,
                    name: "assets/damage.bmp".to_string(),
                    raw_sdl_surface: SendableRawSdlSurface::new(sdl_surface),
                    minmag: MyGlEnum::NEAREST,
                });
                texture_id
            },
            magic_target: self
                .load_texture(
                    "data\\texture\\effect\\magic_target.tga",
                    MyGlEnum::NEAREST,
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            fire_particle: self
                .load_texture(
                    "data\\texture\\effect\\fireparticle.tga",
                    MyGlEnum::NEAREST,
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            clock: self
                .load_texture(
                    "data\\texture\\effect\\blast_mine##clock.bmp",
                    MyGlEnum::NEAREST,
                    texture_id_pool,
                    reserved_textures,
                )
                .unwrap(),
            mounted_character_sprites: {
                log::info!(">>> load mounted_character_sprites");
                let mut mounted_sprites = HashMap::new();
                let mounted_file_name = &job_sprite_name_table[&JobSpriteId::CRUSADER2];
                let folder1 = encoding::all::WINDOWS_1252
                    .decode(&[0xC0, 0xCE, 0xB0, 0xA3, 0xC1, 0xB7], DecoderTrap::Strict)
                    .unwrap();
                let folder2 = encoding::all::WINDOWS_1252
                    .decode(&[0xB8, 0xF6, 0xC5, 0xEB], DecoderTrap::Strict)
                    .unwrap();
                let male_file_name = format!(
                    "data\\sprite\\{}\\{}\\³²\\{}_³²",
                    folder1, folder2, mounted_file_name
                );
                let mut male = self
                    .load_spr_and_act(&male_file_name, texture_id_pool, reserved_textures)
                    .expect(&format!("Failed loading {:?}", JobSpriteId::CRUSADER2));
                // for Idle action, character sprites contains head rotating animations, we don't need them
                male.action
                    .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                let female = male.clone();
                mounted_sprites.insert(JobId::CRUSADER, [male, female]);
                log::info!("<<< load mounted_character_sprites");
                mounted_sprites
            },
            character_sprites: self.load_char_sprites(
                &job_sprite_name_table,
                texture_id_pool,
                reserved_textures,
            ),
            head_sprites: self.load_head_sprites(
                &mut string_buffer,
                texture_id_pool,
                reserved_textures,
            ),
            monster_sprites: self.load_monster_sprites(
                &mut string_buffer,
                texture_id_pool,
                reserved_textures,
            ),
            effect_sprites: EffectSprites {
                torch: self
                    .load_spr_and_act(
                        "data\\sprite\\ÀÌÆÑÆ®\\torch_01",
                        texture_id_pool,
                        reserved_textures,
                    )
                    .unwrap(),
                fire_wall: self
                    .load_spr_and_act(
                        "data\\sprite\\ÀÌÆÑÆ®\\firewall",
                        texture_id_pool,
                        reserved_textures,
                    )
                    .unwrap(),
                fire_ball: self
                    .load_spr_and_act(
                        "data\\sprite\\ÀÌÆÑÆ®\\fireball",
                        texture_id_pool,
                        reserved_textures,
                    )
                    .unwrap(),
                plasma: self
                    .load_spr_and_act(
                        "data\\sprite\\¸ó½ºÅÍ\\plasma_r",
                        texture_id_pool,
                        reserved_textures,
                    )
                    .unwrap(),
            },
        };
        return sprites;
    }

    pub fn load_texture(
        &self,
        texture_path: &str,
        min_mag: MyGlEnum,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> Result<TextureId, String> {
        if let Ok(content) = self.asset_loader.get_content(texture_path) {
            let surface =
                GrfEntryLoader::load_sdl_surface2(content, texture_path.ends_with(".tga")).unwrap();
            let texture_id = texture_id_pool.pop().unwrap();
            reserved_textures.push(ReservedTexturedata {
                texture_id,
                name: texture_path.to_string(),
                raw_sdl_surface: SendableRawSdlSurface::new(surface),
                minmag: min_mag,
            });
            return Ok(texture_id);
        } else {
            return Err(format!("No entry found in GRFs '{}'", texture_path));
        }
    }

    pub fn load_spr_and_act(
        &self,
        path: &str,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> Result<SpriteResource, String> {
        self.load_spr_and_act_inner(path, None, None, texture_id_pool, reserved_textures)
    }

    fn load_spr_and_act_inner(
        &self,
        path: &str,
        palette_index: Option<usize>,
        palette: Option<&[u8]>,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> Result<SpriteResource, String> {
        let content = self.asset_loader.get_content(&format!("{}.spr", path))?;
        let mut reader = BinaryReader::from_vec(content);
        let (version, indexed_frame_count, rgba_frame_count) = SpriteFile::read_header(&mut reader);
        let texture_ids = (0..(indexed_frame_count + rgba_frame_count as usize))
            .map(|_it| texture_id_pool.pop().unwrap())
            .collect::<Vec<_>>();

        let frames = SpriteFile::load(
            reader,
            palette,
            version,
            indexed_frame_count,
            rgba_frame_count,
        )
        .frames;
        frames
            .into_iter()
            .map(|frame| BackgroundAssetLoader::sdl_surface_from_frame(frame))
            .enumerate()
            .for_each(|(index, sdl_surface)| {
                reserved_textures.push(ReservedTexturedata {
                    texture_id: texture_ids[index],
                    name: format!(
                        "{}_{}_{}",
                        &path.to_string(),
                        palette_index.unwrap_or(0),
                        index
                    ),
                    raw_sdl_surface: SendableRawSdlSurface::new(sdl_surface),
                    minmag: MyGlEnum::NEAREST,
                })
            });

        let content = self.asset_loader.get_content(&format!("{}.act", path))?;
        let action = ActionFile::load(BinaryReader::from_vec(content));
        return Ok(SpriteResource {
            action,
            textures: texture_ids,
        });
    }

    fn sdl_surface_from_frame(
        mut frame: crate::grf::spr::SprFrame,
    ) -> sdl2::surface::Surface<'static> {
        let frame_surface = sdl2::surface::Surface::from_data(
            &mut frame.data,
            frame.width as u32,
            frame.height as u32,
            (4 * frame.width) as u32,
            PixelFormatEnum::RGBA32,
        )
        .unwrap();

        let mut opengl_surface = sdl2::surface::Surface::new(
            frame.width as u32,
            frame.height as u32,
            PixelFormatEnum::RGBA32,
        )
        .unwrap();

        let dst_rect = sdl2::rect::Rect::new(0, 0, frame.width as u32, frame.height as u32);
        frame_surface
            .blit(None, &mut opengl_surface, dst_rect)
            .unwrap();
        return opengl_surface;
    }

    pub fn sdl_surface_from_file<P: AsRef<Path>>(path: P) -> sdl2::surface::Surface<'static> {
        use sdl2::image::LoadSurface;
        let mut surface = sdl2::surface::Surface::from_file(&path).unwrap();
        let mut optimized_surf = sdl2::surface::Surface::new(
            surface.width(),
            surface.height(),
            sdl2::pixels::PixelFormatEnum::RGBA32,
        )
        .unwrap();
        surface
            .set_color_key(true, sdl2::pixels::Color::RGB(255, 0, 255))
            .unwrap();
        surface.blit(None, &mut optimized_surf, None).unwrap();
        return optimized_surf;
    }

    fn load_head_sprites(
        &self,
        string_buffer: &mut String,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> [Vec<SpriteResource>; 2] {
        log::info!(">>> load_head_sprites");
        let sprites = [
            (1..=25)
                .map(|i| {
                    let male_file_name = {
                        string_buffer.clear();
                        write!(
                            string_buffer,
                            "data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\³²\\{}_³²",
                            i.to_string()
                        )
                        .expect("");
                        &string_buffer
                    };
                    let male = if self
                        .asset_loader
                        .exists(&((*male_file_name).to_owned() + ".act"))
                    {
                        let mut head = self
                            .load_spr_and_act(male_file_name, texture_id_pool, reserved_textures)
                            .expect(&format!("Failed loading head({})", i));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        head.action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        Some(head)
                    } else {
                        None
                    };
                    male
                })
                .filter_map(|it| it)
                .collect::<Vec<SpriteResource>>(),
            (1..=25)
                .map(|i| {
                    let female_file_name = {
                        string_buffer.clear();
                        write!(
                            string_buffer,
                            "data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\¿©\\{}_¿©",
                            i.to_string()
                        )
                        .expect("");
                        &string_buffer
                    };
                    let female = if self
                        .asset_loader
                        .exists(&((*female_file_name).to_owned() + ".act"))
                    {
                        let mut head = self
                            .load_spr_and_act(female_file_name, texture_id_pool, reserved_textures)
                            .expect(&format!("Failed loading head({})", i));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        head.action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        Some(head)
                    } else {
                        None
                    };
                    female
                })
                .filter_map(|it| it)
                .collect::<Vec<SpriteResource>>(),
        ];
        log::info!("<<< load_head_sprites");
        return sprites;
    }

    fn load_monster_sprites(
        &self,
        mut string_buffer: &mut String,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> HashMap<MonsterId, SpriteResource> {
        log::info!(">>> load_monster_sprites");
        let sprites = MonsterId::iter()
            .map(|monster_id| {
                let file_name = {
                    string_buffer.clear();
                    write!(
                        &mut string_buffer,
                        "data\\sprite\\npc\\{}",
                        monster_id.to_string().to_lowercase()
                    )
                    .expect("");
                    &string_buffer
                };
                (
                    monster_id,
                    self.load_spr_and_act(&file_name, texture_id_pool, reserved_textures)
                        .or_else(|_e| {
                            let file_name = {
                                string_buffer.clear();
                                write!(
                                    &mut string_buffer,
                                    "data\\sprite\\¸ó½ºÅÍ\\{}",
                                    monster_id.to_string().to_lowercase()
                                )
                                .expect("");
                                &string_buffer
                            };
                            self.load_spr_and_act(&file_name, texture_id_pool, reserved_textures)
                        })
                        .unwrap(),
                )
            })
            .collect::<HashMap<MonsterId, SpriteResource>>();
        log::info!("<<< load_monster_sprites");
        return sprites;
    }

    fn load_char_sprites(
        &self,
        job_sprite_name_table: &HashMap<JobSpriteId, String>,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> HashMap<JobSpriteId, [[SpriteResource; 2]; 2]> {
        log::info!(">>> load_char_sprites");

        let mut string_buffer1 = String::with_capacity(512);
        let mut string_buffer2 = String::with_capacity(512);
        let sprites = PLAYABLE_CHAR_SPRITES
            .iter()
            .map(|job_sprite_id| {
                let job_file_name = &job_sprite_name_table[&job_sprite_id];
                let folder1 = encoding::all::WINDOWS_1252
                    .decode(&[0xC0, 0xCE, 0xB0, 0xA3, 0xC1, 0xB7], DecoderTrap::Strict)
                    .unwrap();
                let folder2 = encoding::all::WINDOWS_1252
                    .decode(&[0xB8, 0xF6, 0xC5, 0xEB], DecoderTrap::Strict)
                    .unwrap();
                let male_file_path = {
                    string_buffer1.clear();
                    write!(
                        &mut string_buffer1,
                        "data\\sprite\\{}\\{}\\³²\\{}_³²",
                        folder1, folder2, job_file_name
                    )
                    .expect("");
                    &string_buffer1
                };
                let female_file_path = {
                    string_buffer2.clear();
                    write!(
                        &mut string_buffer2,
                        "data\\sprite\\{}\\{}\\¿©\\{}_¿©",
                        folder1, folder2, job_file_name
                    )
                    .expect("");
                    &string_buffer2
                };

                // order is red, blue
                let (male_palette_ids, female_palette_ids) = match job_sprite_id {
                    JobSpriteId::CRUSADER => ([153, 152], [153, 152]),
                    JobSpriteId::SWORDMAN => ([153, 152], [153, 152]),
                    JobSpriteId::ARCHER => ([153, 152], [153, 152]),
                    JobSpriteId::ASSASSIN => ([39, 38], [39, 38]),
                    JobSpriteId::ROGUE => ([153, 152], [153, 152]),
                    JobSpriteId::KNIGHT => ([348, 316], [348, 316]),
                    JobSpriteId::WIZARD => ([3, 1], [122, 129]),
                    JobSpriteId::SAGE => ([89, 84], [122, 132]),
                    JobSpriteId::ALCHEMIST => ([3, 1], [3, 1]),
                    JobSpriteId::BLACKSMITH => ([293, 292], [293, 292]),
                    JobSpriteId::PRIEST => ([153, 152], [153, 152]),
                    JobSpriteId::MONK => ([39, 38], [39, 38]),
                    JobSpriteId::GUNSLINGER => ([55, 54], [55, 54]),
                    JobSpriteId::RANGER => ([3, 1], [3, 1]),
                    _ => panic!(),
                };

                let (male_red, male_blue, female_red, female_blue) = if !self
                    .asset_loader
                    .exists(&format!("{}.act", female_file_path))
                {
                    let mut male = self
                        .load_spr_and_act(&male_file_path, texture_id_pool, reserved_textures)
                        .expect(&format!("Failed loading {:?}", job_sprite_id));
                    // for Idle action, character sprites contains head rotating animations, we don't need them
                    male.action
                        .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                    let female = male.clone();
                    (male.clone(), female.clone(), male, female)
                } else if !self.asset_loader.exists(&format!("{}.act", male_file_path)) {
                    let mut female = self
                        .load_spr_and_act(&female_file_path, texture_id_pool, reserved_textures)
                        .expect(&format!("Failed loading {:?}", job_sprite_id));
                    // for Idle action, character sprites contains head rotating animations, we don't need them
                    female
                        .action
                        .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                    let male = female.clone();
                    (male.clone(), female.clone(), male, female)
                } else {
                    let male_red = self.load_sprite(
                        &job_sprite_id,
                        &job_file_name,
                        &male_file_path,
                        male_palette_ids[0],
                        texture_id_pool,
                        reserved_textures,
                    );
                    let male_blue = self.load_sprite(
                        &job_sprite_id,
                        &job_file_name,
                        &male_file_path,
                        male_palette_ids[1],
                        texture_id_pool,
                        reserved_textures,
                    );
                    let female_red = self.load_sprite(
                        &job_sprite_id,
                        &job_file_name,
                        &female_file_path,
                        female_palette_ids[0],
                        texture_id_pool,
                        reserved_textures,
                    );
                    let female_blue = self.load_sprite(
                        &job_sprite_id,
                        &job_file_name,
                        &female_file_path,
                        female_palette_ids[1],
                        texture_id_pool,
                        reserved_textures,
                    );
                    (male_red, male_blue, female_red, female_blue)
                };
                (
                    *job_sprite_id,
                    [[male_red, female_red], [male_blue, female_blue]],
                )
            })
            .collect::<HashMap<JobSpriteId, [[SpriteResource; 2]; 2]>>();
        log::info!("<<< load_char_sprites");
        return sprites;
    }

    fn load_sprite(
        &self,
        job_sprite_id: &JobSpriteId,
        job_file_name: &str,
        file_path: &str,
        palette_id: usize,
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> SpriteResource {
        let palette = self.load_palette(&job_sprite_id, job_file_name, palette_id);
        let mut sprite_res = self
            .load_spr_and_act_with_palette(
                &file_path,
                palette_id,
                palette.as_slice(),
                texture_id_pool,
                reserved_textures,
            )
            .expect(&format!("Failed loading {:?}", job_sprite_id));
        // for Idle action, character sprites contains head rotating animations, we don't need them
        sprite_res
            .action
            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
        sprite_res
    }

    pub fn load_spr_and_act_with_palette(
        &self,
        path: &str,
        palette_index: usize,
        palette: &[u8],
        texture_id_pool: &mut Vec<TextureId>,
        reserved_textures: &mut Vec<ReservedTexturedata>,
    ) -> Result<SpriteResource, String> {
        self.load_spr_and_act_inner(
            path,
            Some(palette_index),
            Some(palette),
            texture_id_pool,
            reserved_textures,
        )
    }

    fn load_palette(
        &self,
        job_sprite_id: &JobSpriteId,
        job_file_name: &str,
        palette_id: usize,
    ) -> Vec<u8> {
        let palette = {
            // for some jobs, the palette file name is truncated, so this
            // code tries names one by one removing the last char in each
            // iteration
            let mut tmp_name: String = job_file_name.to_owned();
            loop {
                if tmp_name.is_empty() {
                    break Err("".to_owned());
                }
                let pal = self.asset_loader.get_content(&format!(
                    "data\\palette\\¸ö\\{}_³²_{}.pal",
                    tmp_name, palette_id
                ));
                if pal.is_ok() {
                    break pal;
                }
                tmp_name.pop();
            }
        }
        .expect(&format!(
            "Couldn't load palette file for {}, id: {}",
            job_sprite_id, palette_id
        ));
        palette
    }
}
