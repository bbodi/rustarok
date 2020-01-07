use encoding::types::Encoding;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct BinaryReader {
    buf: Vec<u8>,
    index: usize,
}

impl BinaryReader {
    pub fn new<P: AsRef<Path> + Clone>(path: P) -> Result<BinaryReader, std::io::Error> {
        let mut reader = BinaryReader {
            buf: Vec::new(),
            index: 0,
        };
        let _read = File::open(path)?.read_to_end(&mut reader.buf)?;
        return Ok(reader);
    }

    pub fn from_vec(vec: Vec<u8>) -> BinaryReader {
        BinaryReader { buf: vec, index: 0 }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf[self.index..]
    }

    pub fn tell(&self) -> usize {
        self.index
    }

    pub fn seek(&mut self, index: usize) {
        self.index = index;
    }

    pub fn get_u8(&self, index: usize) -> u8 {
        return self.buf[index];
    }

    #[inline]
    pub fn as_u16(buf: &[u8], index: usize) -> u16 {
        unsafe { *(buf.as_ptr().offset(index as isize) as *const u16) }
    }

    pub fn get_u16(&self, index: usize) -> u16 {
        let result = BinaryReader::as_u16(&self.buf, index);
        return result;
    }

    pub fn get_slice(&self, index: usize, size: usize) -> &[u8] {
        return &self.buf[index..index + size];
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn next_u8(&mut self) -> u8 {
        self.index += 1;
        self.buf[self.index - 1]
    }

    pub fn next_f32(&mut self) -> f32 {
        let result = unsafe { *(self.buf.as_ptr().offset(self.index as isize) as *const f32) };
        self.index += 4;
        return result;
    }

    pub fn next_i32(&mut self) -> i32 {
        let result = unsafe { *(self.buf.as_ptr().offset(self.index as isize) as *const i32) };
        self.index += 4;
        return result;
    }

    pub fn next_u32(&mut self) -> u32 {
        let result = unsafe { *(self.buf.as_ptr().offset(self.index as isize) as *const u32) };
        self.index += 4;
        return result;
    }

    pub fn next_u16(&mut self) -> u16 {
        let result = unsafe { *(self.buf.as_ptr().offset(self.index as isize) as *const u16) };
        self.index += 2;
        return result;
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
            .decode(&bytes, encoding::DecoderTrap::Strict)
            .unwrap();
        decoded
    }

    pub fn skip(&mut self, size: u32) {
        self.index += size as usize;
    }

    pub fn next(&mut self, size: u32) -> &[u8] {
        let from = self.index;
        self.index += size as usize;
        &self.buf[from..self.index]
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.index..]
    }

    pub fn as_slice_from(&self, from: usize) -> &[u8] {
        &self.buf[from..]
    }
}
