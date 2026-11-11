//! CPU geometry for native GUI full-block icons.
//!
//! The source quads, UVs and transform sequence come from the pinned `bb`/`cv`
//! capture in `tests/fixtures/inventory_cube/native.tsv`. Colors are the native
//! white GL input; directional GL lighting, depth, and rasterization are not
//! modeled here. Callers must supply lighting and a depth-capable render pass.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// GUI-space position after the native model-view transforms, including depth.
    pub position: [f64; 3],
    /// Terrain atlas UV in the original 256 by 256 atlas.
    pub uv: [f64; 2],
    /// Face normal before the native model-view transforms.
    pub normal: [f64; 3],
}

const POS: [[[u8; 3]; 4]; 6] = [
    [[0, 0, 1], [0, 0, 0], [1, 0, 0], [1, 0, 1]],
    [[1, 1, 1], [1, 1, 0], [0, 1, 0], [0, 1, 1]],
    [[0, 1, 0], [1, 1, 0], [1, 0, 0], [0, 0, 0]],
    [[0, 1, 1], [0, 0, 1], [1, 0, 1], [1, 1, 1]],
    [[0, 1, 1], [0, 1, 0], [0, 0, 0], [0, 0, 1]],
    [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
];
const UV: [[[u8; 2]; 4]; 6] = [
    [[0, 1], [0, 0], [1, 0], [1, 1]],
    [[1, 1], [1, 0], [0, 0], [0, 1]],
    [[1, 0], [0, 0], [0, 1], [1, 1]],
    [[0, 0], [0, 1], [1, 1], [1, 0]],
    [[1, 0], [0, 0], [0, 1], [1, 1]],
    [[0, 1], [1, 1], [1, 0], [0, 0]],
];
const NORMAL: [[f64; 3]; 6] = [
    [0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, 1.0],
    [-1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
];

/// The original renderer's six ordered quads in model coordinates.
/// Face tiles must be valid native terrain atlas indices (0..=255).
pub fn model_quads(tiles: [u16; 6]) -> Option<[[Vertex; 4]; 6]> {
    if tiles.iter().any(|&tile| tile > 255) {
        return None;
    }
    Some(std::array::from_fn(|face| {
        let tile = tiles[face];
        std::array::from_fn(|corner| {
            let position = POS[face][corner].map(f64::from);
            let [u, v] = UV[face][corner];
            let x = f64::from(tile % 16) * 16.0;
            let y = f64::from(tile / 16) * 16.0;
            Vertex {
                position,
                uv: [
                    (x + f64::from(u) * 15.99) / 256.0,
                    (y + f64::from(v) * 15.99) / 256.0,
                ],
                normal: NORMAL[face],
            }
        })
    }))
}

/// Apply the recorded OpenGL model-view sequence to a GUI origin in logical pixels.
/// GL matrices postmultiply, so the local translation and rotations apply first.
pub fn transform_position(position: [f64; 3], origin: [f64; 2]) -> [f64; 3] {
    let [x, y, z] = position.map(|value| value - 0.5);
    // Rotations about Y by -90 then +45 degrees combine to -45 degrees.
    let c = std::f64::consts::FRAC_1_SQRT_2;
    let (x, z) = (c * (x - z), c * (x + z));
    let angle = 210.0_f64.to_radians();
    let (s, c) = angle.sin_cos();
    let y_rot = c * y - s * z;
    let z_rot = s * y + c * z;
    [
        origin[0] - 2.0 + 10.0 * (x + 1.0),
        origin[1] + 3.0 + 10.0 * (y_rot + 0.5),
        -3.0 + 10.0 * (1.0 - z_rot),
    ]
}

/// Apply the native transform to all submitted vertices, keeping native order.
pub fn gui_quads(tiles: [u16; 6], origin: [f64; 2]) -> Option<[[Vertex; 4]; 6]> {
    let mut quads = model_quads(tiles)?;
    for vertex in quads.iter_mut().flatten() {
        vertex.position = transform_position(vertex.position, origin);
    }
    Some(quads)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_float(s: &str) -> f64 {
        let (sign, s) = s.strip_prefix('-').map_or((1.0, s), |v| (-1.0, v));
        let (mantissa, exponent) = s.strip_prefix("0x").unwrap().split_once('p').unwrap();
        let (integer, fraction) = mantissa.split_once('.').unwrap();
        let mut value = u64::from_str_radix(integer, 16).unwrap() as f64;
        for (i, c) in fraction.chars().enumerate() {
            value += f64::from(c.to_digit(16).unwrap()) * 16_f64.powi(-(i as i32 + 1));
        }
        sign * value * 2_f64.powi(exponent.parse().unwrap())
    }

    #[test]
    fn all_recorded_original_vertices_uvs_and_normals_match() {
        let fixture = include_str!("../../../tests/fixtures/inventory_cube/native.tsv");
        let mut quads = None;
        let mut face = 0;
        let mut corner = 0;
        let mut cases = 0;
        for line in fixture.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields[0] {
                "case" => {
                    let id: u16 = fields[1].parse().unwrap();
                    let meta: u16 = fields[2].parse().unwrap();
                    let tiles = match id {
                        1 => [1; 6],
                        17 => {
                            let side = match meta {
                                1 => 116,
                                2 => 117,
                                _ => 20,
                            };
                            [21, 21, side, side, side, side]
                        }
                        35 => {
                            [match meta {
                                0 => 64,
                                1 => 210,
                                15 => 113,
                                _ => panic!("unexpected wool"),
                            }; 6]
                        }
                        _ => panic!("unexpected block"),
                    };
                    quads = model_quads(tiles);
                    face = 0;
                    corner = 0;
                    cases += 1;
                }
                "normal" => {
                    let actual = quads.unwrap()[face][0].normal;
                    for i in 0..3 {
                        assert_eq!(actual[i], hex_float(fields[i + 1]));
                    }
                }
                "vertex" => {
                    let actual = quads.unwrap()[face][corner];
                    for i in 0..3 {
                        assert_eq!(actual.position[i], hex_float(fields[i + 1]));
                    }
                    for i in 0..2 {
                        assert!((actual.uv[i] - hex_float(fields[i + 4])).abs() < 1e-15);
                    }
                    corner += 1;
                    if corner == 4 {
                        corner = 0;
                        face += 1;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(cases, 7);
        assert_eq!(face, 6);
        assert!(model_quads([256; 6]).is_none());
    }

    #[test]
    fn native_transform_centers_cube_and_preserves_edges() {
        let origin = [10.0, 20.0];
        let center = transform_position([0.5; 3], origin);
        assert_eq!(center, [18.0, 28.0, 7.0]);
        let p = transform_position([0.0, 0.0, 0.0], origin);
        let q = transform_position([1.0, 0.0, 0.0], origin);
        let length = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
        assert!((length - 10.0).abs() < 1e-12);
        assert_eq!(
            gui_quads([1; 6], origin).unwrap()[0][0].position,
            transform_position([0.0, 0.0, 1.0], origin)
        );
    }

    #[test]
    fn cpu_transform_matches_recorded_gl_commands_for_every_cube_corner() {
        let fixture = include_str!("../../../tests/fixtures/inventory_cube/native.tsv");
        let mut commands: Vec<(&str, Vec<f64>)> = Vec::new();
        for line in fixture.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields[0] {
                "case" => commands.clear(),
                "translate" | "scale" | "rotate" if commands.len() < 8 => {
                    commands.push((
                        fields[0],
                        fields[1..].iter().map(|v| hex_float(v)).collect(),
                    ));
                }
                "vertex" => {
                    assert_eq!(commands.len(), 8);
                    let mut p = [
                        hex_float(fields[1]),
                        hex_float(fields[2]),
                        hex_float(fields[3]),
                    ];
                    for (op, v) in commands.iter().rev() {
                        match *op {
                            "translate" => {
                                for i in 0..3 {
                                    p[i] += v[i];
                                }
                            }
                            "scale" => {
                                for i in 0..3 {
                                    p[i] *= v[i];
                                }
                            }
                            "rotate" => {
                                let (s, c) = v[0].to_radians().sin_cos();
                                p = if v[1] == 1.0 {
                                    [p[0], c * p[1] - s * p[2], s * p[1] + c * p[2]]
                                } else {
                                    [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]]
                                };
                            }
                            _ => unreachable!(),
                        }
                    }
                    let expected = transform_position(
                        [
                            hex_float(fields[1]),
                            hex_float(fields[2]),
                            hex_float(fields[3]),
                        ],
                        [10.0, 20.0],
                    );
                    for i in 0..3 {
                        assert!((p[i] - expected[i]).abs() < 1e-12);
                    }
                }
                _ => {}
            }
        }
    }
}
