pub mod asset_loader;
pub mod binary_reader;
pub mod gat;

#[derive(Debug, Clone)]
pub struct GrfEntry {
    pub pack_size: u32,
    pub length_aligned: u32,
    pub real_size: u32,
    pub typ: u8,
    pub offset: u32,
}
