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
const UVS: [[[u32; 2]; 24]; 7] = [
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
];

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
