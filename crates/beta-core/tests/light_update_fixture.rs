use beta_core::{
    block::{BlockScene, BlockSceneBounds, BlockSceneChunk, MAX_RELIGHT_CELLS},
    chunk::{BlockData, ChunkData},
    light::combined_light,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    x: i32,
    y: i32,
    z: i32,
    old_id: u8,
    new_id: u8,
}

type ProbeRow = (i32, i32, i32, u8, u8, u8, u8, u8, u8, u8);

const CASES: [Case; 5] = [
    Case {
        name: "remove",
        x: 8,
        y: 64,
        z: 8,
        old_id: 1,
        new_id: 0,
    },
    Case {
        name: "place",
        x: 8,
        y: 64,
        z: 8,
        old_id: 0,
        new_id: 1,
    },
    Case {
        name: "roof",
        x: 8,
        y: 127,
        z: 8,
        old_id: 0,
        new_id: 1,
    },
    Case {
        name: "edge",
        x: 15,
        y: 64,
        z: 8,
        old_id: 0,
        new_id: 1,
    },
    Case {
        name: "edge-x31",
        x: 31,
        y: 64,
        z: 8,
        old_id: 0,
        new_id: 1,
    },
];

fn scene(case: Case, edited: bool) -> BlockScene {
    let mut native = HashMap::new();
    let text = include_str!("../../../tests/fixtures/light_update/native.tsv");
    let mut active = false;
    for line in text.lines() {
        let p: Vec<_> = line.split('\t').collect();
        if p.first() == Some(&"case") {
            active = p.get(1) == Some(&case.name);
            continue;
        }
        if active && p.len() >= 9 && p[0] == "state" && p[1] == case.name {
            native.insert(
                (
                    p[2].parse::<i32>().unwrap(),
                    p[3].parse::<i32>().unwrap(),
                    p[4].parse::<i32>().unwrap(),
                ),
                (
                    p[5].parse().unwrap(),
                    p[6].parse().unwrap(),
                    p[7].parse().unwrap(),
                    p[8].parse().unwrap(),
                ),
            );
        }
        if p.first() == Some(&"final") {
            active = false;
        }
    }
    let mut chunks = Vec::new();
    for cz in -1..=1 {
        for cx in -1..=1 {
            let mut data = ChunkData::new();
            for x in 0..16u8 {
                for z in 0..16u8 {
                    for y in 1..128u8 {
                        data.set(
                            x,
                            y,
                            z,
                            BlockData {
                                id: native
                                    .get(&(
                                        cx * 16 + i32::from(x),
                                        i32::from(y),
                                        cz * 16 + i32::from(z),
                                    ))
                                    .map_or(0, |v| v.0),
                                metadata: native
                                    .get(&(
                                        cx * 16 + i32::from(x),
                                        i32::from(y),
                                        cz * 16 + i32::from(z),
                                    ))
                                    .map_or(0, |v| v.1),
                                block_light: native
                                    .get(&(
                                        cx * 16 + i32::from(x),
                                        i32::from(y),
                                        cz * 16 + i32::from(z),
                                    ))
                                    .map_or(0, |v| v.2),
                                sky_light: native
                                    .get(&(
                                        cx * 16 + i32::from(x),
                                        i32::from(y),
                                        cz * 16 + i32::from(z),
                                    ))
                                    .map(|v| v.3)
                                    .unwrap_or(if y == 0 { 0 } else { 15 }),
                            },
                        )
                        .unwrap();
                    }
                }
            }
            if cx == (case.x >> 4) && cz == (case.z >> 4) {
                let key = (case.x, case.y, case.z);
                let prior = native.get(&key).copied().unwrap_or((
                    0,
                    0,
                    0,
                    if case.y == 0 { 0 } else { 15 },
                ));
                data.set(
                    (case.x & 15) as u8,
                    case.y as u8,
                    (case.z & 15) as u8,
                    BlockData {
                        id: if edited {
                            if case.name == "remove" {
                                prior.0
                            } else {
                                case.new_id
                            }
                        } else {
                            case.old_id
                        },
                        metadata: prior.1,
                        block_light: prior.2,
                        sky_light: prior.3,
                    },
                )
                .unwrap();
            }
            chunks.push(BlockSceneChunk { x: cx, z: cz, data });
        }
    }
    BlockScene::new(BlockSceneBounds::new(-1, -1, 3, 3).unwrap(), chunks).unwrap()
}

fn expected(name: &str, stage: &str) -> Vec<ProbeRow> {
    let text = include_str!("../../../tests/fixtures/light_update/native.tsv");
    let mut active = false;
    let mut in_stage = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let p: Vec<_> = line.split('\t').collect();
        if p.first() == Some(&"case") {
            active = p.get(1) == Some(&name);
            in_stage = false;
            continue;
        }
        if !active {
            continue;
        }
        if p.len() >= 2 && p[0] == name && p[1] == stage {
            in_stage = true;
        }
        if !in_stage || p.len() < 12 || p[0] != name || p[1] != stage || p[5] == "UNLOADED" {
            continue;
        }
        out.push((
            p[2].parse().unwrap(),
            p[3].parse().unwrap(),
            p[4].parse().unwrap(),
            p[5].parse().unwrap(),
            p[6].parse().unwrap(),
            p[7].parse().unwrap(),
            p[8].parse().unwrap(),
            p[10].parse().unwrap(),
            p[9].parse().unwrap(),
            p[11].parse().unwrap(),
        ));
    }
    out
}

fn expected_count(name: &str) -> usize {
    match name {
        "remove" | "place" | "edge" => 15,
        "roof" => 14,
        "edge-x31" => 3,
        _ => 0,
    }
}

fn input_queries(name: &str) -> Vec<(i32, i32, i32)> {
    let text = include_str!("../../../tests/fixtures/light_update/input.tsv");
    for line in text.lines() {
        let p: Vec<_> = line.split('\t').collect();
        if p.len() > 1 && p[0] == "case" && p[1] == name {
            return p[8]
                .split('|')
                .nth(1)
                .unwrap_or("")
                .split(';')
                .filter_map(|q| {
                    let mut v = q.split(',');
                    Some((
                        v.next()?.parse().ok()?,
                        v.next()?.parse().ok()?,
                        v.next()?.parse().ok()?,
                    ))
                })
                .collect();
        }
    }
    Vec::new()
}

fn scene_light_snapshot(scene: &BlockScene) -> Vec<(i32, i32, i32, u8, u8, u8, u8)> {
    let mut state = Vec::with_capacity(48 * 48 * 128);
    for x in -16..32 {
        for z in -16..32 {
            for y in 0..128 {
                let block = scene.block(x, y, z).unwrap().unwrap();
                state.push((
                    x,
                    y,
                    z,
                    block.id,
                    block.metadata,
                    block.block_light,
                    block.sky_light,
                ));
            }
        }
    }
    state
}

#[test]
fn native_c01_cases_match_post_edit_light_and_combined_values() {
    for case in CASES {
        let before_scene = scene(case, false);
        let before_rows = expected(case.name, "before");
        let queries = input_queries(case.name);
        assert_eq!(
            before_rows
                .iter()
                .map(|r| (r.0, r.1, r.2))
                .collect::<Vec<_>>(),
            queries
                .iter()
                .copied()
                .filter(|(x, _, _)| *x != 32)
                .collect::<Vec<_>>(),
            "before query order for {}",
            case.name
        );
        for (x, y, z, id, meta, opacity, emission, block_light, sky_light, combined) in &before_rows
        {
            let b = before_scene.block(*x, *y, *z).unwrap().unwrap();
            assert_eq!(
                (b.id, b.metadata, b.block_light, b.sky_light),
                (*id, *meta, *block_light, *sky_light),
                "{} before ({x},{y},{z})",
                case.name
            );
            assert_eq!(
                combined_light(b.block_light, b.sky_light, 0).unwrap(),
                *combined
            );
            assert_eq!((if b.id == 0 { 0 } else { 255 }, 0), (*opacity, *emission));
        }

        let mut s = scene(case, true);
        if case.name == "remove" {
            let hit = beta_core::block::BlockHit {
                x: case.x,
                y: case.y,
                z: case.z,
                face: 1,
                position: beta_core::geometry::Vec3::new(0.0, 0.0, 0.0),
                kind: beta_core::block::BlockHitKind::FullCube,
                id: case.old_id,
                metadata: 0,
            };
            s.break_block(hit, &|id, _| id == 1).unwrap();
        }
        let expected_rows = expected(case.name, "after");
        let initial = scene(case, false);
        let before = initial.block(case.x, case.y, case.z).unwrap().unwrap();
        if case.name == "remove" {
            assert_eq!(
                before.id, case.old_id,
                "native before target id for {}",
                case.name
            );
        }
        let queries = input_queries(case.name);
        assert_eq!(
            expected_rows
                .iter()
                .map(|r| (r.0, r.1, r.2))
                .collect::<Vec<_>>(),
            queries
                .into_iter()
                .filter(|(x, _, _)| *x != 32)
                .collect::<Vec<_>>()
        );
        assert!(
            !expected_rows.is_empty(),
            "empty fixture parser for {}",
            case.name
        );
        assert_eq!(
            expected_rows.len(),
            expected_count(case.name),
            "fixture row count for {}",
            case.name
        );
        let result = s
            .relight((case.x, case.y, case.z), MAX_RELIGHT_CELLS, |id, _| {
                (if id == 0 { 0 } else { 255 }, 0)
            })
            .unwrap();
        let _got: Vec<_> = expected_rows
            .iter()
            .copied()
            .map(|(x, y, z, id, meta, opacity, emission, bl, sl, combined)| {
                let b = s.block(x, y, z).unwrap().unwrap();
                assert_eq!(
                    (b.id, b.metadata, b.block_light, b.sky_light),
                    (id, meta, bl, sl),
                    "{} ({x},{y},{z})",
                    case.name
                );
                assert_eq!(
                    combined_light(b.block_light, b.sky_light, 0).unwrap(),
                    combined
                );
                assert_eq!((if b.id == 0 { 0 } else { 255 }, 0), (opacity, emission));
                (x, y, z)
            })
            .collect();
        let updates: Vec<_> = result.light_updates.iter().map(|(p, _, _)| *p).collect();
        let sampled_light_deltas = before_rows
            .iter()
            .zip(expected_rows.iter())
            .filter_map(|(before, after)| {
                ((before.8, before.9) != (after.8, after.9)).then_some((after.0, after.1, after.2))
            })
            .collect::<Vec<_>>();
        assert!(
            sampled_light_deltas.iter().all(|p| updates.contains(p)),
            "{} sampled light deltas missing from result: {:?}",
            case.name,
            sampled_light_deltas
        );
        for ((x, y, z), block_light, sky_light) in &result.light_updates {
            let before = before_scene.block(*x, *y, *z).unwrap().unwrap();
            assert_ne!(
                (before.block_light, before.sky_light),
                (*block_light, *sky_light),
                "{} reported an unchanged light cell ({x},{y},{z})",
                case.name
            );
        }
        assert_eq!(result.changed, (case.x, case.y, case.z));
        assert!(
            updates.windows(2).all(|w| w[0] < w[1]),
            "{} updates not sorted",
            case.name
        );
        assert!(result.dirty_chunks.windows(2).all(|w| w[0] < w[1]));
        assert!(result.dirty_chunks.contains(&(case.x >> 4, case.z >> 4)));
        let mut expected_dirty = updates
            .iter()
            .map(|(x, _, z)| (*x >> 4, *z >> 4))
            .collect::<Vec<_>>();
        expected_dirty.push((case.x >> 4, case.z >> 4));
        expected_dirty.sort_unstable();
        expected_dirty.dedup();
        assert_eq!(result.dirty_chunks, expected_dirty);
        for ((x, y, z), block_light, sky_light) in &result.light_updates {
            let b = s.block(*x, *y, *z).unwrap().unwrap();
            assert_eq!((b.block_light, b.sky_light), (*block_light, *sky_light));
        }
        if case.name == "edge-x31" {
            assert!(s.block(32, 63, 8).unwrap().is_none());
            assert_eq!(
                result.unloaded_neighbors,
                vec![(32, 63, 8), (32, 64, 8), (32, 65, 8)]
            );
        } else if case.name == "roof" {
            assert!(
                result.unloaded_neighbors.is_empty(),
                "Y=128 is outside the world, not an unloaded chunk"
            );
        }
    }
}

#[test]
fn relight_failures_are_atomic_and_validate_inputs() {
    let mut s = scene(CASES[0], true);
    let before_revision = s.revision();
    let before_state = scene_light_snapshot(&s);
    assert!(s.relight((8, 64, 8), 1, |_, _| (0, 0)).is_err());
    assert!(s.relight((8, 64, 8), 128, |_, _| (0, 0)).is_err());
    assert_eq!(s.revision(), before_revision);
    assert_eq!(scene_light_snapshot(&s), before_state);
    assert!(
        s.relight((8, 128, 8), MAX_RELIGHT_CELLS, |_, _| (0, 0))
            .is_err()
    );
    assert!(
        s.relight((8, 64, 8), MAX_RELIGHT_CELLS, |_, _| (0, 16))
            .is_err()
    );
    assert_eq!(scene_light_snapshot(&s), before_state);
}

#[test]
fn relight_handles_world_coordinate_extremes_without_overflow() {
    for (chunk_x, block_x) in [(-134_217_728, i32::MIN), (134_217_727, i32::MAX)] {
        let scene = BlockScene::new(
            BlockSceneBounds::new(chunk_x, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk {
                x: chunk_x,
                z: 0,
                data: ChunkData::new(),
            }],
        )
        .unwrap();
        let mut scene = scene;
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scene.relight((block_x, 64, 0), MAX_RELIGHT_CELLS, |_, _| (0, 0))
        }))
        .expect("world-coordinate edge must not panic");
    }
}

#[test]
fn native_light_scan_skips_column_when_diagonal_radius_chunk_is_missing() {
    let mut chunks = Vec::new();
    for (cx, cz) in [(0, 0), (1, 0), (0, 1)] {
        let mut data = ChunkData::new();
        for x in 0..16u8 {
            for z in 0..16u8 {
                for y in 1..128u8 {
                    data.set(
                        x,
                        y,
                        z,
                        BlockData {
                            id: 0,
                            metadata: 0,
                            block_light: 0,
                            sky_light: 15,
                        },
                    )
                    .unwrap();
                }
            }
        }
        chunks.push(BlockSceneChunk { x: cx, z: cz, data });
    }
    let target_chunk = chunks.iter_mut().find(|c| (c.x, c.z) == (0, 0)).unwrap();
    target_chunk
        .data
        .set(
            15,
            64,
            15,
            BlockData {
                id: 1,
                metadata: 0,
                block_light: 0,
                sky_light: 0,
            },
        )
        .unwrap();
    target_chunk
        .data
        .set(
            15,
            63,
            15,
            BlockData {
                id: 0,
                metadata: 0,
                block_light: 0,
                sky_light: 0,
            },
        )
        .unwrap();

    let mut scene = BlockScene::new(BlockSceneBounds::new(0, 0, 2, 2).unwrap(), chunks).unwrap();
    let result = scene
        .relight((15, 64, 15), MAX_RELIGHT_CELLS, |id, _| {
            (if id == 0 { 0 } else { 255 }, 0)
        })
        .unwrap();

    // fd.b(x, 0, z, 1) checks every chunk touched by x±1,z±1, including
    // (1,1). Since it is unloaded, native st skips the whole (15,15) column;
    // it must not pull sky light laterally from the loaded x=14 column.
    assert_eq!(scene.block(15, 63, 15).unwrap().unwrap().sky_light, 0);
    assert!(scene.block(16, 64, 16).unwrap().is_none());
    assert_eq!(scene.block(14, 63, 15).unwrap().unwrap().sky_light, 15);
    assert!(
        result
            .light_updates
            .iter()
            .all(|(p, _, _)| *p != (15, 63, 15))
    );
}
