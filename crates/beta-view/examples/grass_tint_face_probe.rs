//! Headless RGB vertex colors for synthetic grass/stone meshes.
use beta_core::{
    chunk::{BlockData, ChunkData},
    light::{LightDimension, brightness},
};
use beta_view::{BlockMapping, BlockStateKey, MeshLimits, Scene, SceneBounds, SceneChunk};
use glam::Vec3;
use std::collections::HashMap;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for tint in [0x3aa457_u32, 0xa1b2c3, 0xffffff, 0x050b13] {
        for phase in 0..2 {
            let mut data = ChunkData::new();
            for x in 0..16 {
                for y in 0..128 {
                    for z in 0..16 {
                        let id = if y == 64 && z == 4 && (x == 4 || x == 5) {
                            2
                        } else if phase == 1 && (x, y, z) == (4, 66, 4) {
                            1
                        } else {
                            0
                        };
                        let sky_light = if phase == 1 && (x, y, z) == (4, 65, 4) {
                            14
                        } else {
                            15
                        };
                        data.set(
                            x,
                            y,
                            z,
                            BlockData {
                                id,
                                sky_light,
                                ..Default::default()
                            },
                        )?;
                    }
                }
            }
            let mappings = (1..=2)
                .map(|id| BlockMapping {
                    key: BlockStateKey {
                        id,
                        metadata: 0,
                        above_id: None,
                    },
                    render_type: 0,
                    opaque: true,
                    full_cube: true,
                    alpha_opaque_faces: [true; 6],
                    tiles: if id == 2 { [2, 0, 3, 3, 3, 3] } else { [1; 6] },
                    supported: true,
                })
                .collect();
            let scene = Scene::new(
                SceneBounds::new(0, 0, 1, 1)?,
                vec![SceneChunk { x: 0, z: 0, data }],
                mappings,
            )?;
            let mesh = scene.mesh_with_tint(MeshLimits::default(), |_, _, _, _| tint)?;
            let mut layers = HashMap::new();
            for q in mesh.vertices.chunks_exact(4) {
                let a = Vec3::from_array(q[0].position);
                let b = Vec3::from_array(q[1].position);
                let c = Vec3::from_array(q[2].position);
                let n = (b - a).cross(c - a);
                let center =
                    q.iter().map(|v| Vec3::from_array(v.position)).sum::<Vec3>() / 4.0 - n / 2.0;
                let (x, y, z) = (
                    center.x.floor() as i32,
                    center.y.floor() as i32,
                    center.z.floor() as i32,
                );
                let face = if n.x < 0.0 {
                    4
                } else if n.x > 0.0 {
                    5
                } else if n.y < 0.0 {
                    0
                } else if n.y > 0.0 {
                    1
                } else if n.z < 0.0 {
                    2
                } else {
                    3
                };
                let layer = layers.entry((x, y, z, face)).or_insert(0);
                let current = *layer;
                *layer += 1;
                for (vertex, v) in q.iter().enumerate() {
                    let light = brightness(v.sky_light, LightDimension::Overworld)?;
                    let color: [u32; 3] = std::array::from_fn(|channel| {
                        let value = if v.tint_after_light {
                            (v.shade * light) * v.tint[channel]
                        } else {
                            (v.shade * v.tint[channel]) * light
                        };
                        (value * 255.0).trunc().clamp(0.0, 255.0) as u32
                    });
                    let packed = 0xff00_0000 | (color[2] << 16) | (color[1] << 8) | color[0];
                    println!(
                        "{tint:06x}\t{phase}\t{x}\t{y}\t{z}\t{face}\t{current}\t{vertex}\t{packed:08x}"
                    );
                }
            }
        }
    }
    Ok(())
}
