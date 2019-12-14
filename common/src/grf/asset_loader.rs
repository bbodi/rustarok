use crate::grf::binary_reader::BinaryReader;
use crate::grf::gat::{BlockingRectangle, Gat};
use crate::grf::GrfEntry;
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::ops::Shl;
use std::path::Path;

const GRF_HEADER_SIZE: usize = 15 + 15 + 4 * 4;

// entry is a file
#[allow(dead_code)]
const GRF_FILELIST_TYPE_FILE: u8 = 0x01;

// encryption mode 0 (header DES + periodic DES/shuffle)
const GRF_FILELIST_TYPE_ENCRYPT_MIXED: u8 = 0x02;

// encryption mode 1 (header DES only)
const GRF_FILELIST_TYPE_ENCRYPT_HEADER: u8 = 0x04;

#[derive(Clone)]
pub struct CommonAssetLoader {
    entries: HashMap<String, (usize, GrfEntry)>,
    paths: Vec<String>,
}

impl<'a> CommonAssetLoader {
    pub fn new<P: AsRef<Path> + Clone>(paths: &[P]) -> Result<CommonAssetLoader, std::io::Error> {
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
                            CommonAssetLoader::read_grf_entries(paths, file_index, buf)
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
        Ok(CommonAssetLoader {
            paths: path_str,
            entries,
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
        let mut decoder = libflate::zlib::Decoder::new(data).unwrap();
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

    pub fn get_entry_names(&self) -> Vec<String> {
        self.entries.keys().map(|it| it.to_owned()).collect()
    }

    pub fn exists(&self, file_name: &str) -> bool {
        self.entries.get(file_name).is_some()
    }

    pub fn get_content(&self, file_name: &str) -> Result<Vec<u8>, String> {
        return match &self.entries.get(&file_name.to_ascii_lowercase()) {
            Some((path_index, entry)) => {
                return Ok(CommonAssetLoader::get_content2(
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

    pub fn load_gat(&self, map_name: &str) -> Result<(Gat, Vec<BlockingRectangle>), String> {
        let file_name = format!("data\\{}.gat", map_name);
        let content = self.get_content(&file_name)?;
        return Ok(Gat::load(BinaryReader::from_vec(content), map_name));
    }
}
