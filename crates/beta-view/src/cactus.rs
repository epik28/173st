//! Beta cactus geometry captured from the original client.
use crate::{Mesh, MeshLimits, Vertex, ViewError};
use beta_core::chunk::BlockData;
const VERTICES: [[u32; 5]; 36] = [
    [0x00000000, 0x00000000, 0x3f800000, 0x3ee00000, 0x3e9ffae1],
    [0x00000000, 0x00000000, 0x00000000, 0x3ee00000, 0x3e800000],
    [0x3f800000, 0x00000000, 0x00000000, 0x3efffae1, 0x3e800000],
    [0x00000000, 0x00000000, 0x3f800000, 0x3ee00000, 0x3e9ffae1],
    [0x3f800000, 0x00000000, 0x00000000, 0x3efffae1, 0x3e800000],
    [0x3f800000, 0x00000000, 0x3f800000, 0x3efffae1, 0x3e9ffae1],
    [0x3f800000, 0x3f800000, 0x3f800000, 0x3ebffae1, 0x3e9ffae1],
    [0x3f800000, 0x3f800000, 0x00000000, 0x3ebffae1, 0x3e800000],
    [0x00000000, 0x3f800000, 0x00000000, 0x3ea00000, 0x3e800000],
    [0x3f800000, 0x3f800000, 0x3f800000, 0x3ebffae1, 0x3e9ffae1],
    [0x00000000, 0x3f800000, 0x00000000, 0x3ea00000, 0x3e800000],
    [0x00000000, 0x3f800000, 0x3f800000, 0x3ea00000, 0x3e9ffae1],
    [0x00000000, 0x3f800000, 0x3d800000, 0x3edffae1, 0x3e800000],
    [0x3f800000, 0x3f800000, 0x3d800000, 0x3ec00000, 0x3e800000],
    [0x3f800000, 0x00000000, 0x3d800000, 0x3ec00000, 0x3e9ffae1],
    [0x00000000, 0x3f800000, 0x3d800000, 0x3edffae1, 0x3e800000],
    [0x3f800000, 0x00000000, 0x3d800000, 0x3ec00000, 0x3e9ffae1],
    [0x00000000, 0x00000000, 0x3d800000, 0x3edffae1, 0x3e9ffae1],
    [0x00000000, 0x3f800000, 0x3f700000, 0x3ec00000, 0x3e800000],
    [0x00000000, 0x00000000, 0x3f700000, 0x3ec00000, 0x3e9ffae1],
    [0x3f800000, 0x00000000, 0x3f700000, 0x3edffae1, 0x3e9ffae1],
    [0x00000000, 0x3f800000, 0x3f700000, 0x3ec00000, 0x3e800000],
    [0x3f800000, 0x00000000, 0x3f700000, 0x3edffae1, 0x3e9ffae1],
    [0x3f800000, 0x3f800000, 0x3f700000, 0x3edffae1, 0x3e800000],
    [0x3d800000, 0x3f800000, 0x3f800000, 0x3edffae1, 0x3e800000],
    [0x3d800000, 0x3f800000, 0x00000000, 0x3ec00000, 0x3e800000],
    [0x3d800000, 0x00000000, 0x00000000, 0x3ec00000, 0x3e9ffae1],
    [0x3d800000, 0x3f800000, 0x3f800000, 0x3edffae1, 0x3e800000],
    [0x3d800000, 0x00000000, 0x00000000, 0x3ec00000, 0x3e9ffae1],
    [0x3d800000, 0x00000000, 0x3f800000, 0x3edffae1, 0x3e9ffae1],
    [0x3f700000, 0x00000000, 0x3f800000, 0x3ec00000, 0x3e9ffae1],
    [0x3f700000, 0x00000000, 0x00000000, 0x3edffae1, 0x3e9ffae1],
    [0x3f700000, 0x3f800000, 0x00000000, 0x3edffae1, 0x3e800000],
    [0x3f700000, 0x00000000, 0x3f800000, 0x3ec00000, 0x3e9ffae1],
    [0x3f700000, 0x3f800000, 0x00000000, 0x3edffae1, 0x3e800000],
    [0x3f700000, 0x3f800000, 0x3f800000, 0x3ec00000, 0x3e800000],
];
pub(crate) fn is_visible(_face: u8, neighbor_opaque: bool) -> bool {
    !neighbor_opaque
}

pub(crate) fn append_face(
    mesh: &mut Mesh,
    pos: [i32; 3],
    _block: BlockData,
    neighbor: BlockData,
    face: u8,
    minimum_light: u8,
    limits: MeshLimits,
) -> Result<(), ViewError> {
    if mesh.vertices.len().saturating_add(6) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(6) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 6,
            indices: mesh.indices.len() + 6,
        });
    }
    let base = mesh.vertices.len() as u32;
    for (i, bits) in VERTICES
        .iter()
        .enumerate()
        .skip(usize::from(face) * 6)
        .take(6)
    {
        let light = neighbor;
        mesh.vertices.push(Vertex {
            position: [
                f32::from_bits(bits[0]) + pos[0] as f32,
                f32::from_bits(bits[1]) + pos[1] as f32,
                f32::from_bits(bits[2]) + pos[2] as f32,
            ],
            uv: [f32::from_bits(bits[3]), f32::from_bits(bits[4])],
            block_light: light.block_light,
            sky_light: light.sky_light,
            minimum_light,
            shade: crate::face_shade(face)?,
            tint: [1.0; 3],
            tint_after_light: false,
        });
        mesh.indices.push(base + (i % 6) as u32);
    }
    mesh.visible_faces += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cactus_visibility_matches_native_air_stone_and_cactus() {
        for line in include_str!("../../../tests/fixtures/cactus_render/visibility.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            let neighbor: u8 = fields[0].parse().unwrap();
            let face: u8 = fields[1].parse().unwrap();
            assert_eq!(
                is_visible(face, neighbor == 1),
                fields[2] == "1",
                "neighbor={neighbor} face={face}"
            );
        }
    }

    #[test]
    fn all_cactus_states_match_native_vertices() {
        let corpus = include_str!("../../../tests/fixtures/cactus_render/native.tsv");
        let mut count = 0;
        for line in corpus.lines().filter(|line| line.starts_with("vertex\t")) {
            let row: Vec<_> = line.split('\t').collect();
            let meta = row[2].parse().unwrap();
            let index: usize = row[3].parse().unwrap();
            let bits: Vec<u32> = row[4..9]
                .iter()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            let mut mesh = Mesh::default();
            let block = BlockData {
                id: 81,
                metadata: meta,
                block_light: 0,
                sky_light: 15,
            };
            append_face(
                &mut mesh,
                [0, 0, 0],
                block,
                block,
                (index / 6) as u8,
                0,
                MeshLimits::default(),
            )
            .unwrap();
            let v = mesh.vertices[index % 6];
            assert_eq!(
                v.position.map(f32::to_bits),
                [bits[0], bits[1], bits[2]],
                "meta={meta} vertex={index}"
            );
            assert_eq!(
                v.uv.map(f32::to_bits),
                [bits[3], bits[4]],
                "meta={meta} vertex={index}"
            );
            count += 1;
        }
        assert_eq!(count, 16 * 36);
    }
}
