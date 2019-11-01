use crate::asset::asset_async_loader::{
    AsyncGroundLoadResult, BackgroundAssetLoader, FromBackgroundAssetLoaderMsg, ModelLoadingData,
    ReservedTexturedata, ToBackgroundAssetLoaderMsg,
};
use crate::asset::binary_reader::BinaryReader;
use crate::asset::database::AssetDatabase;
use crate::asset::gat::{BlockingRectangle, Gat};
use crate::asset::rsw::{Rsw, RswModelInstance, WaterData};
use crate::asset::str::StrFile;
use crate::asset::texture::{GlNativeTextureId, GlTexture, TextureId};
use crate::asset::GrfEntry;
use crate::common::Vec2;
use crate::my_gl::{Gl, MyGlEnum};
use crate::runtime_assets::map::{
    MapRenderData, ModelInstance, ModelRenderData, SameTextureNodeFaces,
};
use crate::systems::SystemVariables;
use crate::video::{VertexArray, VertexAttribDefinition};
use byteorder::WriteBytesExt;
use byteorder::{LittleEndian, ReadBytesExt};
use sdl2::image::ImageRWops;
use sdl2::mixer::LoaderRWops;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::ops::Shl;
use std::os::raw::c_void;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};

const GRF_HEADER_SIZE: usize = 15 + 15 + 4 * 4;

// entry is a file
#[allow(dead_code)]
const GRF_FILELIST_TYPE_FILE: u8 = 0x01;

// encryption mode 0 (header DES + periodic DES/shuffle)
const GRF_FILELIST_TYPE_ENCRYPT_MIXED: u8 = 0x02;

// encryption mode 1 (header DES only)
const GRF_FILELIST_TYPE_ENCRYPT_HEADER: u8 = 0x04;

pub struct AssetLoader<'a> {
    to_2nd_thread: Sender<ToBackgroundAssetLoaderMsg>,
    from_2nd_thread: Receiver<FromBackgroundAssetLoaderMsg<'a>>,
    entries: HashMap<String, (usize, GrfEntry)>,
    paths: Vec<String>,
}

impl<'a> AssetLoader<'a> {
    pub fn load_sprites(&self, gl: &Gl, asset_db: &mut AssetDatabase) {
        self.to_2nd_thread
            .send(ToBackgroundAssetLoaderMsg::StartLoadingSprites(
                asset_db.reserve_texture_slots(gl, 10_000),
            ))
            .expect("");
    }

    pub fn start_loading_ground(
        &self,
        gl: &Gl,
        asset_db: &mut AssetDatabase,
        map_name: &str,
        rectangles: Vec<BlockingRectangle>,
        gat: Gat,
        water: WaterData,
        colliders: Vec<(Vec2, Vec2)>,
    ) {
        let texture_id_pool = asset_db.reserve_texture_slots(gl, 3);
        self.to_2nd_thread
            .send(ToBackgroundAssetLoaderMsg::StartLoadingGnd {
                texture_id_pool,
                map_name: map_name.to_string(),
                rectangles: rectangles,
                gat,
                water,
                colliders,
            })
            .expect("");
    }

    pub fn process_async_loading(
        &self,
        gl: &Gl,
        sys_vars: &mut SystemVariables,
        asset_db: &mut AssetDatabase,
        map_render_data: &mut MapRenderData,
    ) {
        loop {
            let msg = self.from_2nd_thread.try_recv();
            if let Ok(msg) = msg {
                match msg {
                    FromBackgroundAssetLoaderMsg::LoadTextureResponse {
                        texture_id,
                        minmag,
                        content,
                        file_name,
                    } => {
                        let surface =
                            AssetLoader::load_sdl_surface2(content, file_name.ends_with(".tga"))
                                .unwrap();

                        let gl_texture =
                            AssetLoader::create_texture_from_surface_inner(gl, surface, minmag);
                        asset_db.fill_reserved_texture_slot(texture_id, gl_texture);
                    }
                    FromBackgroundAssetLoaderMsg::StartLoadingSpritesResponse {
                        sprites,
                        reserved_textures,
                        texture_id_pool,
                    } => {
                        sys_vars.assets.sprites = sprites;
                        log::info!("{} Sprites have been loaded", reserved_textures.len());
                        log::info!("{} Unused texture slot", texture_id_pool.len());
                        AssetLoader::set_reserved_textures(gl, asset_db, reserved_textures)
                    }
                    FromBackgroundAssetLoaderMsg::LoadModelsResponse {
                        models,
                        model_instances,
                        reserved_textures,
                        texture_id_pool,
                        model_id_pool,
                    } => AssetLoader::process_load_models_response(
                        gl,
                        asset_db,
                        map_render_data,
                        models,
                        model_instances,
                        reserved_textures,
                        texture_id_pool,
                        model_id_pool,
                    ),
                    FromBackgroundAssetLoaderMsg::StartLoadingGroundResponse {
                        ground_result,
                        reserved_textures,
                        texture_id_pool,
                    } => <AssetLoader<'a>>::process_load_ground_response(
                        gl,
                        asset_db,
                        map_render_data,
                        ground_result,
                        reserved_textures,
                        texture_id_pool,
                    ),
                }
            } else {
                break;
            }
        }
    }

    fn process_load_ground_response(
        gl: &Gl,
        asset_db: &mut AssetDatabase,
        map_render_data: &mut MapRenderData,
        ground_result: AsyncGroundLoadResult,
        reserved_textures: Vec<ReservedTexturedata>,
        texture_id_pool: Vec<TextureId>,
    ) -> () {
        map_render_data.ground_width = ground_result.ground_width;
        map_render_data.ground_height = ground_result.ground_height;
        map_render_data.ground_vertex_array = VertexArray::new_static(
            gl,
            MyGlEnum::TRIANGLES,
            ground_result.ground_vertex_array,
            vec![
                VertexAttribDefinition {
                    number_of_components: 3,
                    offset_of_first_element: 0,
                },
                VertexAttribDefinition {
                    // normals
                    number_of_components: 3,
                    offset_of_first_element: 3,
                },
                VertexAttribDefinition {
                    // texcoords
                    number_of_components: 2,
                    offset_of_first_element: 6,
                },
                VertexAttribDefinition {
                    // lightmap_coord
                    number_of_components: 2,
                    offset_of_first_element: 8,
                },
                VertexAttribDefinition {
                    // tile color coordinate
                    number_of_components: 2,
                    offset_of_first_element: 10,
                },
            ],
        );
        map_render_data.ground_walkability_mesh = VertexArray::new_static(
            gl,
            MyGlEnum::TRIANGLES,
            ground_result.ground_walkability_mesh,
            vec![VertexAttribDefinition {
                number_of_components: 3,
                offset_of_first_element: 0,
            }],
        );
        map_render_data.ground_walkability_mesh2 = VertexArray::new_static(
            gl,
            MyGlEnum::TRIANGLES,
            ground_result.ground_walkability_mesh2,
            vec![VertexAttribDefinition {
                number_of_components: 3,
                offset_of_first_element: 0,
            }],
        );
        map_render_data.ground_walkability_mesh3 = VertexArray::new_static(
            gl,
            MyGlEnum::TRIANGLES,
            ground_result.ground_walkability_mesh3,
            vec![VertexAttribDefinition {
                number_of_components: 3,
                offset_of_first_element: 0,
            }],
        );
        map_render_data.texture_atlas = ground_result.texture_atlas;
        map_render_data.tile_color_texture = ground_result.tile_color_texture;
        map_render_data.lightmap_texture = ground_result.lightmap_texture;
        log::info!(
            "load ground: {} textures have been loaded",
            reserved_textures.len()
        );
        log::info!("load ground: {} Unused texture slot", texture_id_pool.len());
        AssetLoader::set_reserved_textures(gl, asset_db, reserved_textures)
    }

    fn process_load_models_response(
        gl: &Gl,
        asset_db: &mut AssetDatabase,
        map_render_data: &mut MapRenderData,
        models: HashMap<String, ModelLoadingData>,
        mut model_instances: Vec<ModelInstance>,
        reserved_textures: Vec<ReservedTexturedata>,
        texture_id_pool: Vec<TextureId>,
        model_id_pool: Vec<usize>,
    ) -> () {
        log::info!("{} Models have been loaded", model_id_pool.len());
        log::info!(
            "{} Model textures have been loaded",
            reserved_textures.len()
        );
        log::info!("{} Unused texture slot", texture_id_pool.len());

        models.into_iter().for_each(|(model_name, model)| {
            let model_id = model.model_id;
            let model_render_data = AssetLoader::allocate_vbo(gl, model);

            asset_db.fill_bulk_reserved_model_slot(model_id, model_render_data, model_name);
        });
        AssetLoader::set_reserved_textures(gl, asset_db, reserved_textures);
        AssetLoader::create_and_set_half_lamp_models(asset_db, &mut model_instances);
        map_render_data.model_instances = model_instances;
    }

    fn allocate_vbo(gl: &Gl, model: ModelLoadingData) -> ModelRenderData {
        let same_node_faces: Vec<Vec<SameTextureNodeFaces>> = model
            .data_for_rendering_full_model
            .into_iter()
            .map(|it| {
                it.into_iter()
                    .map(|it| {
                        SameTextureNodeFaces {
                            vao: VertexArray::new_static(
                                gl,
                                MyGlEnum::TRIANGLES,
                                it.mesh,
                                vec![
                                    VertexAttribDefinition {
                                        number_of_components: 3,
                                        offset_of_first_element: 0,
                                    },
                                    VertexAttribDefinition {
                                        // normal
                                        number_of_components: 3,
                                        offset_of_first_element: 3,
                                    },
                                    VertexAttribDefinition {
                                        // uv
                                        number_of_components: 2,
                                        offset_of_first_element: 6,
                                    },
                                ],
                            ),
                            texture: it.texture,
                            texture_name: it.texture_name,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        return ModelRenderData {
            bounding_box: model.bbox,
            alpha: model.alpha,
            model: same_node_faces,
        };
    }

    fn create_and_set_half_lamp_models(
        asset_db: &mut AssetDatabase,
        model_instances: &mut Vec<ModelInstance>,
    ) {
        // remove the the upper half of lamps on which Guards are standing
        /*if map_name == "prontera"*/
        {
            let lamp_name = "ÇÁ·ÐÅ×¶ó\\ÈÖÀå°¡·Îµî.rsm";
            let model_index = asset_db.get_model_index(lamp_name);
            let model = asset_db.get_model(model_index);
            let new_model = ModelRenderData {
                bounding_box: model.bounding_box.clone(),
                alpha: 255,
                model: model
                    .model
                    .iter()
                    .map(|m| {
                        m.iter()
                            .filter(|m| {
                                m.texture_name.ends_with("stone-down.bmp")
                                    || m.texture_name.ends_with("STONE-UP.BMP")
                            })
                            .map(|m| m.clone())
                            .collect()
                    })
                    .collect(),
            };
            asset_db.register_model("half_lamp", new_model);
            let new_model_index = asset_db.get_model_index("half_lamp");
            // RIGHT TEAM GUARDS
            // middle final 4 guards on lamps
            model_instances[453].asset_db_model_index = new_model_index;
            model_instances[454].asset_db_model_index = new_model_index;
            model_instances[455].asset_db_model_index = new_model_index;
            model_instances[456].asset_db_model_index = new_model_index;
            // top, guard alone on lamp
            model_instances[695].asset_db_model_index = new_model_index;
            // top, two guards on lamps
            model_instances[549].asset_db_model_index = new_model_index;
            model_instances[550].asset_db_model_index = new_model_index;
            // LEFT TEAM GUARDS
            // middle final 4 guards on lamps
            model_instances[457].asset_db_model_index = new_model_index;
            model_instances[458].asset_db_model_index = new_model_index;
            model_instances[459].asset_db_model_index = new_model_index;
            model_instances[460].asset_db_model_index = new_model_index;
            // top, guard alone on lamp
            model_instances[712].asset_db_model_index = new_model_index;
            // top, two guards on lamps
            model_instances[536].asset_db_model_index = new_model_index;
            model_instances[537].asset_db_model_index = new_model_index;
        }
    }

    fn set_reserved_textures(
        gl: &Gl,
        asset_db: &mut AssetDatabase,
        reserved_textures: Vec<ReservedTexturedata>,
    ) -> () {
        for reserved_texture in reserved_textures.into_iter() {
            let sdl_surface = unsafe {
                sdl2::surface::Surface::from_ll(&mut *reserved_texture.raw_sdl_surface.0)
            };
            let gl_texture = AssetLoader::create_texture_from_surface_inner(
                gl,
                sdl_surface,
                reserved_texture.minmag,
            );
            asset_db.fill_bulk_reserved_texture_slot(
                reserved_texture.texture_id,
                gl_texture,
                reserved_texture.name,
            );
        }
    }

    pub fn new<P: AsRef<Path> + Clone>(
        paths: &[P],
    ) -> Result<AssetLoader<'static>, std::io::Error> {
        let (to_main_thread, from_2nd_thread) = channel::<FromBackgroundAssetLoaderMsg>();
        let (to_2nd_thread, from_main_thread) = channel::<ToBackgroundAssetLoaderMsg>();
        let path_str: Vec<String> = paths
            .iter()
            .map(|path| path.as_ref().to_str().unwrap().to_owned())
            .collect();

        let entries = if let Ok(mut cache_file) = File::open("grf.cache") {
            let count = cache_file.read_u32::<LittleEndian>().unwrap() as usize;
            let mut entries = HashMap::with_capacity(count);
            loop {
                let len = cache_file.read_u16::<LittleEndian>();
                if len.is_err() {
                    break;
                }
                let mut name = String::from_utf8(vec![b'X'; len.unwrap() as usize]).unwrap();
                unsafe {
                    cache_file.read_exact(name.as_bytes_mut()).expect("");
                }
                let grf_index = cache_file.read_u8().unwrap() as usize;
                let entry = GrfEntry {
                    pack_size: cache_file.read_u32::<LittleEndian>().unwrap(),
                    length_aligned: cache_file.read_u32::<LittleEndian>().unwrap(),
                    real_size: cache_file.read_u32::<LittleEndian>().unwrap(),
                    typ: cache_file.read_u8().unwrap(),
                    offset: cache_file.read_u32::<LittleEndian>().unwrap(),
                };
                entries.insert(name, (grf_index, entry));
            }
            entries
        } else {
            let readers: Result<Vec<BinaryReader>, std::io::Error> = paths
                .iter()
                .enumerate()
                .map(|(_i, path)| BinaryReader::new(path.clone()))
                .collect();
            match readers {
                Err(e) => return Err(e),
                Ok(readers) => {
                    let entries: HashMap<String, (usize, GrfEntry)> = readers
                        .into_iter()
                        .enumerate()
                        .map(|(file_index, buf)| {
                            AssetLoader::read_grf_entries(paths, file_index, buf)
                        })
                        .flatten()
                        .collect();

                    match File::create("grf.cache") {
                        Ok(mut cache_file) => {
                            log::info!(">>> Cache grf file content");
                            cache_file
                                .write_u32::<LittleEndian>(entries.len() as u32)
                                .unwrap();
                            for (filename, (grf_index, grf_entry)) in entries.iter() {
                                cache_file
                                    .write_u16::<LittleEndian>(filename.len() as u16)
                                    .unwrap();
                                cache_file.write(filename.as_bytes()).unwrap();
                                cache_file.write_u8(*grf_index as u8).unwrap();
                                cache_file
                                    .write_u32::<LittleEndian>(grf_entry.pack_size)
                                    .unwrap();
                                cache_file
                                    .write_u32::<LittleEndian>(grf_entry.length_aligned)
                                    .unwrap();
                                cache_file
                                    .write_u32::<LittleEndian>(grf_entry.real_size)
                                    .unwrap();
                                cache_file.write_u8(grf_entry.typ).unwrap();
                                cache_file
                                    .write_u32::<LittleEndian>(grf_entry.offset)
                                    .unwrap();
                            }
                            log::info!("<<< Cache grf file content");
                        }
                        Err(e) => {
                            log::warn!("Failed to create grf cache file: {}", e);
                        }
                    }
                    entries
                }
            }
        };
        let cloned_path_str = path_str.clone();
        let cloned_entries = entries.clone();
        std::thread::spawn(move || {
            BackgroundAssetLoader::new(
                to_main_thread,
                from_main_thread,
                cloned_path_str,
                cloned_entries,
            )
            .run();
        });
        Ok(AssetLoader {
            to_2nd_thread,
            paths: path_str,
            entries,
            from_2nd_thread,
        })
    }

    fn read_grf_entries<P: AsRef<Path> + Clone>(
        paths: &[P],
        file_index: usize,
        mut buf: BinaryReader,
    ) -> HashMap<String, (usize, GrfEntry)> {
        log::info!(
            "Loading {}",
            paths[file_index].as_ref().to_str().unwrap_or("")
        );
        let signature = buf.string(15);
        let _key = buf.string(15);
        let file_table_offset = buf.next_u32();
        let skip = buf.next_u32();
        let file_count = buf.next_u32() - (skip + 7);
        let version = buf.next_u32();

        if signature != "Master of Magic" {
            panic!("Incorrect signature: {}", signature);
        }

        if version != 0x200 {
            panic!("Incorrect version: {}", version);
        }

        buf.skip(file_table_offset);
        let pack_size = buf.next_u32();
        let real_size = buf.next_u32();
        let data = buf.next(pack_size);
        let mut out = Vec::<u8>::with_capacity(real_size as usize);
        let mut decoder = libflate::zlib::Decoder::new(data.as_slice()).unwrap();
        std::io::copy(&mut decoder, &mut out).unwrap();

        let mut table_reader = BinaryReader::from_vec(out);
        let entries: HashMap<String, (usize, GrfEntry)> = (0..file_count)
            .map(|_i| {
                let mut filename = String::new();
                loop {
                    let ch = table_reader.next_u8();
                    if ch == 0 {
                        break;
                    }
                    filename.push(ch as char);
                }
                let entry = GrfEntry {
                    pack_size: table_reader.next_u8() as u32
                        | (table_reader.next_u8() as u32).shl(8)
                        | (table_reader.next_u8() as u32).shl(16)
                        | (table_reader.next_u8() as u32).shl(24),
                    length_aligned: table_reader.next_u8() as u32
                        | (table_reader.next_u8() as u32).shl(8)
                        | (table_reader.next_u8() as u32).shl(16)
                        | (table_reader.next_u8() as u32).shl(24),
                    real_size: table_reader.next_u8() as u32
                        | (table_reader.next_u8() as u32).shl(8)
                        | (table_reader.next_u8() as u32).shl(16)
                        | (table_reader.next_u8() as u32).shl(24),
                    typ: table_reader.next_u8(),
                    offset: table_reader.next_u8() as u32
                        | (table_reader.next_u8() as u32).shl(8)
                        | (table_reader.next_u8() as u32).shl(16)
                        | (table_reader.next_u8() as u32).shl(24),
                };
                (filename.to_ascii_lowercase(), (file_index, entry))
            })
            .collect();
        entries
    }

    /// Clones backup surfaces, quite inefficient to share one surface...
    pub fn backup_surface(&self) -> sdl2::surface::Surface {
        let mut missing_texture =
            sdl2::surface::Surface::new(256, 256, PixelFormatEnum::RGBA8888).unwrap();
        missing_texture
            .fill_rect(None, sdl2::pixels::Color::RGB(255, 20, 147))
            .unwrap();
        missing_texture
    }

    pub fn get_entry_names(&self) -> Vec<String> {
        self.entries.keys().map(|it| it.to_owned()).collect()
    }

    pub fn get_content(&self, file_name: &str) -> Result<Vec<u8>, String> {
        return match &self.entries.get(&file_name.to_ascii_lowercase()) {
            Some((path_index, entry)) => {
                return Ok(AssetLoader::get_content2(
                    &self.paths[*path_index],
                    entry,
                    file_name,
                ));
            }
            None => Err(format!("No entry found in GRFs '{}'", file_name)),
        };
    }

    pub(super) fn get_content2(path_to_grf: &str, entry: &GrfEntry, file_name: &str) -> Vec<u8> {
        let mut f = File::open(path_to_grf).unwrap();

        let mut buf = Vec::<u8>::with_capacity(entry.length_aligned as usize);
        f.seek(SeekFrom::Start(
            entry.offset as u64 + GRF_HEADER_SIZE as u64,
        ))
        .expect(&format!("Could not get {}", file_name));
        f.take(entry.length_aligned as u64)
            .read_to_end(&mut buf)
            .expect(&format!("Could not get {}", file_name));

        if entry.typ & GRF_FILELIST_TYPE_ENCRYPT_MIXED != 0 {
            panic!("'{}' is encrypted!", file_name);
        } else if entry.typ & GRF_FILELIST_TYPE_ENCRYPT_HEADER != 0 {
            panic!("'{}' is encrypted!", file_name);
        }
        let mut decoder = libflate::zlib::Decoder::new(buf.as_slice()).unwrap();
        let mut out = Vec::<u8>::with_capacity(entry.real_size as usize);
        std::io::copy(&mut decoder, &mut out).unwrap();
        return out;
    }

    pub fn read_dir(&self, dir_name: &str) -> Vec<String> {
        self.get_entry_names()
            .into_iter()
            .filter(|it| it.starts_with(dir_name))
            .collect()
    }

    pub fn load_effect(
        &self,
        gl: &Gl,
        effect_name: &str,
        asset_db: &mut AssetDatabase,
    ) -> Result<StrFile, String> {
        let file_name = format!("data\\texture\\effect\\{}.str", effect_name);
        let content = self.get_content(&file_name)?;
        return Ok(StrFile::load(
            gl,
            &self,
            asset_db,
            BinaryReader::from_vec(content),
            effect_name,
        ));
    }

    pub fn load_map(&self, map_name: &str) -> Result<Rsw, String> {
        let file_name = format!("data\\{}.rsw", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Rsw::load(BinaryReader::from_vec(content)));
    }

    pub fn load_gat(&self, map_name: &str) -> Result<(Gat, Vec<BlockingRectangle>), String> {
        let file_name = format!("data\\{}.gat", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Gat::load(BinaryReader::from_vec(content), map_name));
    }

    pub fn start_loading_models(
        &self,
        gl: &Gl,
        rsw_model_instances: Vec<RswModelInstance>,
        asset_db: &mut AssetDatabase,
        map_width: u32,
        map_height: u32,
    ) {
        self.to_2nd_thread
            .send(ToBackgroundAssetLoaderMsg::LoadModelPart1 {
                model_id_pool: asset_db.reserve_model_slots(500),
                texture_id_pool: asset_db.reserve_texture_slots(gl, 500),
                rsw_model_instances,
                map_width,
                map_height,
            })
            .expect("");
    }

    pub fn load_wav(&self, path: &str) -> Result<sdl2::mixer::Chunk, String> {
        let buffer = self.get_content(path)?;
        let rwops = sdl2::rwops::RWops::from_bytes(buffer.as_slice())?;
        return rwops.load_wav();
    }

    pub fn load_sdl_surface(&self, path: &str) -> Result<sdl2::surface::Surface, String> {
        let buffer = self.get_content(path)?;
        return AssetLoader::load_sdl_surface2(buffer, path.ends_with(".tga"));
    }

    pub fn load_sdl_surface2(
        buffer: Vec<u8>,
        is_tga: bool,
    ) -> Result<sdl2::surface::Surface<'static>, String> {
        let rwops = sdl2::rwops::RWops::from_bytes(buffer.as_slice())?;
        let mut surface = if is_tga {
            rwops.load_tga()?
        } else {
            rwops.load()?
        };

        // I think it is an incorrect implementation in SDL rust lib.
        // Creating a new surface from an RWops keeps a reference to RWOPS,
        // which is a local variable and will be destroyed at the end of this function.
        // So the surface have to be copied.
        let mut optimized_surf = sdl2::surface::Surface::new(
            surface.width(),
            surface.height(),
            PixelFormatEnum::RGBA32,
        )?;
        surface
            .set_color_key(true, sdl2::pixels::Color::RGB(255, 0, 255))
            .unwrap();
        surface.blit(None, &mut optimized_surf, None)?;
        return Ok(optimized_surf);
    }

    pub fn create_texture_from_surface(
        gl: &Gl,
        name: &str,
        surface: sdl2::surface::Surface,
        min_mag: MyGlEnum,
        asset_db: &mut AssetDatabase,
    ) -> TextureId {
        let ret = AssetLoader::create_texture_from_surface_inner(gl, surface, min_mag);
        log::trace!("Texture was created: {}", name);
        return asset_db.register_texture(&name, ret);
    }

    pub fn create_texture_from_surface_inner(
        gl: &Gl,
        mut surface: sdl2::surface::Surface,
        min_mag: MyGlEnum,
    ) -> GlTexture {
        let surface = if surface.pixel_format_enum() != PixelFormatEnum::RGBA32 {
            let mut optimized_surf = sdl2::surface::Surface::new(
                surface.width(),
                surface.height(),
                PixelFormatEnum::RGBA32,
            )
            .unwrap();
            surface
                .set_color_key(true, sdl2::pixels::Color::RGB(255, 0, 255))
                .unwrap();
            surface.blit(None, &mut optimized_surf, None).unwrap();
            optimized_surf
        } else {
            surface
        };
        return AssetLoader::create_gl_texture(
            gl,
            surface.width() as i32,
            surface.height() as i32,
            surface.without_lock().unwrap().as_ptr() as *const c_void,
            min_mag,
        );
    }

    fn create_gl_texture(
        gl: &Gl,
        w: i32,
        h: i32,
        ptr: *const c_void,
        min_mag: MyGlEnum,
    ) -> GlTexture {
        let mut texture_native_id = GlNativeTextureId(0);
        unsafe {
            gl.gen_textures(1, &mut texture_native_id.0);
            gl.bind_texture(MyGlEnum::TEXTURE_2D, texture_native_id);
            gl.tex_image2d(
                MyGlEnum::TEXTURE_2D,
                0,                     // Pyramid level (for mip-mapping) - 0 is the top level
                MyGlEnum::RGBA as i32, // Internal colour format to convert to
                w,
                h,
                0,              // border
                MyGlEnum::RGBA, // Input image format (i.e. GL_RGB, GL_RGBA, GL_BGR etc.)
                MyGlEnum::UNSIGNED_BYTE,
                ptr,
            );

            gl.tex_parameteri(
                MyGlEnum::TEXTURE_2D,
                MyGlEnum::TEXTURE_MIN_FILTER,
                min_mag as i32,
            );
            gl.tex_parameteri(
                MyGlEnum::TEXTURE_2D,
                MyGlEnum::TEXTURE_MAG_FILTER,
                min_mag as i32,
            );
            gl.tex_parameteri(
                MyGlEnum::TEXTURE_2D,
                MyGlEnum::TEXTURE_WRAP_S,
                MyGlEnum::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameteri(
                MyGlEnum::TEXTURE_2D,
                MyGlEnum::TEXTURE_WRAP_T,
                MyGlEnum::CLAMP_TO_EDGE as i32,
            );
            gl.generate_mipmap(MyGlEnum::TEXTURE_2D);
        }
        return GlTexture::new(gl, texture_native_id, w, h);
    }

    pub fn start_loading_texture(
        &self,
        gl: &Gl,
        texture_path: &str,
        min_mag: MyGlEnum,
        asset_db: &mut AssetDatabase,
    ) -> Result<TextureId, String> {
        let texture_id = asset_db.reserve_texture_slot(gl, texture_path);
        if let Some((grf_index, entry)) = self.entries.get(&texture_path.to_ascii_lowercase()) {
            self.to_2nd_thread
                .send(ToBackgroundAssetLoaderMsg::LoadTexture {
                    texture_id,
                    minmag: min_mag,
                    grf_entry: (*entry).clone(),
                    grf_index: *grf_index,
                    file_name_for_debug: texture_path.to_string(),
                })
                .expect("");
            return Ok(texture_id);
        } else {
            return Err(format!("No entry found in GRFs '{}'", texture_path));
        }
    }
}
