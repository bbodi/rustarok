use crate::common::BinaryReader;

enum CellType {
    None = 1 << 0,
    Walkable = 1 << 1,
    Water = 1 << 2,
    Snipable = 1 << 3,
}

// GroundAltitude
#[derive(Debug)]
pub struct Gat {
    width: u32,
    height: u32,
    cells: Box<[f32]>,
    version: f32,
}

static TYPE_TABLE: [u8; 7] = [
    CellType::Walkable as u8 | CellType::Snipable as u8,                  // walkable ground
    CellType::None as u8,                                          // non-walkable ground
    CellType::Walkable as u8 | CellType::Snipable as u8,                  // ???
    CellType::Walkable as u8 | CellType::Snipable as u8 | CellType::Water as u8, // walkable water
    CellType::Walkable as u8 | CellType::Snipable as u8,                  // ???
    CellType::Snipable as u8,                                      // gat (snipable)
    CellType::Walkable as u8 | CellType::Snipable as u8                   // ???
];

impl Gat {
    pub fn load(mut buf: BinaryReader) -> Gat {
        let header = buf.string(4);
        if header != "GRAT" {
            panic!("Invalig GAT header: {}", header);
        }

        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        let width = buf.next_u32();
        let height = buf.next_u32();
        println!("version: {}", version);
        println!("version: {}", width);
        println!("version: {}", height);
        let mut cells: Box<[f32]> = vec![0f32; (width * height * 5) as usize].into_boxed_slice();
        for i in 0.. (width * height) as usize {
            cells[i * 5 + 0] = buf.next_f32() * 0.2;
            cells[i * 5 + 1] = buf.next_f32() * 0.2;
            cells[i * 5 + 2] = buf.next_f32() * 0.2;
            cells[i * 5 + 3] = buf.next_f32() * 0.2;
            cells[i * 5 + 4] = TYPE_TABLE[buf.next_u32() as usize] as f32;
        }
        Gat {
            width,
            height,
            cells,
            version,
        }
    }
}