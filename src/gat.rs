use crate::common::BinaryReader;
use std::cmp::max;

pub enum CellType {
    None = 1 << 0,
    Walkable = 1 << 1,
    Water = 1 << 2,
    Snipable = 1 << 3,
}

#[derive(Debug)]
pub struct GatCell {
    pub cells: [f32; 4],
    pub cell_type: u8,
}

// GroundAltitude
#[derive(Debug)]
pub struct Gat {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<GatCell>,
    pub version: f32,
    pub rectangles: Vec<BlockingRectangle>,
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

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct BlockingRectangle {
    pub area: i32,
    pub start_x: i32,
    pub bottom: i32,
    pub width: i32,
    pub height: i32,
}

impl Gat {
    pub fn load(mut buf: BinaryReader) -> Gat {
        let header = buf.string(4);
        if header != "GRAT" {
            panic!("Invalig GAT header: {}", header);
        }

        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        let width = buf.next_u32();
        let height = buf.next_u32();
        let cells: Vec<GatCell> = (0..width * height).map(|i| {
            GatCell {
                cells: [buf.next_f32() * 0.2,
                    buf.next_f32() * 0.2,
                    buf.next_f32() * 0.2,
                    buf.next_f32() * 0.2],
                cell_type: TYPE_TABLE[buf.next_u32() as usize],
            }
        }).collect();
        let rectangles = Gat::shit0(&cells, width as usize, height as usize);
        Gat {
            width,
            height,
            cells,
            version,
            rectangles,
        }
    }

    fn shit0(cells: &[GatCell], width: usize, height: usize) -> Vec<BlockingRectangle> {
        let mut non_walkable_cells: Vec<bool> = cells.iter().map(|c| {
            c.cell_type & CellType::Walkable as u8 == 0
        }).collect();
        dbg!(non_walkable_cells.iter().filter(|&&it| it).count());

        let mut rectangles: Vec<BlockingRectangle> = Vec::new();
        loop {
            let largest_rect = {
                let row_areas = Gat::shit(&non_walkable_cells, width, height);
                row_areas.iter().max_by(|x, y| {
                    x.area.cmp(&y.area)
                }).unwrap().clone()
            };
            // remove the max rectangle
            let start_y = largest_rect.bottom - (largest_rect.height-1);
            for x in largest_rect.start_x..=largest_rect.start_x + largest_rect.width {
                for y in start_y..=largest_rect.bottom {
                    let i = (y as usize * width) + x as usize;
                    non_walkable_cells[i] = false;
                }
            }
            let area = largest_rect.area;
            rectangles.push(largest_rect);
            if area == 1 {
                // all  the rectangles are a unit tile
                for (i, non_walkable) in non_walkable_cells.iter().enumerate().filter(|(_i, &non_walkable)| non_walkable) {
                    let x = i % width;
                    let y = i / width;
                    rectangles.push(BlockingRectangle {
                        area: 1,
                        start_x: x as i32,
                        bottom: y as i32,
                        width: 1,
                        height: 1,
                    });
                }
                break;
            } else if area == 0 {
                break;
            }
        }
        dbg!(rectangles.len());
        return rectangles;
    }

    fn shit(cells: &[bool], width: usize, height: usize) -> Vec<BlockingRectangle> {
        // végigmenni minden soron, és egy számot rendelni a cellákhoz
        // kiszámolni az adott sor legnagyobb területű téglalapját
        // kiválasztani a egy cellát, h = cell_h
        // start = balra lépkedni, amig bal cella h >=, h = min(h, cell_h)
        // end = jobbra lépkedni, amig bal cella h >=, h = min(h, cell_h)
        // w = end - start; terület = w*h;, elmenteni ezt és a hozzá tartozó cellákat
        // a legnagyobb téglalap celláit eltávolítani a cellák listájából
        //  és ...
        // kezdődik előlről a művelet, amig a legnagyobb méretü cella 1 nem lesz
        // ekko minden cella 1 méretű

        let mut heights = vec![0; (width * height) as usize];
        let mut row_heights = Vec::<BlockingRectangle>::with_capacity(height);
        for (i, cell) in cells.iter().enumerate() {
            let x = i % width;
            let y = i / width;
            let prev_y: i32 = (i / width) as i32 - 1;

            if cells[i] {
                if y == 0 {
                    heights[i] = 1;
                } else {
                    heights[i] = heights[prev_y as usize * width + x] + 1;
                }
            }

            if (x + 1) % width == 0 && x > 1 { // row is ready
                let row = &heights[y * width..(y * width) + width];
                let (area, start_x, width, height) = Gat::shit2(row, width);
                row_heights.push(BlockingRectangle {
                    area,
                    start_x,
                    bottom: y as i32,
                    width,
                    height: height as i32,
                });
            }
        }

        return row_heights;
    }

    fn shit2(heights: &[usize], width: usize) -> (i32, i32, i32, usize) {
        let mut max_area = 0;
        let mut max_width = 0;
        let mut max_start_i = 0;
        let mut max_height = 0;
        for x in 0..width {
            let reference_bar_h = heights[x] as usize;
            if reference_bar_h == 0 {
                continue;
            }
            let mut left_i = (x as i32) - 1;
            while left_i >= 0 && heights[left_i as usize] >= reference_bar_h {
                left_i -= 1;
            }
            let mut right_i = (x as i32) + 1;
            while right_i < width as i32 && heights[right_i as usize] >= reference_bar_h {
                right_i += 1;
            }
            let bar_width = (right_i - 1) - (left_i + 1) + 1;
            let area = bar_width * reference_bar_h as i32;
            if area > max_area {
                max_area = area;
                max_start_i = left_i + 1;
                max_width = bar_width;
                max_height = reference_bar_h;
            }
        }
        return (max_area, max_start_i, max_width, max_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shit2() {
        assert_eq!(Gat::shit2(&[1, 1, 0, 0, 1, 0], 6), (2, 0, 2, 1));
        assert_eq!(Gat::shit2(&[1, 3, 2, 2, 3, 0], 6), (8, 1, 4, 2));
        assert_eq!(Gat::shit2(&[0, 0, 0, 0, 0, 0], 6), (0, 0, 0, 0));
    }

    #[test]
    fn test_shit() {
        let walkable = false;
        let non_walkable = true;
        let input = [
            non_walkable,
            non_walkable,
            walkable,
            walkable,
            non_walkable,
            walkable,
            //
            walkable,
            non_walkable,
            non_walkable,
            non_walkable,
            non_walkable,
            walkable,
            //
            non_walkable,
            non_walkable,
            non_walkable,
            non_walkable,
            non_walkable,
            walkable,
            //
            walkable,
            walkable,
            non_walkable,
            non_walkable,
            walkable,
            walkable,
        ];
        let expected_output = vec![
            BlockingRectangle {
                area: 2,
                start_x: 0,
                bottom: 0,
                width: 2,
                height: 1,
            },
            BlockingRectangle {
                area: 4,
                start_x: 1,
                bottom: 1,
                width: 4,
                height: 1,
            },
            BlockingRectangle {
                area: 8,
                start_x: 1,
                bottom: 2,
                width: 4,
                height: 2,
            },
            BlockingRectangle {
                area: 6,
                start_x: 2,
                bottom: 3,
                width: 2,
                height: 3,
            }
        ];
        assert_eq!(Gat::shit(&input, 6, 4), expected_output);
    }
}