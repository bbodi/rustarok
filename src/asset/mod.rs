use crate::asset::act::ActionFile;
use crate::asset::database::AssetDatabase;
use crate::asset::gat::Gat;
use crate::asset::gnd::Gnd;
use crate::asset::rsm::Rsm;
use crate::asset::rsw::Rsw;
use crate::asset::spr::{SpriteFile, SpriteTexture};
use crate::asset::str::StrFile;
use crate::video::{GlTexture, GlTextureIndex};
use encoding::types::Encoding;
use encoding::DecoderTrap;
use libflate::zlib::Decoder;
use sdl2::image::ImageRWops;
use sdl2::mixer::LoaderRWops;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::io::SeekFrom;
use std::ops::*;
use std::path::Path;

pub mod act;
pub mod database;
pub mod gat;
pub mod gnd;
pub mod rsm;
pub mod rsw;
pub mod spr;
pub mod str;

const GRF_HEADER_SIZE: usize = 15 + 15 + 4 * 4;

// entry is a file
#[allow(dead_code)]
const GRF_FILELIST_TYPE_FILE: u8 = 0x01;

// encryption mode 0 (header DES + periodic DES/shuffle)
const GRF_FILELIST_TYPE_ENCRYPT_MIXED: u8 = 0x02;

// encryption mode 1 (header DES only)
const GRF_FILELIST_TYPE_ENCRYPT_HEADER: u8 = 0x04;

pub struct AssetLoader {
    entries: HashMap<String, (usize, GrfEntry)>,
    paths: Vec<String>,
}

#[derive(Debug)]
pub struct GrfEntry {
    pack_size: u32,
    length_aligned: u32,
    real_size: u32,
    typ: u8,
    offset: u32,
}

impl AssetLoader {
    pub fn new<P: AsRef<Path> + Clone>(paths: &[P]) -> Result<AssetLoader, std::io::Error> {
        let readers: Result<Vec<BinaryReader>, std::io::Error> = paths
            .iter()
            .enumerate()
            .map(|(_i, path)| BinaryReader::new(path.clone()))
            .collect();
        return match readers {
            Err(e) => Err(e),
            Ok(readers) => {
                let entries: HashMap<String, (usize, GrfEntry)> = readers
                    .into_iter()
                    .enumerate()
                    .map(|(file_index, mut buf)| {
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
                        let mut decoder = Decoder::new(data.as_slice()).unwrap();
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
                    })
                    .flatten()
                    .collect();
                Ok(AssetLoader {
                    paths: paths
                        .iter()
                        .map(|path| path.as_ref().to_str().unwrap().to_owned())
                        .collect(),
                    entries,
                })
            }
        };
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

    pub fn exists(&self, file_name: &str) -> bool {
        self.entries.get(file_name).is_some()
    }

    pub fn get_entry_names(&self) -> Vec<String> {
        self.entries.keys().map(|it| it.to_owned()).collect()
    }

    pub fn get_content(&self, file_name: &str) -> Result<Vec<u8>, String> {
        return match &self.entries.get(&file_name.to_ascii_lowercase()) {
            Some((path_index, entry)) => {
                let mut f = File::open(&self.paths[*path_index]).unwrap();

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
                let mut decoder = Decoder::new(buf.as_slice()).unwrap();
                let mut out = Vec::<u8>::with_capacity(entry.real_size as usize);
                std::io::copy(&mut decoder, &mut out).unwrap();
                return Ok(out);
            }
            None => Err(format!("Could not load '{}'", file_name)),
        };
    }

    pub fn read_dir(&self, dir_name: &str) -> Vec<String> {
        self.get_entry_names()
            .into_iter()
            .filter(|it| it.starts_with(dir_name))
            .collect()
    }

    pub fn load_effect(
        &self,
        effect_name: &str,
        asset_database: &mut AssetDatabase,
    ) -> Result<StrFile, String> {
        let file_name = format!("data\\texture\\effect\\{}.str", effect_name);
        let content = self.get_content(&file_name)?;
        return Ok(StrFile::load(
            &self,
            asset_database,
            BinaryReader::from_vec(content),
            effect_name,
        ));
    }

    pub fn load_map(&self, map_name: &str) -> Result<Rsw, String> {
        let file_name = format!("data\\{}.rsw", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Rsw::load(BinaryReader::from_vec(content)));
    }

    pub fn load_gat(&self, map_name: &str) -> Result<Gat, String> {
        let file_name = format!("data\\{}.gat", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Gat::load(BinaryReader::from_vec(content), map_name));
    }

    pub fn load_model(&self, model_name: &str) -> Result<Rsm, String> {
        let file_name = format!("data\\model\\{}", model_name);
        let content = self.get_content(&file_name)?;
        return Ok(Rsm::load(BinaryReader::from_vec(content)));
    }

    pub fn load_gnd(
        &self,
        map_name: &str,
        water_level: f32,
        water_height: f32,
    ) -> Result<Gnd, String> {
        let file_name = format!("data\\{}.gnd", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Gnd::load(
            BinaryReader::from_vec(content),
            water_level,
            water_height,
        ));
    }

    pub fn load_wav(&self, path: &str) -> Result<sdl2::mixer::Chunk, String> {
        let buffer = self.get_content(path)?;
        let rwops = sdl2::rwops::RWops::from_bytes(buffer.as_slice())?;
        return rwops.load_wav();
    }

    fn load_sdl_surface(&self, path: &str) -> Result<sdl2::surface::Surface, String> {
        let buffer = self.get_content(path)?;
        let rwops = sdl2::rwops::RWops::from_bytes(buffer.as_slice())?;
        let mut surface = rwops.load()?;

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
        name: &str,
        surface: sdl2::surface::Surface,
        min_mag: u32,
        asset_db: &mut AssetDatabase,
    ) -> GlTexture {
        let ret = AssetLoader::create_texture_from_surface_inner(surface, min_mag);
        asset_db.register_texture(&name, &[&ret]);
        log::trace!("Texture was created loaded: {}", name);
        ret
    }

    pub fn create_texture_from_data(
        name: &str,
        data: &Vec<u8>,
        width: i32,
        height: i32,
        asset_db: &mut AssetDatabase,
    ) -> GlTexture {
        let mut texture_id = GlTextureIndex(0);
        unsafe {
            gl::GenTextures(1, &mut texture_id.0);
            log::debug!("Texture from_data {}", texture_id.0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id.0);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,               // Pyramid level (for mip-mapping) - 0 is the top level
                gl::RGBA as i32, // Internal colour format to convert to
                width,
                height,
                0,        // border
                gl::RGBA, // Input image format (i.e. GL_RGB, GL_RGBA, GL_BGR etc.)
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const gl::types::GLvoid,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        let texture = GlTexture::new(texture_id, width, height);

        asset_db.register_texture(&name, &[&texture]);
        return texture;
    }

    pub fn load_texture(
        &self,
        path: &str,
        min_mag: u32,
        asset_db: &mut AssetDatabase,
    ) -> Result<GlTexture, String> {
        let surface = self.load_sdl_surface(&path);
        return surface.map(|surface| {
            AssetLoader::create_texture_from_surface(path, surface, min_mag, asset_db)
        });
    }

    pub fn create_texture_from_surface_inner(
        mut surface: sdl2::surface::Surface,
        min_mag: u32,
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
        let mut texture_id = GlTextureIndex(0);
        unsafe {
            gl::GenTextures(1, &mut texture_id.0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id.0);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,               // Pyramid level (for mip-mapping) - 0 is the top level
                gl::RGBA as i32, // Internal colour format to convert to
                surface.width() as i32,
                surface.height() as i32,
                0,               // border
                gl::RGBA as u32, // Input image format (i.e. GL_RGB, GL_RGBA, GL_BGR etc.)
                gl::UNSIGNED_BYTE,
                surface.without_lock().unwrap().as_ptr() as *const gl::types::GLvoid,
            );

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min_mag as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, min_mag as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        GlTexture::new(texture_id, surface.width() as i32, surface.height() as i32)
    }

    pub fn load_spr_and_act(
        &self,
        path: &str,
        asset_db: &mut AssetDatabase,
    ) -> Result<SpriteResource, String> {
        let content = self.get_content(&format!("{}.spr", path))?;
        let frames: Vec<SpriteTexture> = SpriteFile::load(BinaryReader::from_vec(content))
            .frames
            .into_iter()
            .map(|frame| SpriteTexture::from(frame))
            .collect();
        let content = self.get_content(&format!("{}.act", path))?;
        let action = ActionFile::load(BinaryReader::from_vec(content));

        let s = frames.iter().map(|it| &it.texture).collect::<Vec<_>>();
        asset_db.register_texture(&path.to_string(), s.as_slice());

        return Ok(SpriteResource {
            action,
            textures: frames,
        });
    }
}

#[derive(Clone)]
pub struct SpriteResource {
    pub action: ActionFile,
    pub textures: Vec<SpriteTexture>,
}

struct BinaryReader {
    buf: Vec<u8>,
    index: usize,
}

impl BinaryReader {
    pub fn tell(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn new<P: AsRef<Path> + Clone>(path: P) -> Result<BinaryReader, std::io::Error> {
        let mut buf = BinaryReader {
            buf: Vec::new(),
            index: 0,
        };
        let _read = File::open(path)?.read_to_end(&mut buf.buf)?;
        return Ok(buf);
    }

    pub fn from_vec(vec: Vec<u8>) -> BinaryReader {
        BinaryReader { buf: vec, index: 0 }
    }

    pub fn next_u8(&mut self) -> u8 {
        self.index += 1;
        self.buf[self.index - 1]
    }

    pub fn next_f32(&mut self) -> f32 {
        let bytes = [
            self.buf[self.index],
            self.buf[self.index + 1],
            self.buf[self.index + 2],
            self.buf[self.index + 3],
        ];
        self.index += 4;
        unsafe { std::mem::transmute(bytes) }
    }

    pub fn next_i32(&mut self) -> i32 {
        let bytes = [
            self.buf[self.index],
            self.buf[self.index + 1],
            self.buf[self.index + 2],
            self.buf[self.index + 3],
        ];
        self.index += 4;
        i32::from_le_bytes(bytes)
    }

    pub fn next_u32(&mut self) -> u32 {
        let bytes = [
            self.buf[self.index],
            self.buf[self.index + 1],
            self.buf[self.index + 2],
            self.buf[self.index + 3],
        ];
        self.index += 4;
        u32::from_le_bytes(bytes)
    }

    pub fn next_u16(&mut self) -> u16 {
        let bytes = [self.buf[self.index], self.buf[self.index + 1]];
        self.index += 2;
        u16::from_le_bytes(bytes)
    }

    pub fn string(&mut self, max_len: u32) -> String {
        let i = self.index;
        self.index += max_len as usize;
        let bytes: Vec<u8> = self
            .buf
            .iter()
            .skip(i)
            .take(max_len as usize)
            .take_while(|b| **b != 0)
            .map(|b| *b)
            .collect();
        let decoded = encoding::all::WINDOWS_1252
            .decode(&bytes, DecoderTrap::Strict)
            .unwrap();
        //        return String::from_utf8(encoding::all::UTF_8.encode(&decoded, EncoderTrap::Strict).unwrap()).unwrap();
        decoded
    }

    pub fn skip(&mut self, size: u32) {
        self.index += size as usize;
    }

    pub fn next(&mut self, size: u32) -> Vec<u8> {
        let from = self.index;
        self.index += size as usize;
        self.buf[from..self.index].to_vec()
    }
}
