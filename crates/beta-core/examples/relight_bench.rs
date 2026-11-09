//! Reproducible full-output and release benchmark for the bounded relighter.
//!
//! Run with `cargo run --locked --offline --release --example relight_bench`.

use beta_core::{
    block::{BlockScene, BlockSceneBounds, BlockSceneChunk, MAX_RELIGHT_CELLS, RelightResult},
    chunk::{BlockData, ChunkData, split_block_coord},
    light::{combined_light, emission_level, opacity_level},
};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    hint::black_box,
    time::Instant,
};

const MATERIAL: &str = include_str!("../../../tests/fixtures/light_material/native_edits.tsv");
const UPDATE: &str = include_str!("../../../tests/fixtures/light_update/native.tsv");
const REPEATS: usize = 7;
const SAMPLES: usize = 60;

#[derive(Clone, Copy)]
enum Material {
    Registry,
    OpaqueNonAir,
}

struct Scenario {
    name: String,
    target: (i32, i32, i32),
    target_id: u8,
    material: Material,
    scene: BlockScene,
    timed: bool,
}

fn hash_i32(hash: &mut Sha256, value: i32) {
    hash.update(value.to_le_bytes());
}

fn hash_u64(hash: &mut Sha256, value: usize) {
    hash.update((value as u64).to_le_bytes());
}

fn finish_hash(hash: Sha256) -> String {
    format!("{:x}", hash.finalize())
}

fn scene_hash(scene: &BlockScene) -> String {
    let bounds = scene.bounds();
    let min_x = bounds.min_chunk_x * 16;
    let min_z = bounds.min_chunk_z * 16;
    let max_x = (bounds.min_chunk_x + i32::from(bounds.width)) * 16;
    let max_z = (bounds.min_chunk_z + i32::from(bounds.depth)) * 16;
    let mut hash = Sha256::new();
    hash_i32(&mut hash, bounds.min_chunk_x);
    hash_i32(&mut hash, bounds.min_chunk_z);
    hash.update([bounds.width, bounds.depth]);
    hash.update(scene.revision().to_le_bytes());
    for x in min_x..max_x {
        for z in min_z..max_z {
            for y in 0..128 {
                hash_i32(&mut hash, x);
                hash_i32(&mut hash, y);
                hash_i32(&mut hash, z);
                match scene.block(x, y, z).expect("scene coordinate is valid") {
                    Some(block) => hash.update([
                        1,
                        block.id,
                        block.metadata,
                        block.block_light,
                        block.sky_light,
                    ]),
                    None => hash.update([0, 0, 0, 0, 0]),
                }
            }
        }
    }
    finish_hash(hash)
}

fn result_hash(result: &RelightResult) -> String {
    let mut hash = Sha256::new();
    hash_i32(&mut hash, result.changed.0);
    hash_i32(&mut hash, result.changed.1);
    hash_i32(&mut hash, result.changed.2);
    hash_u64(&mut hash, result.light_updates.len());
    for ((x, y, z), block, sky) in &result.light_updates {
        hash_i32(&mut hash, *x);
        hash_i32(&mut hash, *y);
        hash_i32(&mut hash, *z);
        hash.update([*block, *sky]);
    }
    hash_u64(&mut hash, result.dirty_chunks.len());
    for (x, z) in &result.dirty_chunks {
        hash_i32(&mut hash, *x);
        hash_i32(&mut hash, *z);
    }
    hash_u64(&mut hash, result.unloaded_neighbors.len());
    for (x, y, z) in &result.unloaded_neighbors {
        hash_i32(&mut hash, *x);
        hash_i32(&mut hash, *y);
        hash_i32(&mut hash, *z);
    }
    finish_hash(hash)
}

fn scene_bytes(scene: &BlockScene) -> Vec<u8> {
    let bounds = scene.bounds();
    let min_x = bounds.min_chunk_x * 16;
    let min_z = bounds.min_chunk_z * 16;
    let max_x = (bounds.min_chunk_x + i32::from(bounds.width)) * 16;
    let max_z = (bounds.min_chunk_z + i32::from(bounds.depth)) * 16;
    let mut bytes = Vec::with_capacity(
        usize::from(bounds.width) * usize::from(bounds.depth) * 16 * 16 * 128 * 5,
    );
    for x in min_x..max_x {
        for z in min_z..max_z {
            for y in 0..128 {
                match scene.block(x, y, z).expect("scene coordinate is valid") {
                    Some(block) => bytes.extend_from_slice(&[
                        1,
                        block.id,
                        block.metadata,
                        block.block_light,
                        block.sky_light,
                    ]),
                    None => bytes.extend_from_slice(&[0, 0, 0, 0, 0]),
                }
            }
        }
    }
    bytes
}

fn validate_material_samples(scene: &BlockScene, name: &str, phase: &str) -> usize {
    let mut samples = 0;
    for line in MATERIAL.lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row.len() != 12 || row[0] != name || row[1] != phase {
            continue;
        }
        let (x, y, z) = (
            row[2].parse::<i32>().unwrap(),
            row[3].parse::<i32>().unwrap(),
            row[4].parse::<i32>().unwrap(),
        );
        let block = scene.block(x, y, z).unwrap().unwrap();
        assert_eq!(block.id.to_string(), row[5], "{line}");
        assert_eq!(opacity_level(block.id).to_string(), row[7], "{line}");
        assert_eq!(emission_level(block.id).to_string(), row[8], "{line}");
        assert_eq!(block.sky_light.to_string(), row[9], "{line}");
        assert_eq!(block.block_light.to_string(), row[10], "{line}");
        assert_eq!(
            combined_light(block.block_light, block.sky_light, 0)
                .unwrap()
                .to_string(),
            row[11],
            "{line}"
        );
        samples += 1;
    }
    samples
}

fn validate_update_samples(scene: &BlockScene, name: &str) -> usize {
    let mut samples = 0;
    for line in UPDATE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row.len() < 12 || row[0] != name || row[1] != "after" {
            continue;
        }
        let (x, y, z) = (
            row[2].parse::<i32>().unwrap(),
            row[3].parse::<i32>().unwrap(),
            row[4].parse::<i32>().unwrap(),
        );
        let actual = scene.block(x, y, z).unwrap();
        if row[5] == "UNLOADED" {
            assert!(actual.is_none(), "{line}");
            samples += 1;
            continue;
        }
        let block = actual.unwrap();
        assert_eq!(block.id.to_string(), row[5], "{line}");
        assert_eq!(block.metadata.to_string(), row[6], "{line}");
        assert_eq!(block.sky_light.to_string(), row[9], "{line}");
        assert_eq!(block.block_light.to_string(), row[10], "{line}");
        assert_eq!(
            combined_light(block.block_light, block.sky_light, 0)
                .unwrap()
                .to_string(),
            row[11],
            "{line}"
        );
        samples += 1;
    }
    samples
}

fn dump_full_output(scenario: &Scenario, scene: &BlockScene, result: &RelightResult) {
    let Ok(root) = std::env::var("RELIGHT_DUMP_DIR") else {
        return;
    };
    let root = std::path::Path::new(&root);
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join(format!("{}.scene.bin", scenario.name)),
        scene_bytes(scene),
    )
    .unwrap();
    let mut result_bytes = Vec::new();
    result_bytes.extend_from_slice(&result.changed.0.to_le_bytes());
    result_bytes.extend_from_slice(&result.changed.1.to_le_bytes());
    result_bytes.extend_from_slice(&result.changed.2.to_le_bytes());
    result_bytes.extend_from_slice(&(result.light_updates.len() as u64).to_le_bytes());
    for ((x, y, z), block, sky) in &result.light_updates {
        result_bytes.extend_from_slice(&x.to_le_bytes());
        result_bytes.extend_from_slice(&y.to_le_bytes());
        result_bytes.extend_from_slice(&z.to_le_bytes());
        result_bytes.extend_from_slice(&[*block, *sky]);
    }
    result_bytes.extend_from_slice(&(result.dirty_chunks.len() as u64).to_le_bytes());
    for (x, z) in &result.dirty_chunks {
        result_bytes.extend_from_slice(&x.to_le_bytes());
        result_bytes.extend_from_slice(&z.to_le_bytes());
    }
    result_bytes.extend_from_slice(&(result.unloaded_neighbors.len() as u64).to_le_bytes());
    for (x, y, z) in &result.unloaded_neighbors {
        result_bytes.extend_from_slice(&x.to_le_bytes());
        result_bytes.extend_from_slice(&y.to_le_bytes());
        result_bytes.extend_from_slice(&z.to_le_bytes());
    }
    std::fs::write(
        root.join(format!("{}.result.bin", scenario.name)),
        result_bytes,
    )
    .unwrap();
}

fn material_for(material: Material, id: u8, _metadata: u8) -> (u8, u8) {
    match material {
        Material::Registry => (opacity_level(id), emission_level(id)),
        Material::OpaqueNonAir => (if id == 0 { 0 } else { 255 }, 0),
    }
}

fn parse_xyz(value: &str) -> (i32, i32, i32) {
    let mut parts = value.split(',').map(|part| part.parse().unwrap());
    (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    )
}

fn make_scene(chunks: BTreeMap<(i32, i32), ChunkData>, width: u8, depth: u8) -> BlockScene {
    BlockScene::new(
        BlockSceneBounds::new(-1, -1, width, depth).unwrap(),
        chunks
            .into_iter()
            .map(|((x, z), data)| BlockSceneChunk { x, z, data })
            .collect(),
    )
    .unwrap()
}

fn material_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    for line in MATERIAL.lines().filter(|line| line.starts_with("case\t")) {
        let fields: Vec<_> = line.split('\t').collect();
        let name = fields[1];
        let target = parse_xyz(fields[2].strip_prefix("target=").unwrap());
        let replacement = fields[3]
            .strip_prefix("replacement=")
            .unwrap()
            .parse::<u8>()
            .unwrap();
        let mut chunks = BTreeMap::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut data = ChunkData::new();
                for x in 0..16 {
                    for z in 0..16 {
                        for y in 1..128 {
                            data.set(
                                x,
                                y,
                                z,
                                BlockData {
                                    sky_light: 15,
                                    ..BlockData::default()
                                },
                            )
                            .unwrap();
                        }
                    }
                }
                chunks.insert((cx, cz), data);
            }
        }
        for state in MATERIAL
            .lines()
            .filter(|line| line.starts_with(&format!("state\t{name}\t")))
        {
            let row: Vec<_> = state.split('\t').collect();
            let (x, y, z) = (
                row[2].parse::<i32>().unwrap(),
                row[3].parse::<u8>().unwrap(),
                row[4].parse::<i32>().unwrap(),
            );
            let (cx, lx) = split_block_coord(x);
            let (cz, lz) = split_block_coord(z);
            chunks
                .get_mut(&(cx, cz))
                .unwrap()
                .set(
                    lx,
                    y,
                    lz,
                    BlockData {
                        id: row[5].parse().unwrap(),
                        metadata: row[6].parse().unwrap(),
                        block_light: row[7].parse().unwrap(),
                        sky_light: row[8].parse().unwrap(),
                    },
                )
                .unwrap();
        }
        let (cx, lx) = split_block_coord(target.0);
        let (cz, lz) = split_block_coord(target.2);
        let data = chunks.get_mut(&(cx, cz)).unwrap();
        let prior = data.get(lx, target.1 as u8, lz).unwrap();
        data.set(
            lx,
            target.1 as u8,
            lz,
            BlockData {
                id: replacement,
                metadata: 0,
                ..prior
            },
        )
        .unwrap();
        scenarios.push(Scenario {
            name: format!("material-{name}"),
            target,
            target_id: replacement,
            material: Material::Registry,
            scene: make_scene(chunks, 3, 3),
            timed: true,
        });
    }
    scenarios
}

fn update_scenarios() -> Vec<Scenario> {
    let case_specs = [
        ("remove", 8, 64, 8, 0),
        ("place", 8, 64, 8, 1),
        ("roof", 8, 127, 8, 1),
        ("edge", 15, 64, 8, 1),
        ("edge-x31", 31, 64, 8, 1),
    ];
    let mut scenarios = Vec::new();
    for (name, edit_x, edit_y, edit_z, new_id) in case_specs {
        let mut native = HashMap::new();
        let mut active = false;
        for line in UPDATE.lines() {
            let row: Vec<_> = line.split('\t').collect();
            if row.first() == Some(&"case") {
                active = row.get(1) == Some(&name);
                continue;
            }
            if active && row.len() >= 9 && row[0] == "state" && row[1] == name {
                native.insert(
                    (
                        row[2].parse::<i32>().unwrap(),
                        row[3].parse::<i32>().unwrap(),
                        row[4].parse::<i32>().unwrap(),
                    ),
                    (
                        row[5].parse::<u8>().unwrap(),
                        row[6].parse::<u8>().unwrap(),
                        row[7].parse::<u8>().unwrap(),
                        row[8].parse::<u8>().unwrap(),
                    ),
                );
            }
            if row.first() == Some(&"final") {
                active = false;
            }
        }
        let target = (edit_x, edit_y, edit_z);
        let mut chunks = BTreeMap::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut data = ChunkData::new();
                for x in 0..16u8 {
                    for z in 0..16u8 {
                        for y in 1..128u8 {
                            let pos =
                                (cx * 16 + i32::from(x), i32::from(y), cz * 16 + i32::from(z));
                            let old = native.get(&pos).copied().unwrap_or((0, 0, 0, 15));
                            let edited = pos == target;
                            data.set(
                                x,
                                y,
                                z,
                                BlockData {
                                    id: if edited { new_id } else { old.0 },
                                    metadata: if edited { 0 } else { old.1 },
                                    block_light: old.2,
                                    sky_light: old.3,
                                },
                            )
                            .unwrap();
                        }
                    }
                }
                chunks.insert((cx, cz), data);
            }
        }
        scenarios.push(Scenario {
            name: format!("update-{name}"),
            target,
            target_id: new_id,
            material: Material::OpaqueNonAir,
            scene: make_scene(chunks, 3, 3),
            timed: false,
        });
    }
    scenarios
}

fn invoke(scenario: &Scenario, scene: &mut BlockScene) -> RelightResult {
    scene
        .relight(scenario.target, MAX_RELIGHT_CELLS, |id, metadata| {
            material_for(scenario.material, id, metadata)
        })
        .unwrap()
}

fn digest_rows(scenarios: &[Scenario]) {
    println!(
        "DIGESTS\tname\tinput_scene_sha256\toutput_scene_sha256\tresult_sha256\tupdates\tdirty\tunloaded"
    );
    let mut material_samples = 0;
    let mut update_samples = 0;
    for scenario in scenarios {
        assert_eq!(
            scenario
                .scene
                .block(scenario.target.0, scenario.target.1, scenario.target.2)
                .unwrap()
                .unwrap()
                .id,
            scenario.target_id,
            "edited target must match native replacement for {}",
            scenario.name
        );
        let input_hash = scene_hash(&scenario.scene);
        let mut output = scenario.scene.clone();
        let result = invoke(scenario, &mut output);
        let sample_count = if scenario.name.starts_with("material-") {
            let name = scenario.name.strip_prefix("material-").unwrap();
            let count = validate_material_samples(&output, name, "after");
            material_samples += count;
            count
        } else {
            let count =
                validate_update_samples(&output, scenario.name.strip_prefix("update-").unwrap());
            update_samples += count;
            count
        };
        assert!(
            sample_count > 0,
            "missing native samples for {}",
            scenario.name
        );
        dump_full_output(scenario, &output, &result);
        println!(
            "DIGESTS\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            scenario.name,
            input_hash,
            scene_hash(&output),
            result_hash(&result),
            result.light_updates.len(),
            result.dirty_chunks.len(),
            result.unloaded_neighbors.len(),
        );
    }
    assert_eq!(material_samples, 19, "all native material after samples");
    assert_eq!(
        update_samples, 63,
        "all native light-update after rows including unloaded"
    );
}

fn percentile(sorted: &[u128], pct: usize) -> u128 {
    sorted[(sorted.len() - 1) * pct / 100]
}

fn benchmark(scenarios: &[Scenario]) {
    println!(
        "TIMINGS\trepeat\tname\tsamples\tvisited\tupdates\tcallbacks\tp50_us\tp90_us\tmin_us\tmax_us"
    );
    for repeat in 1..=REPEATS {
        for scenario in scenarios.iter().filter(|scenario| scenario.timed) {
            let mut times = Vec::with_capacity(SAMPLES);
            let mut updates = 0;
            for _ in 0..SAMPLES {
                let mut scene = scenario.scene.clone();
                let start = Instant::now();
                let result = invoke(scenario, &mut scene);
                let elapsed = start.elapsed().as_nanos();
                black_box(&scene);
                times.push(elapsed);
                updates = result.light_updates.len();
            }
            times.sort_unstable();
            let mut count_scene = scenario.scene.clone();
            let calls = Cell::new(0usize);
            count_scene
                .relight(scenario.target, MAX_RELIGHT_CELLS, |id, metadata| {
                    calls.set(calls.get() + 1);
                    material_for(scenario.material, id, metadata)
                })
                .unwrap();
            println!(
                "TIMINGS\t{repeat}\t{}\t{SAMPLES}\t1152\t{updates}\t{}\t{}\t{}\t{}\t{}",
                scenario.name,
                calls.get(),
                percentile(&times, 50) / 1000,
                percentile(&times, 90) / 1000,
                times[0] / 1000,
                times[times.len() - 1] / 1000,
            );
        }
    }
}

fn main() {
    let mut scenarios = material_scenarios();
    scenarios.extend(update_scenarios());
    assert_eq!(scenarios.len(), 8);
    digest_rows(&scenarios);
    benchmark(&scenarios);
}
