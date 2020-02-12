// remove grf::gat from common

pub enum CellType {
    None = 1 << 0,
    Walkable = 1 << 1,
    Water = 1 << 2,
    Snipable = 1 << 3,
}

pub struct MapWalkingInfo {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<CellType>,
}

impl MapWalkingInfo {
    pub fn new() -> MapWalkingInfo {
        MapWalkingInfo {
            width: 0,
            height: 0,
            cells: vec![],
        }
    }
}
