//! Diagnostic full-domain comparison for the native light-material V2 corpus.
//! Each edit starts from its preceding native snapshot, isolating one mutation.

use beta_core::{
    block::{BlockScene, BlockSceneBounds, BlockSceneChunk, MAX_RELIGHT_CELLS},
    chunk::{BlockData, ChunkData},
    light::{emission_level, opacity_level},
};
use std::{
    collections::BTreeMap,
    io::{self, Read},
    process::ExitCode,
};

type Position = (i32, i32, i32);
type Snapshot = BTreeMap<Position, BlockData>;

struct Edit {
    phase: String,
    position: Position,
    id: u8,
    metadata: u8,
}

#[derive(Default)]
struct Case {
    domain_seen: bool,
    snapshots: BTreeMap<String, Snapshot>,
    edits: Vec<Edit>,
}

fn position(fields: &[&str]) -> Result<Position, Box<dyn std::error::Error>> {
    let point = (fields[0].parse()?, fields[1].parse()?, fields[2].parse()?);
    if !(-16..32).contains(&point.0)
        || !(0..128).contains(&point.1)
        || !(-16..32).contains(&point.2)
    {
        return Err("position outside the native nine-chunk domain".into());
    }
    Ok(point)
}

fn parse(text: &str) -> Result<BTreeMap<String, Case>, Box<dyn std::error::Error>> {
    let mut cases = BTreeMap::<String, Case>::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let row: Vec<_> = line.split('\t').collect();
        match row.first().copied() {
            Some("case") if row.len() >= 2 => {
                if cases.insert(row[1].into(), Case::default()).is_some() {
                    return Err(format!("duplicate case at line {}", number + 1).into());
                }
            }
            Some("state") if row.len() == 10 => {
                let case = cases.get_mut(row[1]).ok_or("state precedes its case")?;
                let point = position(&row[3..6])?;
                let value = BlockData {
                    id: row[6].parse()?,
                    metadata: row[7].parse()?,
                    block_light: row[8].parse()?,
                    sky_light: row[9].parse()?,
                };
                if value.metadata > 15 || value.block_light > 15 || value.sky_light > 15 {
                    return Err("native nibble outside 0..15".into());
                }
                if case
                    .snapshots
                    .get_mut(row[2])
                    .ok_or("state precedes snapshot header")?
                    .insert(point, value)
                    .is_some()
                {
                    return Err("duplicate state coordinate in native snapshot".into());
                }
            }
            Some("edit") if row.len() == 9 => {
                let case = cases.get_mut(row[1]).ok_or("edit precedes its case")?;
                if !matches!(row[3], "place" | "remove") {
                    return Err("unknown edit operation".into());
                }
                let edit = Edit {
                    phase: row[2].into(),
                    position: position(&row[4..7])?,
                    id: row[7].parse()?,
                    metadata: row[8].parse()?,
                };
                if edit.metadata > 15
                    || edit.phase == "before"
                    || case.edits.iter().any(|old| old.phase == edit.phase)
                {
                    return Err("invalid or duplicate edit phase".into());
                }
                case.edits.push(edit);
            }
            Some("domain") if row.len() >= 3 => {
                let case = cases.get_mut(row[1]).ok_or("domain precedes case")?;
                let fields: BTreeMap<_, _> = row[2..]
                    .iter()
                    .map(|field| field.split_once('=').ok_or("invalid domain field"))
                    .collect::<Result<_, _>>()?;
                for (key, value) in [
                    ("chunks_x", "-1,0,1"),
                    ("chunks_z", "-1,0,1"),
                    ("y", "0..127"),
                    ("default_block", "0"),
                    ("default_meta", "0"),
                    ("default_block_light", "0"),
                ] {
                    if fields.get(key) != Some(&value) {
                        return Err(format!("unsupported domain: {key}").into());
                    }
                }
                if case.domain_seen
                    || fields
                        .get("default_sky")
                        .is_none_or(|value| *value != "0@y0;15@y1..127(native-lm.c-all-air)")
                {
                    return Err("duplicate domain or unsupported default skylight".into());
                }
                case.domain_seen = true;
            }
            // A snapshot declares even an all-default (zero state-row) phase.
            Some("snapshot") if row.len() == 4 => {
                let case = cases.get_mut(row[1]).ok_or("snapshot precedes case")?;
                let digest = row[3]
                    .strip_prefix("sha256=")
                    .ok_or("missing snapshot digest")?;
                if !case.domain_seen
                    || digest.len() != 64
                    || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || case
                        .snapshots
                        .insert(row[2].into(), Snapshot::new())
                        .is_some()
                {
                    return Err("invalid or duplicate snapshot header".into());
                }
            }
            // Additional native evidence is checked by the independent audit.
            Some("seed" | "height" | "sample" | "delta" | "hdelta") => {}
            _ => return Err(format!("unknown row at line {}", number + 1).into()),
        }
    }
    if cases.is_empty() || cases.values().any(|case| case.edits.is_empty()) {
        return Err("native corpus must contain cases with edits".into());
    }
    Ok(cases)
}

fn expected(snapshot: &Snapshot, point: Position) -> BlockData {
    snapshot.get(&point).copied().unwrap_or(BlockData {
        sky_light: if point.1 == 0 { 0 } else { 15 },
        ..BlockData::default()
    })
}

fn edited_scene(
    snapshot: &Snapshot,
    edit: &Edit,
) -> Result<BlockScene, Box<dyn std::error::Error>> {
    let mut chunks = Vec::new();
    for cx in -1..=1 {
        for cz in -1..=1 {
            let mut data = ChunkData::new();
            for lx in 0..16 {
                for lz in 0..16 {
                    for y in 0..128 {
                        let point = (
                            cx * 16 + i32::from(lx),
                            i32::from(y),
                            cz * 16 + i32::from(lz),
                        );
                        let mut value = expected(snapshot, point);
                        if point == edit.position {
                            value.id = edit.id;
                            value.metadata = edit.metadata;
                        }
                        data.set(lx, y, lz, value)?;
                    }
                }
            }
            chunks.push(BlockSceneChunk { x: cx, z: cz, data });
        }
    }
    Ok(BlockScene::new(
        BlockSceneBounds::new(-1, -1, 3, 3)?,
        chunks,
    )?)
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin()
        .take(32 * 1024 * 1024 + 1)
        .read_to_string(&mut input)?;
    if input.len() > 32 * 1024 * 1024 {
        return Err("native corpus exceeds 32 MiB".into());
    }
    let cases = parse(&input)?;
    let mut passed = true;
    println!("case\tphase\tcells\tlight_mismatches\tid_mismatches\tmetadata_mismatches");
    for (name, case) in cases {
        let mut previous = "before";
        for edit in &case.edits {
            let before = case
                .snapshots
                .get(previous)
                .ok_or("missing preceding snapshot")?;
            let after = case
                .snapshots
                .get(&edit.phase)
                .ok_or("missing resulting snapshot")?;
            let mut scene = edited_scene(before, edit)?;
            scene.relight(edit.position, MAX_RELIGHT_CELLS, |id, _| {
                (opacity_level(id), emission_level(id))
            })?;
            let (mut lights, mut ids, mut metadata) = (0, 0, 0);
            let mut printed = 0;
            for x in -16..32 {
                for z in -16..32 {
                    for y in 0..128 {
                        let native = expected(after, (x, y, z));
                        let rust = scene.block(x, y, z)?.ok_or("missing loaded cell")?;
                        let light_diff = (native.block_light, native.sky_light)
                            != (rust.block_light, rust.sky_light);
                        lights += usize::from(light_diff);
                        ids += usize::from(native.id != rust.id);
                        metadata += usize::from(native.metadata != rust.metadata);
                        if light_diff && printed < 8 {
                            eprintln!(
                                "{name}/{} ({x},{y},{z}) native block/sky={}/{} rust={}/{}",
                                edit.phase,
                                native.block_light,
                                native.sky_light,
                                rust.block_light,
                                rust.sky_light
                            );
                            printed += 1;
                        }
                    }
                }
            }
            println!(
                "{name}\t{}\t294912\t{lights}\t{ids}\t{metadata}",
                edit.phase
            );
            passed &= lights == 0 && ids == 0 && metadata == 0;
            previous = &edit.phase;
        }
    }
    eprintln!(
        "Scope: independently initialized native prestate per edit; no height-map, GPU or cumulative Rust-state parity."
    );
    Ok(passed)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("light_snapshot_probe: {error}");
            ExitCode::from(2)
        }
    }
}
