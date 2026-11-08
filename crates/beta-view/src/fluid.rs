//! Pure Beta 1.7.3 liquid surface geometry.
//!
//! The input is deliberately a tiny neighbourhood, so this module has no world,
//! texture, or filesystem dependencies.  Vertex order follows the native
//! Tessellator submission order: top, then north, south, west, and east sides.

use crate::{ATLAS_SIZE, TILE_INSET, TILE_SIZE, ViewError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidCell {
    pub id: u8,
    pub metadata: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidNeighborhood {
    pub center: FluidCell,
    pub below: Option<FluidCell>,
    pub above: Option<FluidCell>,
    pub east: Option<FluidCell>,
    pub west: Option<FluidCell>,
    pub north: Option<FluidCell>,
    pub south: Option<FluidCell>,
    pub north_east: Option<FluidCell>,
    pub north_west: Option<FluidCell>,
    pub south_east: Option<FluidCell>,
    pub south_west: Option<FluidCell>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
}

// The Beta atlas uses the still-water tile for the top and the adjacent
// flowing tile for the vertical faces.  These are deliberately kept as the
// atlas indices used by the renderer, rather than treating the UVs as a
// precomputed constant.
const WATER_TOP_TILE: u16 = 205;
const WATER_SIDE_TILE: u16 = 206;
const MAX_VERTICES: usize = 30;

fn native_corner(
    center: FluidCell,
    above: Option<FluidCell>,
    cells: [Option<FluidCell>; 3],
) -> f32 {
    let is_water = |cell: FluidCell| matches!(cell.id, 8 | 9);
    if above.is_some_and(is_water) {
        return 1.0;
    }
    let mut sum = 0.0;
    let mut count = 0i32;
    for cell in [Some(center), cells[0], cells[1], cells[2]] {
        match cell {
            Some(c) if is_water(c) => {
                let level = if c.metadata >= 8 { 0 } else { c.metadata };
                let d = f32::from(level + 1) / 9.0;
                if c.metadata == 0 || c.metadata >= 8 {
                    sum += d * 10.0;
                    count += 10;
                }
                sum += d;
                count += 1;
            }
            Some(c) if c.id != 0 && c.id != 8 && c.id != 9 => {}
            _ => {
                sum += 1.0;
                count += 1;
            }
        }
    }
    1.0 - sum / count as f32
}

fn flow_angle(input: FluidNeighborhood) -> Option<f32> {
    let level = |cell: FluidCell| if cell.metadata >= 8 { 0 } else { cell.metadata };
    let center = i32::from(level(input.center));
    let mut dx = 0i32;
    let mut dz = 0i32;
    for (neighbor, x, z) in [
        (input.west, -1, 0),
        (input.north, 0, -1),
        (input.east, 1, 0),
        (input.south, 0, 1),
    ] {
        if let Some(cell) = neighbor.filter(|c| matches!(c.id, 8 | 9)) {
            let delta = i32::from(level(cell)) - center;
            dx += x * delta;
            dz += z * delta;
        }
    }
    if dx == 0 && dz == 0 {
        None
    } else {
        Some((f64::from(dz).atan2(f64::from(dx)) - std::f64::consts::FRAC_PI_2) as f32)
    }
}

fn top_uv(tile: u16, angle: Option<f32>) -> [[f32; 2]; 4] {
    if let Some(angle) = angle {
        let tx = f32::from(WATER_SIDE_TILE % 16) * TILE_SIZE;
        let ty = f32::from(WATER_SIDE_TILE / 16) * TILE_SIZE;
        let u = (tx + TILE_SIZE) / ATLAS_SIZE;
        let v = (ty + TILE_SIZE) / ATLAS_SIZE;
        let sin = angle.sin() * 8.0 / ATLAS_SIZE;
        let cos = angle.cos() * 8.0 / ATLAS_SIZE;
        return [
            [u - cos - sin, v - cos + sin],
            [u - cos + sin, v + cos + sin],
            [u + cos + sin, v + cos - sin],
            [u + cos - sin, v - cos - sin],
        ];
    }
    let tx = f32::from(tile % 16) * TILE_SIZE;
    let ty = f32::from(tile / 16) * TILE_SIZE;
    let u0 = tx / ATLAS_SIZE;
    let v0 = ty / ATLAS_SIZE;
    let u1 = (tx + TILE_SIZE) / ATLAS_SIZE;
    let v1 = (ty + TILE_SIZE) / ATLAS_SIZE;
    [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
}

fn side_uv(tile: u16, heights: Option<[f32; 2]>) -> [[f32; 2]; 4] {
    let tx = f32::from(WATER_SIDE_TILE % 16) * TILE_SIZE;
    let u0 = tx / ATLAS_SIZE;
    let u1 = (tx + TILE_SIZE - TILE_INSET) / ATLAS_SIZE;
    let meta = tile.saturating_sub(WATER_SIDE_TILE).min(7) as usize;
    let flow_v0 = [
        0x3f44_d34d,
        0x3f4c_e38e,
        0x3f4d_5555,
        0x3f4d_c71c,
        0x3f4e_38e4,
        0x3f4e_aaab,
        0x3f4f_1c72,
        0x3f4f_8e39,
    ];
    let v0 = heights.map_or(f32::from_bits(flow_v0[meta]), |h| {
        0.75 + (1.0 - h[0]) / 16.0
    });
    let v1 = heights.map_or(f32::from_bits(0x3f4f_fd71), |h| 0.75 + (1.0 - h[1]) / 16.0);
    [
        [u0, v0],
        [u1, v1],
        [u1, f32::from_bits(0x3f4f_fd71)],
        [u0, f32::from_bits(0x3f4f_fd71)],
    ]
}

fn push_quad(
    out: &mut Vec<Vertex>,
    positions: [[f32; 3]; 4],
    color: u32,
    tile: u16,
    heights: Option<[f32; 2]>,
    flow: Option<f32>,
) {
    let tex = if tile == WATER_TOP_TILE {
        top_uv(tile, flow)
    } else {
        side_uv(tile, heights)
    };
    let corners: Vec<_> = positions.into_iter().zip(tex).collect();
    for index in [0, 1, 2, 0, 2, 3] {
        let (position, uv) = corners[index];
        out.push(Vertex {
            position,
            uv,
            color,
        });
    }
}

/// Builds the native water quads at `(x, y, z)`.
pub fn mesh(x: i32, y: i32, z: i32, input: FluidNeighborhood) -> Result<Mesh, ViewError> {
    if input.center.id != 8 && input.center.id != 9 {
        return Err(ViewError::UnsupportedFluid(input.center.id));
    }
    if input.center.metadata > 15 {
        return Err(ViewError::InvalidMetadata(input.center.metadata));
    }
    let above_water = input.above.is_some_and(|c| c.id == 8 || c.id == 9);
    let xf = x as f32;
    let yf = y as f32;
    let zf = z as f32;
    // Native f24=(x,z), f25=(x,z+1), f26=(x+1,z+1), f27=(x+1,z).
    let corner = [
        native_corner(
            input.center,
            input.above,
            [input.north, input.west, input.north_west],
        ),
        native_corner(
            input.center,
            input.above,
            [input.south, input.west, input.south_west],
        ),
        native_corner(
            input.center,
            input.above,
            [input.south, input.east, input.south_east],
        ),
        native_corner(
            input.center,
            input.above,
            [input.north, input.east, input.north_east],
        ),
    ];
    let mut vertices = Vec::with_capacity(MAX_VERTICES);
    if !above_water {
        push_quad(
            &mut vertices,
            [
                [xf, yf + corner[0], zf],
                [xf, yf + corner[1], zf + 1.0],
                [xf + 1.0, yf + corner[2], zf + 1.0],
                [xf + 1.0, yf + corner[3], zf],
            ],
            0xffff_ffff,
            WATER_TOP_TILE,
            None,
            flow_angle(input),
        );
    }
    let visible = |side: Option<FluidCell>| {
        !matches!(side, Some(c) if c.id == 8 || c.id == 9) && !matches!(side, Some(c) if c.id != 0)
    };
    let mut sides = Vec::with_capacity(24);
    if visible(input.north) {
        push_quad(
            &mut sides,
            [
                [xf, yf + corner[0], zf],
                [xf + 1.0, yf + corner[3], zf],
                [xf + 1.0, yf, zf],
                [xf, yf, zf],
            ],
            0xffcc_cccc,
            WATER_SIDE_TILE
                + u16::from(if input.center.metadata >= 8 {
                    0
                } else {
                    input.center.metadata
                }),
            Some([corner[0], corner[3]]),
            None,
        );
    }
    if visible(input.south) {
        push_quad(
            &mut sides,
            [
                [xf + 1.0, yf + corner[2], zf + 1.0],
                [xf, yf + corner[1], zf + 1.0],
                [xf, yf, zf + 1.0],
                [xf + 1.0, yf, zf + 1.0],
            ],
            0xffcc_cccc,
            WATER_SIDE_TILE
                + u16::from(if input.center.metadata >= 8 {
                    0
                } else {
                    input.center.metadata
                }),
            Some([corner[2], corner[1]]),
            None,
        );
    }
    if visible(input.west) {
        push_quad(
            &mut sides,
            [
                [xf, yf + corner[1], zf + 1.0],
                [xf, yf + corner[0], zf],
                [xf, yf, zf],
                [xf, yf, zf + 1.0],
            ],
            0xff99_9999,
            WATER_SIDE_TILE
                + u16::from(if input.center.metadata >= 8 {
                    0
                } else {
                    input.center.metadata
                }),
            Some([corner[1], corner[0]]),
            None,
        );
    }
    if visible(input.east) {
        push_quad(
            &mut sides,
            [
                [xf + 1.0, yf + corner[3], zf],
                [xf + 1.0, yf + corner[2], zf + 1.0],
                [xf + 1.0, yf, zf + 1.0],
                [xf + 1.0, yf, zf],
            ],
            0xff99_9999,
            WATER_SIDE_TILE
                + u16::from(if input.center.metadata >= 8 {
                    0
                } else {
                    input.center.metadata
                }),
            Some([corner[3], corner[2]]),
            None,
        );
    }
    // The native cv.b branch order submits north, south, west, east.
    vertices.extend_from_slice(&sides);
    Ok(Mesh { vertices })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(metadata: u8) -> FluidNeighborhood {
        FluidNeighborhood {
            center: FluidCell { id: 9, metadata },
            below: None,
            above: None,
            east: None,
            west: None,
            north: None,
            south: None,
            north_east: None,
            north_west: None,
            south_east: None,
            south_west: None,
        }
    }

    fn sample(name: &str) -> FluidNeighborhood {
        let mut n = input(0);
        if let Some(rest) = name.strip_prefix("level-") {
            let mut p = rest.split('-');
            n.center.id = p.next().unwrap().parse().unwrap();
            n.center.metadata = p.next().unwrap().parse().unwrap();
            return n;
        }
        match name {
            "east-source" => n.east = Some(FluidCell { id: 9, metadata: 0 }),
            "west-flow" => n.west = Some(FluidCell { id: 8, metadata: 7 }),
            "north-source" => n.north = Some(FluidCell { id: 9, metadata: 0 }),
            "south-flow" => {
                n.south = Some(FluidCell {
                    id: 8,
                    metadata: 12,
                })
            }
            "diagonal-water" => n.south_east = Some(FluidCell { id: 9, metadata: 0 }),
            "four-sources" => {
                n.east = Some(FluidCell { id: 9, metadata: 0 });
                n.west = Some(FluidCell { id: 9, metadata: 0 });
                n.north = Some(FluidCell { id: 9, metadata: 0 });
                n.south = Some(FluidCell { id: 9, metadata: 0 });
            }
            "water-above" => n.above = Some(FluidCell { id: 9, metadata: 0 }),
            "flow-column" => {
                n.center = FluidCell { id: 8, metadata: 8 };
                n.below = Some(FluidCell { id: 8, metadata: 8 });
            }
            "solid-east" => n.east = Some(FluidCell { id: 1, metadata: 0 }),
            "solid-west" => n.west = Some(FluidCell { id: 1, metadata: 0 }),
            "solid-north" => n.north = Some(FluidCell { id: 1, metadata: 0 }),
            "solid-south" => n.south = Some(FluidCell { id: 1, metadata: 0 }),
            _ => panic!("unknown fixture case {name}"),
        }
        n
    }

    #[test]
    fn native_fixture_matches_all_samples() {
        let mut actual = Vec::new();
        for line in include_str!("../../../tests/fixtures/fluid/native.tsv").lines() {
            if let Some(rest) = line.strip_prefix("case\t") {
                let mut p = rest.split('\t');
                let _ = p.next();
            } else if let Some(rest) = line.strip_prefix("vertex\t") {
                let mut p = rest.split('\t');
                let name = p.next().unwrap();
                let words: Vec<u32> = p.map(|x| u32::from_str_radix(x, 16).unwrap()).collect();
                actual.push((name.to_owned(), words));
            }
        }
        let mut offset = 0;
        for (name, expected_count) in [
            ("level-8-0", 30),
            ("level-8-1", 30),
            ("level-8-2", 30),
            ("level-8-3", 30),
            ("level-8-4", 30),
            ("level-8-5", 30),
            ("level-8-6", 30),
            ("level-8-7", 30),
            ("level-8-8", 30),
            ("level-8-9", 30),
            ("level-8-10", 30),
            ("level-8-11", 30),
            ("level-8-12", 30),
            ("level-8-13", 30),
            ("level-8-14", 30),
            ("level-8-15", 30),
            ("level-9-0", 30),
            ("level-9-1", 30),
            ("level-9-2", 30),
            ("level-9-3", 30),
            ("level-9-4", 30),
            ("level-9-5", 30),
            ("level-9-6", 30),
            ("level-9-7", 30),
            ("level-9-8", 30),
            ("level-9-9", 30),
            ("level-9-10", 30),
            ("level-9-11", 30),
            ("level-9-12", 30),
            ("level-9-13", 30),
            ("level-9-14", 30),
            ("level-9-15", 30),
            ("east-source", 24),
            ("west-flow", 24),
            ("north-source", 24),
            ("south-flow", 24),
            ("diagonal-water", 30),
            ("four-sources", 6),
            ("water-above", 24),
            ("flow-column", 30),
            ("solid-east", 24),
            ("solid-west", 24),
            ("solid-north", 24),
            ("solid-south", 24),
        ] {
            let mesh = mesh(0, 64, 0, sample(name)).unwrap();
            assert_eq!(mesh.vertices.len(), expected_count, "{name}");
            for (vertex_index, vertex) in mesh.vertices.into_iter().enumerate() {
                let words = [
                    vertex.position[0].to_bits(),
                    vertex.position[1].to_bits(),
                    vertex.position[2].to_bits(),
                    vertex.uv[0].to_bits(),
                    vertex.uv[1].to_bits(),
                    vertex.color,
                    0,
                    0,
                ];
                let expected = &actual[offset].1;
                let fields = ["x", "y", "z", "u", "v", "color", "extra-6", "extra-7"];
                assert_eq!(
                    expected.len(),
                    words.len(),
                    "{name} vertex {vertex_index} field count"
                );
                for (field, (got, want)) in fields.iter().zip(words.iter().zip(expected.iter())) {
                    assert_eq!(
                        got, want,
                        "{name} vertex {vertex_index} field {field} (fixture index {offset})"
                    );
                }
                offset += 1;
            }
        }
        assert_eq!(offset, actual.len());
    }

    #[test]
    fn isolated_fixture_shape_and_raw_uv() {
        let mesh = mesh(0, 64, 0, input(0)).unwrap();
        assert_eq!(mesh.vertices.len(), 30);
        assert_eq!(
            mesh.vertices[0].uv.map(f32::to_bits),
            [0x3f500000, 0x3f400000]
        );
        assert_eq!(mesh.vertices[0].color, 0xffff_ffff);
    }

    #[test]
    fn cardinal_water_removes_one_side_and_above_keeps_sides() {
        let mut n = input(8);
        n.east = Some(FluidCell { id: 8, metadata: 0 });
        assert_eq!(mesh(0, 64, 0, n).unwrap().vertices.len(), 24);
        n.above = Some(FluidCell { id: 8, metadata: 0 });
        assert_eq!(mesh(0, 64, 0, n).unwrap().vertices.len(), 18);
    }

    #[test]
    fn invalid_state_is_rejected() {
        assert_eq!(
            mesh(0, 0, 0, input(16)),
            Err(ViewError::InvalidMetadata(16))
        );
        assert_eq!(
            mesh(
                0,
                0,
                0,
                FluidNeighborhood {
                    center: FluidCell { id: 1, metadata: 0 },
                    ..input(0)
                }
            ),
            Err(ViewError::UnsupportedFluid(1))
        );
    }
}
