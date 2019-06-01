use std::fs::File;
use std::string::FromUtf8Error;
use std::path::Path;
use std::io::Read;

use encoding;
use encoding::types::Encoding;
use encoding::{DecoderTrap, EncoderTrap};

pub struct BinaryReader {
    buf: Vec<u8>,
    index: usize,
}

pub fn init_vec<T, F>(size: u32, def: T, mut init_func: F) -> Vec<T>
    where T: Clone,
          F: FnMut(&mut T) -> ()
{
    let mut vec: Vec<T> = vec![def; size as usize];
    for i in 0..size as usize {
        init_func(&mut vec[i]);
    }
    vec
}

impl BinaryReader {



    pub fn tell(&self) -> usize { self.index }

    pub fn new<P: AsRef<Path>>(path: P) -> BinaryReader {
        let mut buf = BinaryReader {
            buf: Vec::new(),
            index: 0,
        };
        let _read = File::open(path).unwrap().read_to_end(&mut buf.buf);
        return buf;
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
        unsafe {
            std::mem::transmute(bytes)
        }
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
        let bytes = [
            self.buf[self.index],
            self.buf[self.index + 1],
        ];
        self.index += 2;
        u16::from_le_bytes(bytes)
    }

    pub fn string(&mut self, max_len: u32) -> String {
        let i = self.index;
        self.index += max_len as usize;
        let bytes: Vec<u8> = self.buf.iter()
            .skip(i)
            .take(max_len as usize)
            .take_while(|b| **b != 0)
            .map(|b| *b)
            .collect();
        let decoded = encoding::all::WINDOWS_1252.decode(&bytes, DecoderTrap::Strict).unwrap();
//        return String::from_utf8(encoding::all::UTF_8.encode(&decoded, EncoderTrap::Strict).unwrap()).unwrap();
        decoded
    }

    pub fn next(&mut self, size: u32) -> Vec<u8> {
        let from = self.index;
        self.index += size as usize;
        self.buf[from..self.index].to_vec()
    }
}
