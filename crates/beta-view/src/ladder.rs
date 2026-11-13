//! Beta ladder quads for the four native wall metadata values.
use crate::{Mesh, MeshLimits, Vertex, ViewError};
use beta_core::chunk::BlockData;
const VERTICES: [[[u32; 5]; 6]; 4] = [
    [
        [0x3f800000, 0x3f800000, 0x3f733333, 0x3e400000, 0x3ea00000],
        [0x3f800000, 0x00000000, 0x3f733333, 0x3e400000, 0x3ebffae1],
        [0x00000000, 0x00000000, 0x3f733333, 0x3e7ff5c2, 0x3ebffae1],
        [0x3f800000, 0x3f800000, 0x3f733333, 0x3e400000, 0x3ea00000],
        [0x00000000, 0x00000000, 0x3f733333, 0x3e7ff5c2, 0x3ebffae1],
        [0x00000000, 0x3f800000, 0x3f733333, 0x3e7ff5c2, 0x3ea00000],
    ],
    [
        [0x3f800000, 0x00000000, 0x3d4ccccd, 0x3e7ff5c2, 0x3ebffae1],
        [0x3f800000, 0x3f800000, 0x3d4ccccd, 0x3e7ff5c2, 0x3ea00000],
        [0x00000000, 0x3f800000, 0x3d4ccccd, 0x3e400000, 0x3ea00000],
        [0x3f800000, 0x00000000, 0x3d4ccccd, 0x3e7ff5c2, 0x3ebffae1],
        [0x00000000, 0x3f800000, 0x3d4ccccd, 0x3e400000, 0x3ea00000],
        [0x00000000, 0x00000000, 0x3d4ccccd, 0x3e400000, 0x3ebffae1],
    ],
    [
        [0x3f733333, 0x00000000, 0x3f800000, 0x3e7ff5c2, 0x3ebffae1],
        [0x3f733333, 0x3f800000, 0x3f800000, 0x3e7ff5c2, 0x3ea00000],
        [0x3f733333, 0x3f800000, 0x00000000, 0x3e400000, 0x3ea00000],
        [0x3f733333, 0x00000000, 0x3f800000, 0x3e7ff5c2, 0x3ebffae1],
        [0x3f733333, 0x3f800000, 0x00000000, 0x3e400000, 0x3ea00000],
        [0x3f733333, 0x00000000, 0x00000000, 0x3e400000, 0x3ebffae1],
    ],
    [
        [0x3d4ccccd, 0x3f800000, 0x3f800000, 0x3e400000, 0x3ea00000],
        [0x3d4ccccd, 0x00000000, 0x3f800000, 0x3e400000, 0x3ebffae1],
        [0x3d4ccccd, 0x00000000, 0x00000000, 0x3e7ff5c2, 0x3ebffae1],
        [0x3d4ccccd, 0x3f800000, 0x3f800000, 0x3e400000, 0x3ea00000],
        [0x3d4ccccd, 0x00000000, 0x00000000, 0x3e7ff5c2, 0x3ebffae1],
        [0x3d4ccccd, 0x3f800000, 0x00000000, 0x3e7ff5c2, 0x3ea00000],
    ],
];

pub(crate) fn append(
    mesh: &mut Mesh,
    pos: [i32; 3],
    block: BlockData,
    minimum_light: u8,
    limits: MeshLimits,
) -> Result<(), ViewError> {
    if !(2..=5).contains(&block.metadata) {
        return Ok(());
    }
    if mesh.vertices.len().saturating_add(6) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(6) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 6,
            indices: mesh.indices.len() + 6,
        });
    }
    let base = mesh.vertices.len() as u32;
    for (i, bits) in VERTICES[usize::from(block.metadata - 2)].iter().enumerate() {
        mesh.vertices.push(Vertex {
            position: [
                pos[0] as f32 + f32::from_bits(bits[0]),
                pos[1] as f32 + f32::from_bits(bits[1]),
                pos[2] as f32 + f32::from_bits(bits[2]),
            ],
            uv: [f32::from_bits(bits[3]), f32::from_bits(bits[4])],
            block_light: block.block_light,
            sky_light: block.sky_light,
            minimum_light,
            shade: 1.0,
            tint: [1.0; 3],
            tint_after_light: false,
        });
        mesh.indices.push(base + i as u32);
    }
    mesh.visible_faces += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ladder_all_metadata_native_fixture() {
        let data = include_str!("../../../tests/fixtures/ladder_render/native.tsv");
        let mut count = 0;
        for line in data.lines() {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] == "case" {
                let meta = row[2].parse().unwrap();
                let mut mesh = Mesh::default();
                append(
                    &mut mesh,
                    [0, 0, 0],
                    BlockData {
                        id: 65,
                        metadata: meta,
                        block_light: 0,
                        sky_light: 15,
                    },
                    0,
                    MeshLimits::default(),
                )
                .unwrap();
                assert_eq!(
                    mesh.vertices.len(),
                    row[4].parse::<usize>().unwrap(),
                    "metadata {meta}"
                );
            } else if row[0] == "vertex" {
                let meta = row[2].parse().unwrap();
                let i: usize = row[3].parse().unwrap();
                let mut mesh = Mesh::default();
                append(
                    &mut mesh,
                    [0, 0, 0],
                    BlockData {
                        id: 65,
                        metadata: meta,
                        block_light: 0,
                        sky_light: 15,
                    },
                    0,
                    MeshLimits::default(),
                )
                .unwrap();
                let bits: Vec<u32> = row[4..9]
                    .iter()
                    .map(|v| u32::from_str_radix(v, 16).unwrap())
                    .collect();
                let v = mesh.vertices[i];
                assert_eq!(
                    v.position.map(f32::to_bits),
                    [bits[0], bits[1], bits[2]],
                    "meta {meta} index {i}"
                );
                assert_eq!(
                    v.uv.map(f32::to_bits),
                    [bits[3], bits[4]],
                    "meta {meta} index {i}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 4 * 6);
    }
}
