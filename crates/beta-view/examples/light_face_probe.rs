use beta_core::{
    chunk::{BlockData, ChunkData},
    light::{LightDimension, brightness},
};
use beta_view::{BlockMapping, BlockStateKey, MeshLimits, Scene, SceneBounds, SceneChunk};
use glam::Vec3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for phase in 0..=1 {
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
                tiles: [1; 6],
                supported: true,
            })
            .collect();
        let scene = Scene::new(
            SceneBounds::new(0, 0, 1, 1)?,
            vec![SceneChunk { x: 0, z: 0, data }],
            mappings,
        )?;
        let mesh = scene.mesh(MeshLimits::default())?;
        for quad in mesh.vertices.chunks_exact(4) {
            let a = Vec3::from_array(quad[0].position);
            let b = Vec3::from_array(quad[1].position);
            let c = Vec3::from_array(quad[2].position);
            let n = (b - a).cross(c - a);
            let centre = quad
                .iter()
                .map(|v| Vec3::from_array(v.position))
                .sum::<Vec3>()
                / 4.0
                - n / 2.0;
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
            let level = quad[0].sky_light;
            let channel = (quad[0].shade * brightness(level, LightDimension::Overworld)? * 255.0)
                .trunc() as u32;
            let color = 0xff00_0000 | (channel * 0x010101);
            println!(
                "{phase}\t{}\t{}\t{}\t{face}\t{color:08x}",
                centre.x.floor(),
                centre.y.floor(),
                centre.z.floor()
            );
        }
    }
    Ok(())
}
