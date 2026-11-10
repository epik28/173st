use beta_core::{
    block::{BlockHit, BlockHitKind, BlockScene, BlockSceneBounds, BlockSceneChunk},
    chunk::{BlockData, ChunkData, split_block_coord},
    geometry::Vec3,
};

fn pair(field: &str) -> (u8, u8) {
    let (id, metadata) = field.split_once(':').expect("id:metadata");
    (id.parse().unwrap(), metadata.parse().unwrap())
}

fn scene_for(x: i32, y: i32, z: i32, target: (u8, u8), adjacent: (u8, u8), face: u8) -> BlockScene {
    let offsets = [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ];
    let (dx, dy, dz) = offsets[usize::from(face)];
    let adjacent_pos = (x + dx, y + dy, z + dz);
    let target_chunk = (split_block_coord(x).0, split_block_coord(z).0);
    let adjacent_chunk = (
        split_block_coord(adjacent_pos.0).0,
        split_block_coord(adjacent_pos.2).0,
    );
    let mut chunks = Vec::new();
    for cx in -1..=1 {
        for cz in -1..=1 {
            let mut data = ChunkData::new();
            if (cx, cz) == target_chunk {
                let (_, lx) = split_block_coord(x);
                let (_, lz) = split_block_coord(z);
                data.set(
                    lx,
                    y as u8,
                    lz,
                    BlockData {
                        id: target.0,
                        metadata: target.1,
                        ..BlockData::default()
                    },
                )
                .unwrap();
            }
            if adjacent.0 != 0 && (cx, cz) == adjacent_chunk {
                let (_, lx) = split_block_coord(adjacent_pos.0);
                let (_, lz) = split_block_coord(adjacent_pos.2);
                data.set(
                    lx,
                    adjacent_pos.1 as u8,
                    lz,
                    BlockData {
                        id: adjacent.0,
                        metadata: adjacent.1,
                        ..BlockData::default()
                    },
                )
                .unwrap();
            }
            chunks.push(BlockSceneChunk { x: cx, z: cz, data });
        }
    }
    BlockScene::new(BlockSceneBounds::new(-1, -1, 3, 3).unwrap(), chunks).unwrap()
}

#[test]
fn all_native_placement_action_rows_match_block_scene_results() {
    let fixture = include_str!("../../../tests/fixtures/block-placement/native.tsv");
    let mut rows = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        let case = fields[0];
        let target_id: u8 = fields[1].parse().unwrap();
        let target_meta: u8 = fields[2].parse().unwrap();
        let x: i32 = fields[3].parse().unwrap();
        let y: i32 = fields[4].parse().unwrap();
        let z: i32 = fields[5].parse().unwrap();
        let face: u8 = fields[6].parse().unwrap();
        let adjacent_before = pair(fields[10]);
        let adjacent_after = pair(fields[11]);
        let mut world = scene_for(x, y, z, (target_id, target_meta), adjacent_before, face);
        let kind = if target_id == 31 {
            BlockHitKind::TallGrass
        } else {
            BlockHitKind::FullCube
        };
        let hit = BlockHit {
            x,
            y,
            z,
            face,
            position: Vec3::new(f64::from(x), f64::from(y), f64::from(z)),
            kind,
            id: target_id,
            metadata: target_meta,
        };
        let expected_actions: Vec<_> = fields[9].split(',').map(|v| v == "true").collect();
        for expected in expected_actions {
            let result = if target_id == 31 {
                world.place_from_tallgrass_hit(hit, 1, 0)
            } else {
                world.place_block(hit, 1, 0, &|id, meta| id == 1 && meta == 0)
            };
            assert_eq!(result.is_ok(), expected, "{case}");
        }
        let adjacent_pos = {
            let offsets = [
                (0, -1, 0),
                (0, 1, 0),
                (0, 0, -1),
                (0, 0, 1),
                (-1, 0, 0),
                (1, 0, 0),
            ];
            let (dx, dy, dz) = offsets[usize::from(face)];
            (x + dx, y + dy, z + dz)
        };
        let target = world.block(x, y, z).unwrap().unwrap();
        let adjacent = world
            .block(adjacent_pos.0, adjacent_pos.1, adjacent_pos.2)
            .unwrap()
            .unwrap();
        let (expected_target_id, expected_target_meta) =
            if target_id == 31 && expected_actions_are_success(fields[9]) {
                (0, 0)
            } else {
                (target_id, target_meta)
            };
        assert_eq!(
            (target.id, target.metadata),
            (expected_target_id, expected_target_meta),
            "{case}"
        );
        assert_eq!((adjacent.id, adjacent.metadata), adjacent_after, "{case}");
        rows += 1;
    }
    assert_eq!(rows, 16);
}

fn expected_actions_are_success(actions: &str) -> bool {
    actions.split(',').next() == Some("true")
}
