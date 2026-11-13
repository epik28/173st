//! Native Beta crossed plant geometry (saplings, flowers and mushrooms).
use crate::{Mesh, MeshLimits, Vertex, ViewError};
use beta_core::chunk::BlockData;

const POSITIONS: [[u32; 3]; 24] = [
    [0x3d4cccd0, 0x3f800000, 0x3d4cccd0],
    [0x3d4cccd0, 0x00000000, 0x3d4cccd0],
    [0x3f733333, 0x00000000, 0x3f733333],
    [0x3d4cccd0, 0x3f800000, 0x3d4cccd0],
    [0x3f733333, 0x00000000, 0x3f733333],
    [0x3f733333, 0x3f800000, 0x3f733333],
    [0x3f733333, 0x3f800000, 0x3f733333],
    [0x3f733333, 0x00000000, 0x3f733333],
    [0x3d4cccd0, 0x00000000, 0x3d4cccd0],
    [0x3f733333, 0x3f800000, 0x3f733333],
    [0x3d4cccd0, 0x00000000, 0x3d4cccd0],
    [0x3d4cccd0, 0x3f800000, 0x3d4cccd0],
    [0x3d4cccd0, 0x3f800000, 0x3f733333],
    [0x3d4cccd0, 0x00000000, 0x3f733333],
    [0x3f733333, 0x00000000, 0x3d4cccd0],
    [0x3d4cccd0, 0x3f800000, 0x3f733333],
    [0x3f733333, 0x00000000, 0x3d4cccd0],
    [0x3f733333, 0x3f800000, 0x3d4cccd0],
    [0x3f733333, 0x3f800000, 0x3d4cccd0],
    [0x3f733333, 0x00000000, 0x3d4cccd0],
    [0x3d4cccd0, 0x00000000, 0x3f733333],
    [0x3f733333, 0x3f800000, 0x3d4cccd0],
    [0x3d4cccd0, 0x00000000, 0x3f733333],
    [0x3d4cccd0, 0x3f800000, 0x3f733333],
];
const UVS: [[[u32; 2]; 24]; 9] = [
    [
        [0x3f700000, 0x00000000],
        [0x3f700000, 0x3d7fd70a],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f700000, 0x00000000],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f7ffd71, 0x00000000],
        [0x3f700000, 0x00000000],
        [0x3f700000, 0x3d7fd70a],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f700000, 0x00000000],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f7ffd71, 0x00000000],
        [0x3f700000, 0x00000000],
        [0x3f700000, 0x3d7fd70a],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f700000, 0x00000000],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f7ffd71, 0x00000000],
        [0x3f700000, 0x00000000],
        [0x3f700000, 0x3d7fd70a],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f700000, 0x00000000],
        [0x3f7ffd71, 0x3d7fd70a],
        [0x3f7ffd71, 0x00000000],
    ],
    [
        [0x3f700000, 0x3e400000],
        [0x3f700000, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f700000, 0x3e400000],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e400000],
        [0x3f700000, 0x3e400000],
        [0x3f700000, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f700000, 0x3e400000],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e400000],
        [0x3f700000, 0x3e400000],
        [0x3f700000, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f700000, 0x3e400000],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e400000],
        [0x3f700000, 0x3e400000],
        [0x3f700000, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f700000, 0x3e400000],
        [0x3f7ffd71, 0x3e7ff5c2],
        [0x3f7ffd71, 0x3e400000],
    ],
    [
        [0x3f700000, 0x3e800000],
        [0x3f700000, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f700000, 0x3e800000],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e800000],
        [0x3f700000, 0x3e800000],
        [0x3f700000, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f700000, 0x3e800000],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e800000],
        [0x3f700000, 0x3e800000],
        [0x3f700000, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f700000, 0x3e800000],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e800000],
        [0x3f700000, 0x3e800000],
        [0x3f700000, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f700000, 0x3e800000],
        [0x3f7ffd71, 0x3e9ffae1],
        [0x3f7ffd71, 0x3e800000],
    ],
    [
        [0x3f500000, 0x00000000],
        [0x3f500000, 0x3d7fd70a],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f500000, 0x00000000],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f5ffd71, 0x00000000],
        [0x3f500000, 0x00000000],
        [0x3f500000, 0x3d7fd70a],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f500000, 0x00000000],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f5ffd71, 0x00000000],
        [0x3f500000, 0x00000000],
        [0x3f500000, 0x3d7fd70a],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f500000, 0x00000000],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f5ffd71, 0x00000000],
        [0x3f500000, 0x00000000],
        [0x3f500000, 0x3d7fd70a],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f500000, 0x00000000],
        [0x3f5ffd71, 0x3d7fd70a],
        [0x3f5ffd71, 0x00000000],
    ],
    [
        [0x3f400000, 0x00000000],
        [0x3f400000, 0x3d7fd70a],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f400000, 0x00000000],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f4ffd71, 0x00000000],
        [0x3f400000, 0x00000000],
        [0x3f400000, 0x3d7fd70a],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f400000, 0x00000000],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f4ffd71, 0x00000000],
        [0x3f400000, 0x00000000],
        [0x3f400000, 0x3d7fd70a],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f400000, 0x00000000],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f4ffd71, 0x00000000],
        [0x3f400000, 0x00000000],
        [0x3f400000, 0x3d7fd70a],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f400000, 0x00000000],
        [0x3f4ffd71, 0x3d7fd70a],
        [0x3f4ffd71, 0x00000000],
    ],
    [
        [0x3f500000, 0x3d800000],
        [0x3f500000, 0x3dffeb85],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f500000, 0x3d800000],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f5ffd71, 0x3d800000],
        [0x3f500000, 0x3d800000],
        [0x3f500000, 0x3dffeb85],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f500000, 0x3d800000],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f5ffd71, 0x3d800000],
        [0x3f500000, 0x3d800000],
        [0x3f500000, 0x3dffeb85],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f500000, 0x3d800000],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f5ffd71, 0x3d800000],
        [0x3f500000, 0x3d800000],
        [0x3f500000, 0x3dffeb85],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f500000, 0x3d800000],
        [0x3f5ffd71, 0x3dffeb85],
        [0x3f5ffd71, 0x3d800000],
    ],
    [
        [0x3f400000, 0x3d800000],
        [0x3f400000, 0x3dffeb85],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f400000, 0x3d800000],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f4ffd71, 0x3d800000],
        [0x3f400000, 0x3d800000],
        [0x3f400000, 0x3dffeb85],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f400000, 0x3d800000],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f4ffd71, 0x3d800000],
        [0x3f400000, 0x3d800000],
        [0x3f400000, 0x3dffeb85],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f400000, 0x3d800000],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f4ffd71, 0x3d800000],
        [0x3f400000, 0x3d800000],
        [0x3f400000, 0x3dffeb85],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f400000, 0x3d800000],
        [0x3f4ffd71, 0x3dffeb85],
        [0x3f4ffd71, 0x3d800000],
    ],
    [
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
    ],
    [
        [0x3f100000, 0x3e800000],
        [0x3f100000, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f100000, 0x3e800000],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e800000],
        [0x3f100000, 0x3e800000],
        [0x3f100000, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f100000, 0x3e800000],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e800000],
        [0x3f100000, 0x3e800000],
        [0x3f100000, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f100000, 0x3e800000],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e800000],
        [0x3f100000, 0x3e800000],
        [0x3f100000, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f100000, 0x3e800000],
        [0x3f1ffd71, 0x3e9ffae1],
        [0x3f1ffd71, 0x3e800000],
    ],
];

const GRASS_UVS: [[[u32; 2]; 24]; 3] = [
    [
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
        [0x3ee00000, 0x3e400000],
        [0x3ee00000, 0x3e7ff5c2],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3ee00000, 0x3e400000],
        [0x3efffae1, 0x3e7ff5c2],
        [0x3efffae1, 0x3e400000],
    ],
    [
        [0x3ee00000, 0x3e000000],
        [0x3ee00000, 0x3e3ff5c2],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3ee00000, 0x3e000000],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3efffae1, 0x3e000000],
        [0x3ee00000, 0x3e000000],
        [0x3ee00000, 0x3e3ff5c2],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3ee00000, 0x3e000000],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3efffae1, 0x3e000000],
        [0x3ee00000, 0x3e000000],
        [0x3ee00000, 0x3e3ff5c2],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3ee00000, 0x3e000000],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3efffae1, 0x3e000000],
        [0x3ee00000, 0x3e000000],
        [0x3ee00000, 0x3e3ff5c2],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3ee00000, 0x3e000000],
        [0x3efffae1, 0x3e3ff5c2],
        [0x3efffae1, 0x3e000000],
    ],
    [
        [0x3f000000, 0x3e400000],
        [0x3f000000, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f000000, 0x3e400000],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e400000],
        [0x3f000000, 0x3e400000],
        [0x3f000000, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f000000, 0x3e400000],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e400000],
        [0x3f000000, 0x3e400000],
        [0x3f000000, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f000000, 0x3e400000],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e400000],
        [0x3f000000, 0x3e400000],
        [0x3f000000, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f000000, 0x3e400000],
        [0x3f0ffd71, 0x3e7ff5c2],
        [0x3f0ffd71, 0x3e400000],
    ],
];

/// Exact coordinates sampled for Beta tall grass biome color (metadata != 0).
#[must_use]
pub fn tall_grass_biome_coords(x: i32, y: i32, z: i32) -> (i32, i32) {
    let seed = x
        .wrapping_mul(3_129_871)
        .wrapping_add(z.wrapping_mul(6_129_781))
        .wrapping_add(y) as i64;
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    (
        x.wrapping_add(((seed >> 14) & 31) as i32),
        z.wrapping_add(((seed >> 24) & 31) as i32),
    )
}

pub(crate) fn append_grass(
    mesh: &mut Mesh,
    pos: [i32; 3],
    block: BlockData,
    minimum_light: u8,
    tint: [f32; 3],
    limits: MeshLimits,
) -> Result<(), ViewError> {
    if mesh.vertices.len().saturating_add(24) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(24) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 24,
            indices: mesh.indices.len() + 24,
        });
    }
    let seed = i64::from(pos[0].wrapping_mul(3_129_871))
        ^ i64::from(pos[2]).wrapping_mul(116_129_781)
        ^ i64::from(pos[1]);
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    let ratio = |shift: u32| (((seed >> shift) & 15_i64) as f32 / 15.0_f32) as f64;
    let origin = [
        f64::from(pos[0]) + (ratio(16) - 0.5) * 0.5,
        f64::from(pos[1]) + (ratio(20) - 1.0) * 0.2,
        f64::from(pos[2]) + (ratio(24) - 0.5) * 0.5,
    ];
    let tile = match block.metadata {
        0 => 0,
        2 => 2,
        _ => 1,
    };
    let base = mesh.vertices.len() as u32;
    for i in 0..24 {
        let bits = POSITIONS[i];
        let uv = GRASS_UVS[tile][i];
        mesh.vertices.push(Vertex {
            position: [
                (origin[0] + f64::from(f32::from_bits(bits[0]))) as f32,
                (origin[1] + f64::from(f32::from_bits(bits[1]))) as f32,
                (origin[2] + f64::from(f32::from_bits(bits[2]))) as f32,
            ],
            uv: [f32::from_bits(uv[0]), f32::from_bits(uv[1])],
            block_light: block.block_light,
            sky_light: block.sky_light,
            minimum_light,
            shade: 1.0,
            tint,
            tint_after_light: false,
        });
        mesh.indices.push(base + i as u32);
    }
    mesh.visible_faces += 4;
    Ok(())
}

pub(crate) fn append(
    mesh: &mut Mesh,
    pos: [i32; 3],
    block: BlockData,
    minimum_light: u8,
    limits: MeshLimits,
) -> Result<(), ViewError> {
    let tile = match block.id {
        6 => match block.metadata & 3 {
            1 => 1,
            2 => 2,
            _ => 0,
        },
        37 => 3,
        38 => 4,
        39 => 5,
        40 => 6,
        32 => 7,
        83 => 8,
        _ => unreachable!(),
    };
    if mesh.vertices.len().saturating_add(24) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(24) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 24,
            indices: mesh.indices.len() + 24,
        });
    }
    let base = mesh.vertices.len() as u32;
    for i in 0..24 {
        let bits = POSITIONS[i];
        let uv = UVS[tile][i];
        mesh.vertices.push(Vertex {
            position: [
                f32::from_bits(bits[0]) + pos[0] as f32,
                f32::from_bits(bits[1]) + pos[1] as f32,
                f32::from_bits(bits[2]) + pos[2] as f32,
            ],
            uv: [f32::from_bits(uv[0]), f32::from_bits(uv[1])],
            block_light: block.block_light,
            sky_light: block.sky_light,
            minimum_light,
            shade: 1.0,
            tint: [1.0; 3],
            tint_after_light: false,
        });
        mesh.indices.push(base + i as u32);
    }
    mesh.visible_faces += 4;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tall_grass_coordinates_and_biome_samples_match_native() {
        let data = include_str!("../../../tests/fixtures/tall_grass/native.tsv");
        let mut mesh = Mesh::default();
        let mut case = (0, 0);
        let mut count = 0;
        for line in data.lines() {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] == "case" {
                let x: i32 = row[3].parse().unwrap();
                let y: i32 = row[4].parse().unwrap();
                let z: i32 = row[5].parse().unwrap();
                let meta: u8 = row[2].parse().unwrap();
                case = (row[1].parse::<usize>().unwrap(), meta);
                if meta != 0 {
                    assert_eq!(
                        tall_grass_biome_coords(x, y, z),
                        (row[6].parse().unwrap(), row[7].parse().unwrap()),
                        "case {case:?}"
                    );
                }
                mesh = Mesh::default();
                append_grass(
                    &mut mesh,
                    [x, y, z],
                    BlockData {
                        id: 31,
                        metadata: meta,
                        block_light: 0,
                        sky_light: 15,
                    },
                    0,
                    [1.0; 3],
                    MeshLimits::default(),
                )
                .unwrap();
            } else if row[0] == "vertex" {
                let i: usize = row[3].parse().unwrap();
                let bits: Vec<u32> = row[4..9]
                    .iter()
                    .map(|s| u32::from_str_radix(s, 16).unwrap())
                    .collect();
                assert_eq!(
                    mesh.vertices[i].position.map(f32::to_bits),
                    [bits[0], bits[1], bits[2]],
                    "case {case:?} vertex {i}"
                );
                assert_eq!(
                    mesh.vertices[i].uv.map(f32::to_bits),
                    [bits[3], bits[4]],
                    "case {case:?} vertex {i}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 5 * 16 * 24);
    }

    #[test]
    fn wild_plants_match_native_for_every_metadata() {
        let corpus = include_str!("../../../tests/fixtures/wild_plant_render/native.tsv");
        let mut count = 0;
        for line in corpus.lines().filter(|line| line.starts_with("vertex\t")) {
            let row: Vec<_> = line.split('\t').collect();
            let id = row[1].parse().unwrap();
            let meta = row[2].parse().unwrap();
            let index: usize = row[3].parse().unwrap();
            let bits: Vec<u32> = row[4..9]
                .iter()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            let mut mesh = Mesh::default();
            append(
                &mut mesh,
                [0, 0, 0],
                BlockData {
                    id,
                    metadata: meta,
                    block_light: 0,
                    sky_light: 15,
                },
                0,
                MeshLimits::default(),
            )
            .unwrap();
            let v = mesh.vertices[index];
            assert_eq!(
                v.position.map(f32::to_bits),
                [bits[0], bits[1], bits[2]],
                "id={id} meta={meta} vertex={index}"
            );
            assert_eq!(
                v.uv.map(f32::to_bits),
                [bits[3], bits[4]],
                "id={id} meta={meta} vertex={index}"
            );
            count += 1;
        }
        assert_eq!(count, 2 * 16 * 24);
    }

    #[test]
    fn all_plants_and_metadata_match_native_positions_and_uv() {
        let corpus = include_str!("../../../tests/fixtures/plant_render/native.tsv");
        let mut seen = 0;
        for line in corpus.lines().filter(|line| line.starts_with("vertex\t")) {
            let row: Vec<_> = line.split('\t').collect();
            let id = row[1].parse().unwrap();
            let meta = row[2].parse().unwrap();
            let index: usize = row[3].parse().unwrap();
            let bits: Vec<u32> = row[4..9]
                .iter()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            let mut mesh = Mesh::default();
            append(
                &mut mesh,
                [0, 0, 0],
                BlockData {
                    id,
                    metadata: meta,
                    block_light: 0,
                    sky_light: 15,
                },
                0,
                MeshLimits::default(),
            )
            .unwrap();
            let vertex = mesh.vertices[index];
            assert_eq!(
                vertex.position.map(f32::to_bits),
                [bits[0], bits[1], bits[2]],
                "id={id} meta={meta} vertex={index}"
            );
            assert_eq!(
                vertex.uv.map(f32::to_bits),
                [bits[3], bits[4]],
                "id={id} meta={meta} vertex={index}"
            );
            seen += 1;
        }
        assert_eq!(seen, 5 * 16 * 24);
    }
}
