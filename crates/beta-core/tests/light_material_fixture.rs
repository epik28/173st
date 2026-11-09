//! Light-only regressions against complete native pre-edit chunk snapshots.
//! Leaf decay/metadata notification and fluid ticks are outside this test.

use beta_core::{
    block::{BlockScene, BlockSceneBounds, BlockSceneChunk, MAX_RELIGHT_CELLS},
    chunk::{BlockData, ChunkData, split_block_coord},
    light::{combined_light, emission_level, opacity_level},
};
use std::collections::BTreeMap;

const NATIVE: &str = include_str!("../../../tests/fixtures/light_material/native_edits.tsv");

fn position(fields: &[&str]) -> (i32, i32, i32) {
    (
        fields[0].parse().unwrap(),
        fields[1].parse().unwrap(),
        fields[2].parse().unwrap(),
    )
}

fn native_scene(name: &str, edit: Option<((i32, i32, i32), u8)>) -> BlockScene {
    let mut chunks = BTreeMap::new();
    for x in -1..=1 {
        for z in -1..=1 {
            let mut data = ChunkData::new();
            for lx in 0..16 {
                for lz in 0..16 {
                    for y in 1..128 {
                        data.set(
                            lx,
                            y,
                            lz,
                            BlockData {
                                sky_light: 15,
                                ..BlockData::default()
                            },
                        )
                        .unwrap();
                    }
                }
            }
            chunks.insert((x, z), data);
        }
    }
    let mut states = 0;
    for line in NATIVE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] != "state" || row[1] != name {
            continue;
        }
        assert_eq!(row.len(), 9);
        let (x, y, z) = position(&row[2..5]);
        let (cx, lx) = split_block_coord(x);
        let (cz, lz) = split_block_coord(z);
        chunks
            .get_mut(&(cx, cz))
            .unwrap()
            .set(
                lx,
                y.try_into().unwrap(),
                lz,
                BlockData {
                    id: row[5].parse().unwrap(),
                    metadata: row[6].parse().unwrap(),
                    block_light: row[7].parse().unwrap(),
                    sky_light: row[8].parse().unwrap(),
                },
            )
            .unwrap();
        states += 1;
    }
    assert!(
        states > 0,
        "missing complete native pre-edit state for {name}"
    );
    if let Some(((x, y, z), id)) = edit {
        let (cx, lx) = split_block_coord(x);
        let (cz, lz) = split_block_coord(z);
        let data = chunks.get_mut(&(cx, cz)).unwrap();
        let old = data.get(lx, y.try_into().unwrap(), lz).unwrap();
        data.set(
            lx,
            y.try_into().unwrap(),
            lz,
            BlockData {
                id,
                metadata: 0,
                ..old
            },
        )
        .unwrap();
    }
    BlockScene::new(
        BlockSceneBounds::new(-1, -1, 3, 3).unwrap(),
        chunks
            .into_iter()
            .map(|((x, z), data)| BlockSceneChunk { x, z, data })
            .collect(),
    )
    .unwrap()
}

fn assert_samples(scene: &BlockScene, name: &str, phase: &str) -> usize {
    let mut samples = 0;
    for line in NATIVE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] != name || row[1] != phase {
            continue;
        }
        assert_eq!(row.len(), 12);
        let (x, y, z) = position(&row[2..5]);
        let block = scene.block(x, y, z).unwrap().unwrap();
        assert_eq!(block.id.to_string(), row[5], "block ID: {line}");
        // Native log removal marks neighboring leaves with metadata bit 8.
        // This suite establishes raw light only, not that lifecycle behavior.
        if phase == "before" {
            assert_eq!(
                block.metadata.to_string(),
                row[6],
                "initial metadata: {line}"
            );
        }
        assert_eq!(
            opacity_level(block.id).to_string(),
            row[7],
            "opacity: {line}"
        );
        assert_eq!(
            emission_level(block.id).to_string(),
            row[8],
            "emission: {line}"
        );
        assert_eq!(block.sky_light.to_string(), row[9], "sky: {line}");
        assert_eq!(
            block.block_light.to_string(),
            row[10],
            "block light: {line}"
        );
        assert_eq!(
            combined_light(block.block_light, block.sky_light, 0)
                .unwrap()
                .to_string(),
            row[11],
            "combined: {line}"
        );
        samples += 1;
    }
    samples
}

#[test]
fn native_canopy_and_water_edit_light_samples_match() {
    let mut cases = 0;
    let mut total_samples = 0;
    for line in NATIVE.lines().filter(|line| line.starts_with("case\t")) {
        let row: Vec<_> = line.split('\t').collect();
        let name = row[1];
        let xyz: Vec<_> = row[2].strip_prefix("target=").unwrap().split(',').collect();
        let target = position(&xyz);
        let replacement = row[3]
            .strip_prefix("replacement=")
            .unwrap()
            .parse()
            .unwrap();
        let before = native_scene(name, None);
        let before_samples = assert_samples(&before, name, "before");
        let mut edited = native_scene(name, Some((target, replacement)));
        let result = edited
            .relight(target, MAX_RELIGHT_CELLS, |id, _| {
                (opacity_level(id), emission_level(id))
            })
            .unwrap();
        let after_samples = assert_samples(&edited, name, "after");
        assert_eq!(before_samples, after_samples);
        for &((x, y, z), block_light, sky_light) in &result.light_updates {
            let actual = edited.block(x, y, z).unwrap().unwrap();
            assert_eq!(
                (actual.block_light, actual.sky_light),
                (block_light, sky_light)
            );
        }
        total_samples += after_samples;
        cases += 1;
    }
    assert_eq!(cases, 3);
    assert_eq!(total_samples, 19);
}
