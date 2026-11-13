//! Native Beta 1.7.3 torch geometry, captured at block origin.
//! Metadata 1..4 use wall orientations; other values use the floor model.

use crate::{Mesh, MeshLimits, Vertex, ViewError};
use beta_core::chunk::BlockData;

const POSITIONS: [[[u32; 3]; 30]; 5] = [
    [
        [0x3ee00000, 0x3f200000, 0x3ee00000],
        [0x3ee00000, 0x3f200000, 0x3f100000],
        [0x3f100000, 0x3f200000, 0x3f100000],
        [0x3ee00000, 0x3f200000, 0x3ee00000],
        [0x3f100000, 0x3f200000, 0x3f100000],
        [0x3f100000, 0x3f200000, 0x3ee00000],
        [0x3ee00000, 0x3f800000, 0x00000000],
        [0x3ee00000, 0x00000000, 0x00000000],
        [0x3ee00000, 0x00000000, 0x3f800000],
        [0x3ee00000, 0x3f800000, 0x00000000],
        [0x3ee00000, 0x00000000, 0x3f800000],
        [0x3ee00000, 0x3f800000, 0x3f800000],
        [0x3f100000, 0x3f800000, 0x3f800000],
        [0x3f100000, 0x00000000, 0x3f800000],
        [0x3f100000, 0x00000000, 0x00000000],
        [0x3f100000, 0x3f800000, 0x3f800000],
        [0x3f100000, 0x00000000, 0x00000000],
        [0x3f100000, 0x3f800000, 0x00000000],
        [0x00000000, 0x3f800000, 0x3f100000],
        [0x00000000, 0x00000000, 0x3f100000],
        [0x3f800000, 0x00000000, 0x3f100000],
        [0x00000000, 0x3f800000, 0x3f100000],
        [0x3f800000, 0x00000000, 0x3f100000],
        [0x3f800000, 0x3f800000, 0x3f100000],
        [0x3f800000, 0x3f800000, 0x3ee00000],
        [0x3f800000, 0x00000000, 0x3ee00000],
        [0x00000000, 0x00000000, 0x3ee00000],
        [0x3f800000, 0x3f800000, 0x3ee00000],
        [0x00000000, 0x00000000, 0x3ee00000],
        [0x00000000, 0x3f800000, 0x3ee00000],
    ],
    [
        [0x3e400000, 0x3f533333, 0x3ee00000],
        [0x3e400000, 0x3f533333, 0x3f100000],
        [0x3ea00000, 0x3f533333, 0x3f100000],
        [0x3e400000, 0x3f533333, 0x3ee00000],
        [0x3ea00000, 0x3f533333, 0x3f100000],
        [0x3ea00000, 0x3f533333, 0x3ee00000],
        [0x3eaccccd, 0x3f99999a, 0x00000000],
        [0xbd800000, 0x3e4ccccd, 0x00000000],
        [0xbd800000, 0x3e4ccccd, 0x3f800000],
        [0x3eaccccd, 0x3f99999a, 0x00000000],
        [0xbd800000, 0x3e4ccccd, 0x3f800000],
        [0x3eaccccd, 0x3f99999a, 0x3f800000],
        [0x3eeccccd, 0x3f99999a, 0x3f800000],
        [0x3d800000, 0x3e4ccccd, 0x3f800000],
        [0x3d800000, 0x3e4ccccd, 0x00000000],
        [0x3eeccccd, 0x3f99999a, 0x3f800000],
        [0x3d800000, 0x3e4ccccd, 0x00000000],
        [0x3eeccccd, 0x3f99999a, 0x00000000],
        [0xbdcccccc, 0x3f99999a, 0x3f100000],
        [0xbf000000, 0x3e4ccccd, 0x3f100000],
        [0x3f000000, 0x3e4ccccd, 0x3f100000],
        [0xbdcccccc, 0x3f99999a, 0x3f100000],
        [0x3f000000, 0x3e4ccccd, 0x3f100000],
        [0x3f666666, 0x3f99999a, 0x3f100000],
        [0x3f666666, 0x3f99999a, 0x3ee00000],
        [0x3f000000, 0x3e4ccccd, 0x3ee00000],
        [0xbf000000, 0x3e4ccccd, 0x3ee00000],
        [0x3f666666, 0x3f99999a, 0x3ee00000],
        [0xbf000000, 0x3e4ccccd, 0x3ee00000],
        [0xbdcccccc, 0x3f99999a, 0x3ee00000],
    ],
    [
        [0x3f300000, 0x3f533333, 0x3ee00000],
        [0x3f300000, 0x3f533333, 0x3f100000],
        [0x3f500000, 0x3f533333, 0x3f100000],
        [0x3f300000, 0x3f533333, 0x3ee00000],
        [0x3f500000, 0x3f533333, 0x3f100000],
        [0x3f500000, 0x3f533333, 0x3ee00000],
        [0x3f09999a, 0x3f99999a, 0x00000000],
        [0x3f700000, 0x3e4ccccd, 0x00000000],
        [0x3f700000, 0x3e4ccccd, 0x3f800000],
        [0x3f09999a, 0x3f99999a, 0x00000000],
        [0x3f700000, 0x3e4ccccd, 0x3f800000],
        [0x3f09999a, 0x3f99999a, 0x3f800000],
        [0x3f29999a, 0x3f99999a, 0x3f800000],
        [0x3f880000, 0x3e4ccccd, 0x3f800000],
        [0x3f880000, 0x3e4ccccd, 0x00000000],
        [0x3f29999a, 0x3f99999a, 0x3f800000],
        [0x3f880000, 0x3e4ccccd, 0x00000000],
        [0x3f29999a, 0x3f99999a, 0x00000000],
        [0x3dcccccc, 0x3f99999a, 0x3f100000],
        [0x3f000000, 0x3e4ccccd, 0x3f100000],
        [0x3fc00000, 0x3e4ccccd, 0x3f100000],
        [0x3dcccccc, 0x3f99999a, 0x3f100000],
        [0x3fc00000, 0x3e4ccccd, 0x3f100000],
        [0x3f8ccccd, 0x3f99999a, 0x3f100000],
        [0x3f8ccccd, 0x3f99999a, 0x3ee00000],
        [0x3fc00000, 0x3e4ccccd, 0x3ee00000],
        [0x3f000000, 0x3e4ccccd, 0x3ee00000],
        [0x3f8ccccd, 0x3f99999a, 0x3ee00000],
        [0x3f000000, 0x3e4ccccd, 0x3ee00000],
        [0x3dcccccc, 0x3f99999a, 0x3ee00000],
    ],
    [
        [0x3ee00000, 0x3f533333, 0x3e400000],
        [0x3ee00000, 0x3f533333, 0x3ea00000],
        [0x3f100000, 0x3f533333, 0x3ea00000],
        [0x3ee00000, 0x3f533333, 0x3e400000],
        [0x3f100000, 0x3f533333, 0x3ea00000],
        [0x3f100000, 0x3f533333, 0x3e400000],
        [0x3ee00000, 0x3f99999a, 0xbdcccccc],
        [0x3ee00000, 0x3e4ccccd, 0xbf000000],
        [0x3ee00000, 0x3e4ccccd, 0x3f000000],
        [0x3ee00000, 0x3f99999a, 0xbdcccccc],
        [0x3ee00000, 0x3e4ccccd, 0x3f000000],
        [0x3ee00000, 0x3f99999a, 0x3f666666],
        [0x3f100000, 0x3f99999a, 0x3f666666],
        [0x3f100000, 0x3e4ccccd, 0x3f000000],
        [0x3f100000, 0x3e4ccccd, 0xbf000000],
        [0x3f100000, 0x3f99999a, 0x3f666666],
        [0x3f100000, 0x3e4ccccd, 0xbf000000],
        [0x3f100000, 0x3f99999a, 0xbdcccccc],
        [0x00000000, 0x3f99999a, 0x3eeccccd],
        [0x00000000, 0x3e4ccccd, 0x3d800000],
        [0x3f800000, 0x3e4ccccd, 0x3d800000],
        [0x00000000, 0x3f99999a, 0x3eeccccd],
        [0x3f800000, 0x3e4ccccd, 0x3d800000],
        [0x3f800000, 0x3f99999a, 0x3eeccccd],
        [0x3f800000, 0x3f99999a, 0x3eaccccd],
        [0x3f800000, 0x3e4ccccd, 0xbd800000],
        [0x00000000, 0x3e4ccccd, 0xbd800000],
        [0x3f800000, 0x3f99999a, 0x3eaccccd],
        [0x00000000, 0x3e4ccccd, 0xbd800000],
        [0x00000000, 0x3f99999a, 0x3eaccccd],
    ],
    [
        [0x3ee00000, 0x3f533333, 0x3f300000],
        [0x3ee00000, 0x3f533333, 0x3f500000],
        [0x3f100000, 0x3f533333, 0x3f500000],
        [0x3ee00000, 0x3f533333, 0x3f300000],
        [0x3f100000, 0x3f533333, 0x3f500000],
        [0x3f100000, 0x3f533333, 0x3f300000],
        [0x3ee00000, 0x3f99999a, 0x3dcccccc],
        [0x3ee00000, 0x3e4ccccd, 0x3f000000],
        [0x3ee00000, 0x3e4ccccd, 0x3fc00000],
        [0x3ee00000, 0x3f99999a, 0x3dcccccc],
        [0x3ee00000, 0x3e4ccccd, 0x3fc00000],
        [0x3ee00000, 0x3f99999a, 0x3f8ccccd],
        [0x3f100000, 0x3f99999a, 0x3f8ccccd],
        [0x3f100000, 0x3e4ccccd, 0x3fc00000],
        [0x3f100000, 0x3e4ccccd, 0x3f000000],
        [0x3f100000, 0x3f99999a, 0x3f8ccccd],
        [0x3f100000, 0x3e4ccccd, 0x3f000000],
        [0x3f100000, 0x3f99999a, 0x3dcccccc],
        [0x00000000, 0x3f99999a, 0x3f29999a],
        [0x00000000, 0x3e4ccccd, 0x3f880000],
        [0x3f800000, 0x3e4ccccd, 0x3f880000],
        [0x00000000, 0x3f99999a, 0x3f29999a],
        [0x3f800000, 0x3e4ccccd, 0x3f880000],
        [0x3f800000, 0x3f99999a, 0x3f29999a],
        [0x3f800000, 0x3f99999a, 0x3f09999a],
        [0x3f800000, 0x3e4ccccd, 0x3f700000],
        [0x00000000, 0x3e4ccccd, 0x3f700000],
        [0x3f800000, 0x3f99999a, 0x3f09999a],
        [0x00000000, 0x3e4ccccd, 0x3f700000],
        [0x00000000, 0x3f99999a, 0x3f09999a],
    ],
];
const UVS: [[[u32; 2]; 30]; 3] = [
    [
        [0x3ce00000, 0x3eac0000],
        [0x3ce00000, 0x3eb00000],
        [0x3d100000, 0x3eb00000],
        [0x3ce00000, 0x3eac0000],
        [0x3d100000, 0x3eb00000],
        [0x3d100000, 0x3eac0000],
        [0x00000000, 0x3ea00000],
        [0x00000000, 0x3ebffae1],
        [0x3d7fd70a, 0x3ebffae1],
        [0x00000000, 0x3ea00000],
        [0x3d7fd70a, 0x3ebffae1],
        [0x3d7fd70a, 0x3ea00000],
        [0x00000000, 0x3ea00000],
        [0x00000000, 0x3ebffae1],
        [0x3d7fd70a, 0x3ebffae1],
        [0x00000000, 0x3ea00000],
        [0x3d7fd70a, 0x3ebffae1],
        [0x3d7fd70a, 0x3ea00000],
        [0x00000000, 0x3ea00000],
        [0x00000000, 0x3ebffae1],
        [0x3d7fd70a, 0x3ebffae1],
        [0x00000000, 0x3ea00000],
        [0x3d7fd70a, 0x3ebffae1],
        [0x3d7fd70a, 0x3ea00000],
        [0x00000000, 0x3ea00000],
        [0x00000000, 0x3ebffae1],
        [0x3d7fd70a, 0x3ebffae1],
        [0x00000000, 0x3ea00000],
        [0x3d7fd70a, 0x3ebffae1],
        [0x3d7fd70a, 0x3ea00000],
    ],
    [
        [0x3e5c0000, 0x3eec0000],
        [0x3e5c0000, 0x3ef00000],
        [0x3e640000, 0x3ef00000],
        [0x3e5c0000, 0x3eec0000],
        [0x3e640000, 0x3ef00000],
        [0x3e640000, 0x3eec0000],
        [0x3e400000, 0x3ee00000],
        [0x3e400000, 0x3efffae1],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e400000, 0x3ee00000],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e7ff5c2, 0x3ee00000],
        [0x3e400000, 0x3ee00000],
        [0x3e400000, 0x3efffae1],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e400000, 0x3ee00000],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e7ff5c2, 0x3ee00000],
        [0x3e400000, 0x3ee00000],
        [0x3e400000, 0x3efffae1],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e400000, 0x3ee00000],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e7ff5c2, 0x3ee00000],
        [0x3e400000, 0x3ee00000],
        [0x3e400000, 0x3efffae1],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e400000, 0x3ee00000],
        [0x3e7ff5c2, 0x3efffae1],
        [0x3e7ff5c2, 0x3ee00000],
    ],
    [
        [0x3e5c0000, 0x3ecc0000],
        [0x3e5c0000, 0x3ed00000],
        [0x3e640000, 0x3ed00000],
        [0x3e5c0000, 0x3ecc0000],
        [0x3e640000, 0x3ed00000],
        [0x3e640000, 0x3ecc0000],
        [0x3e400000, 0x3ec00000],
        [0x3e400000, 0x3edffae1],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e400000, 0x3ec00000],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e7ff5c2, 0x3ec00000],
        [0x3e400000, 0x3ec00000],
        [0x3e400000, 0x3edffae1],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e400000, 0x3ec00000],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e7ff5c2, 0x3ec00000],
        [0x3e400000, 0x3ec00000],
        [0x3e400000, 0x3edffae1],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e400000, 0x3ec00000],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e7ff5c2, 0x3ec00000],
        [0x3e400000, 0x3ec00000],
        [0x3e400000, 0x3edffae1],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e400000, 0x3ec00000],
        [0x3e7ff5c2, 0x3edffae1],
        [0x3e7ff5c2, 0x3ec00000],
    ],
];

pub(crate) fn append(
    mesh: &mut Mesh,
    pos: [i32; 3],
    block: BlockData,
    minimum_light: u8,
    limits: MeshLimits,
) -> Result<(), ViewError> {
    let meta = block.metadata;
    let model = if (1..=4).contains(&meta) {
        usize::from(meta)
    } else {
        0
    };
    let tile = match block.id {
        50 => 0,
        75 => 1,
        76 => 2,
        _ => unreachable!(),
    };
    if mesh.vertices.len().saturating_add(30) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(30) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 30,
            indices: mesh.indices.len() + 30,
        });
    }
    let base = mesh.vertices.len() as u32;
    for i in 0..30 {
        let bits = POSITIONS[model][i];
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
    mesh.visible_faces += 5;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_torch_orientation_matches_native_vertices_and_uvs() {
        let corpus = include_str!("../../../tests/fixtures/torch_render/native.tsv");
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
                    block_light: 7,
                    sky_light: 13,
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
        assert_eq!(seen, 3 * 16 * 30);
    }
}
