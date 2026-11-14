//! Wheat crop quads and stage textures captured from the Beta client.
use crate::{Mesh, MeshLimits, Vertex, ViewError};
use beta_core::chunk::BlockData;
const POSITIONS: [[u32; 3]; 48] = [
    [0x3e800000, 0x3f700000, 0x00000000],
    [0x3e800000, 0xbd800000, 0x00000000],
    [0x3e800000, 0xbd800000, 0x3f800000],
    [0x3e800000, 0x3f700000, 0x00000000],
    [0x3e800000, 0xbd800000, 0x3f800000],
    [0x3e800000, 0x3f700000, 0x3f800000],
    [0x3e800000, 0x3f700000, 0x3f800000],
    [0x3e800000, 0xbd800000, 0x3f800000],
    [0x3e800000, 0xbd800000, 0x00000000],
    [0x3e800000, 0x3f700000, 0x3f800000],
    [0x3e800000, 0xbd800000, 0x00000000],
    [0x3e800000, 0x3f700000, 0x00000000],
    [0x3f400000, 0x3f700000, 0x3f800000],
    [0x3f400000, 0xbd800000, 0x3f800000],
    [0x3f400000, 0xbd800000, 0x00000000],
    [0x3f400000, 0x3f700000, 0x3f800000],
    [0x3f400000, 0xbd800000, 0x00000000],
    [0x3f400000, 0x3f700000, 0x00000000],
    [0x3f400000, 0x3f700000, 0x00000000],
    [0x3f400000, 0xbd800000, 0x00000000],
    [0x3f400000, 0xbd800000, 0x3f800000],
    [0x3f400000, 0x3f700000, 0x00000000],
    [0x3f400000, 0xbd800000, 0x3f800000],
    [0x3f400000, 0x3f700000, 0x3f800000],
    [0x00000000, 0x3f700000, 0x3e800000],
    [0x00000000, 0xbd800000, 0x3e800000],
    [0x3f800000, 0xbd800000, 0x3e800000],
    [0x00000000, 0x3f700000, 0x3e800000],
    [0x3f800000, 0xbd800000, 0x3e800000],
    [0x3f800000, 0x3f700000, 0x3e800000],
    [0x3f800000, 0x3f700000, 0x3e800000],
    [0x3f800000, 0xbd800000, 0x3e800000],
    [0x00000000, 0xbd800000, 0x3e800000],
    [0x3f800000, 0x3f700000, 0x3e800000],
    [0x00000000, 0xbd800000, 0x3e800000],
    [0x00000000, 0x3f700000, 0x3e800000],
    [0x3f800000, 0x3f700000, 0x3f400000],
    [0x3f800000, 0xbd800000, 0x3f400000],
    [0x00000000, 0xbd800000, 0x3f400000],
    [0x3f800000, 0x3f700000, 0x3f400000],
    [0x00000000, 0xbd800000, 0x3f400000],
    [0x00000000, 0x3f700000, 0x3f400000],
    [0x00000000, 0x3f700000, 0x3f400000],
    [0x00000000, 0xbd800000, 0x3f400000],
    [0x3f800000, 0xbd800000, 0x3f400000],
    [0x00000000, 0x3f700000, 0x3f400000],
    [0x3f800000, 0xbd800000, 0x3f400000],
    [0x3f800000, 0x3f700000, 0x3f400000],
];
const UV_SELECT: [[usize; 2]; 48] = [
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
    [0, 0],
    [0, 1],
    [1, 1],
    [0, 0],
    [1, 1],
    [1, 0],
];
const UV_BOUNDS: [[[u32; 2]; 2]; 16] = [
    [[0x3f000000, 0x3f0ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f100000, 0x3f1ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f200000, 0x3f2ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f300000, 0x3f3ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f400000, 0x3f4ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f500000, 0x3f5ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f600000, 0x3f6ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x3f700000, 0x3f7ffd71], [0x3ea00000, 0x3ebffae1]],
    [[0x00000000, 0x3d7fd70a], [0x3ec00000, 0x3edffae1]],
    [[0x3d800000, 0x3dffeb85], [0x3ec00000, 0x3edffae1]],
    [[0x3e000000, 0x3e3ff5c2], [0x3ec00000, 0x3edffae1]],
    [[0x3e400000, 0x3e7ff5c2], [0x3ec00000, 0x3edffae1]],
    [[0x3e800000, 0x3e9ffae1], [0x3ec00000, 0x3edffae1]],
    [[0x3ea00000, 0x3ebffae1], [0x3ec00000, 0x3edffae1]],
    [[0x3ec00000, 0x3edffae1], [0x3ec00000, 0x3edffae1]],
    [[0x3ee00000, 0x3efffae1], [0x3ec00000, 0x3edffae1]],
];
pub(crate) fn append(
    mesh: &mut Mesh,
    pos: [i32; 3],
    block: BlockData,
    minimum_light: u8,
    limits: MeshLimits,
) -> Result<(), ViewError> {
    if mesh.vertices.len().saturating_add(48) > limits.max_vertices.min(crate::MAX_MESH_VERTICES)
        || mesh.indices.len().saturating_add(48) > limits.max_indices.min(crate::MAX_MESH_INDICES)
    {
        return Err(ViewError::MeshLimitExceeded {
            vertices: mesh.vertices.len() + 48,
            indices: mesh.indices.len() + 48,
        });
    }
    let base = mesh.vertices.len() as u32;
    let uv = UV_BOUNDS[usize::from(block.metadata & 15)];
    for (i, xyz) in POSITIONS.iter().enumerate() {
        let select = UV_SELECT[i];
        mesh.vertices.push(Vertex {
            position: [
                f32::from_bits(xyz[0]) + pos[0] as f32,
                f32::from_bits(xyz[1]) + pos[1] as f32,
                f32::from_bits(xyz[2]) + pos[2] as f32,
            ],
            uv: [
                f32::from_bits(uv[0][select[0]]),
                f32::from_bits(uv[1][select[1]]),
            ],
            block_light: block.block_light,
            sky_light: block.sky_light,
            minimum_light,
            shade: 1.0,
            tint: [1.0; 3],
            tint_after_light: false,
        });
        mesh.indices.push(base + i as u32);
    }
    mesh.visible_faces += 8;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheat_all_stages_match_native_vertices() {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/crops_render/native.tsv")
            .lines()
            .filter(|line| line.starts_with("vertex\t"))
        {
            let row: Vec<_> = line.split('\t').collect();
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
                    id: 59,
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
                "meta={meta} vertex={index}"
            );
            assert_eq!(
                v.uv.map(f32::to_bits),
                [bits[3], bits[4]],
                "meta={meta} vertex={index}"
            );
            count += 1;
        }
        assert_eq!(count, 16 * 48);
    }
}
