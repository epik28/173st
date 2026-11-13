//! Compare face colors using six native sampled light/opacity neighbors per block.
use beta_core::{
    chunk::{BlockData, ChunkData},
    light::{LightDimension, sample_brightness},
};
use beta_view::{BlockMapping, BlockStateKey, MeshLimits, Scene, SceneBounds, SceneChunk};
use glam::Vec3;
use std::io::{self, Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let offsets = [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ];
    for line in input.lines().filter(|line| line.starts_with("case\t")) {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 12 {
            return Err("invalid case".into());
        }
        let phase: i32 = fields[1].parse()?;
        let x: i32 = fields[2].parse()?;
        let y: i32 = fields[3].parse()?;
        let z: i32 = fields[4].parse()?;
        let id: u8 = fields[5].parse()?;
        let mut data = ChunkData::new();
        data.set(
            8,
            64,
            8,
            BlockData {
                id,
                ..Default::default()
            },
        )?;
        for (i, &(dx, dy, dz)) in offsets.iter().enumerate() {
            let v: Vec<u8> = fields[i + 6]
                .split(',')
                .map(str::parse)
                .collect::<Result<_, _>>()?;
            if v.len() != 3 || v[0] > 1 || v[1] > 15 || v[2] > 15 {
                return Err("invalid neighboring cell".into());
            }
            data.set(
                (8_i32 + dx) as u8,
                (64_i32 + dy) as u8,
                (8_i32 + dz) as u8,
                BlockData {
                    id: v[0],
                    sky_light: v[1],
                    block_light: v[2],
                    metadata: 0,
                },
            )?;
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
        for q in mesh.vertices.chunks_exact(4) {
            let a = Vec3::from_array(q[0].position);
            let b = Vec3::from_array(q[1].position);
            let c = Vec3::from_array(q[2].position);
            let n = (b - a).cross(c - a);
            let center =
                q.iter().map(|v| Vec3::from_array(v.position)).sum::<Vec3>() / 4.0 - n / 2.0;
            if center.x.floor() != 8.0 || center.y.floor() != 64.0 || center.z.floor() != 8.0 {
                continue;
            }
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
            let bright = sample_brightness(
                q[0].block_light,
                q[0].sky_light,
                0,
                0,
                LightDimension::Overworld,
            )?;
            let channel = (q[0].shade * bright * 255.0).trunc() as u32;
            let color = 0xff00_0000 | (channel * 0x010101);
            println!("face\t{phase}\t{x}\t{y}\t{z}\t{face}\t{color:08x}");
        }
    }
    Ok(())
}
