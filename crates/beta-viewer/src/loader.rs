//! Read-only loader and strict protocol implementation for fixed scenes and
//! bounded spawn-centered previews.
//!
//! The probe trusts neither paths nor archive contents. It accepts only the marked,
//! hash-pinned five-file copy produced by the repository snapshot tool.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use beta_core::{
    chunk::BlockData,
    nbt::Limits,
    region::{RegionFile, RegionLimits, local_coord, region_coord},
    world::{WorldLimits, WorldMetadata, load_chunk_nbt, load_level_gzip},
};
use beta_view::{BlockMapping, BlockStateKey, MeshLimits, Scene, SceneBounds, SceneChunk};
use sha2::{Digest, Sha256};

pub const TERRAIN_SHA256: &str = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf";
const WORLD_MARKER: &str = "WORLD-LOAD-001 copied inputs\n";
const REGION_MARKER: &str = "REGION-001 copied inputs\n";
const FILES: [(&str, usize, &str); 5] = [
    (
        "level.dat",
        411,
        "2711d16242afd13909575955d4a34502da01f6f666ee79341fe1c1ba96c8a960",
    ),
    (
        "r.-1.-1.mcr",
        761856,
        "b41220e2e389bf303984fa7211192debc7e1db3e49b89c168101a3ed748c435f",
    ),
    (
        "r.-1.0.mcr",
        843776,
        "e189d75077763fd00813f5929a14a9de85704b55ef8042afb8fd2fb2f5801004",
    ),
    (
        "r.0.-1.mcr",
        1019904,
        "59ce84edc6afcec1f962e0d88f519e359137c7245b688a58ab340414b12eb0ba",
    ),
    (
        "r.0.0.mcr",
        782336,
        "67f39f4c77538f178fa7acb155030908f84c079b70f27d44d85f4a20fe965b7a",
    ),
];

/// A local-only source pin supplied by the viewer's ignored post-play manifest.
/// Strict parity probes continue using the historical constants above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSourcePin {
    pub name: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug)]
pub enum ProbeError {
    InvalidArguments,
    InvalidInput(String),
    Io(std::io::Error),
    Core(String),
    Image(String),
    UnsupportedCase(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments => {
                write!(f, "usage: view_probe INPUT_COPY_DIR TERRAIN_PNG SCENE_TSV")
            }
            Self::InvalidInput(s) => write!(f, "invalid input: {s}"),
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Core(s) => write!(f, "world load: {s}"),
            Self::Image(s) => write!(f, "terrain image: {s}"),
            Self::UnsupportedCase(s) => {
                write!(f, "scene contains an unregistered native case: {s}")
            }
        }
    }
}
impl Error for ProbeError {}
impl From<std::io::Error> for ProbeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct State {
    id: u8,
    meta: u8,
    above: i16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Neighbor {
    id: u8,
    meta: u8,
    face: u8,
    other: u8,
    other_meta: u8,
}

#[derive(Debug)]
struct ParsedScene {
    chunks: Vec<(i32, i32)>,
    min_x: i32,
    min_z: i32,
}

type LoadedSceneInputs = (
    Scene,
    WorldMetadata,
    Vec<SceneChunk>,
    ParsedScene,
    Vec<BlockMapping>,
);

fn read_regular(path: &Path, label: &str) -> Result<Vec<u8>, ProbeError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProbeError::InvalidInput(format!(
            "{label} is not a regular non-symlink file"
        )));
    }
    Ok(fs::read(path)?)
}

fn validate_copy(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, ProbeError> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(ProbeError::InvalidInput(
            "input copy must be a real directory".into(),
        ));
    }
    let marker_world = read_regular(&dir.join(".world-input-copy"), "world marker")?;
    let marker_region = read_regular(&dir.join(".region-input-copy"), "region marker")?;
    if marker_world != WORLD_MARKER.as_bytes() || marker_region != REGION_MARKER.as_bytes() {
        return Err(ProbeError::InvalidInput(
            "input marker content mismatch".into(),
        ));
    }
    let mut allowed: BTreeSet<String> = [".world-input-copy", ".region-input-copy"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut files = BTreeMap::new();
    for (name, size, hash) in FILES {
        allowed.insert(name.into());
        let path = dir.join(name);
        let bytes = read_regular(&path, name)?;
        if bytes.len() != size || sha256_hex(&bytes) != hash {
            return Err(ProbeError::InvalidInput(format!(
                "copied {name} size/hash mismatch"
            )));
        }
        files.insert(name.to_owned(), bytes);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| ProbeError::InvalidInput("non-UTF8 path in input copy".into()))?;
        if !allowed.contains(&name) {
            return Err(ProbeError::InvalidInput(format!(
                "unexpected input-copy entry {name}"
            )));
        }
        let ty = entry.file_type()?;
        if ty.is_symlink()
            || (!ty.is_file() && name != ".world-input-copy" && name != ".region-input-copy")
        {
            return Err(ProbeError::InvalidInput(format!(
                "unsafe input-copy entry {name}"
            )));
        }
    }
    Ok(files)
}

fn validate_preview_copy(
    dir: &Path,
    pins: &[PreviewSourcePin],
) -> Result<BTreeMap<String, Vec<u8>>, ProbeError> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(ProbeError::InvalidInput(
            "preview input copy must be a real directory".into(),
        ));
    }
    let expected: BTreeMap<_, _> = FILES.iter().map(|(name, _, _)| (*name, ())).collect();
    if pins.len() != FILES.len()
        || pins.iter().any(|pin| {
            !expected.contains_key(pin.name.as_str())
                || pin.size == 0
                || pin.size
                    > if pin.name == "level.dat" {
                        16 * 1024 * 1024
                    } else {
                        64 * 1024 * 1024
                    }
                || pin.sha256.len() != 64
                || !pin
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    {
        return Err(ProbeError::InvalidInput(
            "preview source pin set is invalid".into(),
        ));
    }
    let by_name: BTreeMap<_, _> = pins.iter().map(|pin| (pin.name.as_str(), pin)).collect();
    if by_name.len() != FILES.len() {
        return Err(ProbeError::InvalidInput(
            "preview source pins contain duplicates".into(),
        ));
    }
    let marker_world = read_regular(&dir.join(".world-input-copy"), "world marker")?;
    let marker_region = read_regular(&dir.join(".region-input-copy"), "region marker")?;
    let marker_sources = read_regular(
        &dir.join(".preview-source-hashes.tsv"),
        "preview source pins",
    )?;
    if marker_world != WORLD_MARKER.as_bytes() || marker_region != REGION_MARKER.as_bytes() {
        return Err(ProbeError::InvalidInput(
            "preview copy marker content mismatch".into(),
        ));
    }
    let expected_manifest = pins
        .iter()
        .map(|pin| format!("{}\t{}\t{}\n", pin.name, pin.size, pin.sha256))
        .collect::<String>();
    if marker_sources != expected_manifest.as_bytes() {
        return Err(ProbeError::InvalidInput(
            "preview source manifest differs from requested pins".into(),
        ));
    }
    let mut allowed: BTreeSet<String> = [
        ".world-input-copy".to_owned(),
        ".region-input-copy".to_owned(),
        ".preview-source-hashes.tsv".to_owned(),
    ]
    .into_iter()
    .collect();
    let mut files = BTreeMap::new();
    for name in expected.keys() {
        allowed.insert((*name).to_owned());
        let pin = by_name
            .get(name)
            .ok_or_else(|| ProbeError::InvalidInput(format!("missing preview pin for {name}")))?;
        let data = read_regular(&dir.join(name), name)?;
        if data.len() != pin.size || sha256_hex(&data) != pin.sha256 {
            return Err(ProbeError::InvalidInput(format!(
                "preview copy {name} size/hash mismatch"
            )));
        }
        files.insert((*name).to_owned(), data);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| ProbeError::InvalidInput("non-UTF8 preview path".into()))?;
        let ty = entry.file_type()?;
        if !allowed.contains(&name) || ty.is_symlink() || !ty.is_file() {
            return Err(ProbeError::InvalidInput(format!(
                "unsafe or unexpected preview input entry {name}"
            )));
        }
    }
    Ok(files)
}

/// Validate a disposable snapshot produced by the local world chooser. Unlike
/// the historical preview protocol above, this accepts a dynamic set of region
/// files; strict parity tools remain pinned to `FILES`.
fn validate_selected_world_copy(
    dir: &Path,
    pins: &[PreviewSourcePin],
) -> Result<BTreeMap<String, Vec<u8>>, ProbeError> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(ProbeError::InvalidInput(
            "selected-world snapshot must be a real directory".into(),
        ));
    }
    let mut by_name = BTreeMap::new();
    for pin in pins {
        let valid_name = pin.name == "level.dat" || valid_region_filename(&pin.name);
        let valid_hash = pin.sha256.len() == 64
            && pin
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if !valid_name
            || pin.size == 0
            || pin.size
                > if pin.name == "level.dat" {
                    16 * 1024 * 1024
                } else {
                    64 * 1024 * 1024
                }
            || !valid_hash
            || by_name.insert(pin.name.as_str(), pin).is_some()
        {
            return Err(ProbeError::InvalidInput(
                "selected-world source pin set is invalid".into(),
            ));
        }
    }
    if !by_name.contains_key("level.dat") || by_name.len() < 2 {
        return Err(ProbeError::InvalidInput(
            "selected-world snapshot requires level.dat and region files".into(),
        ));
    }
    let marker_world = read_regular(&dir.join(".world-input-copy"), "world marker")?;
    let marker_region = read_regular(&dir.join(".region-input-copy"), "region marker")?;
    let marker_sources = read_regular(
        &dir.join(".preview-source-hashes.tsv"),
        "preview source pins",
    )?;
    if marker_world != WORLD_MARKER.as_bytes() || marker_region != REGION_MARKER.as_bytes() {
        return Err(ProbeError::InvalidInput(
            "selected-world marker content mismatch".into(),
        ));
    }
    let manifest = pins
        .iter()
        .map(|pin| format!("{}\t{}\t{}\n", pin.name, pin.size, pin.sha256))
        .collect::<String>();
    if marker_sources != manifest.as_bytes() {
        return Err(ProbeError::InvalidInput(
            "selected-world source manifest differs from requested pins".into(),
        ));
    }
    let mut allowed: BTreeSet<String> = [
        ".world-input-copy".to_owned(),
        ".region-input-copy".to_owned(),
        ".preview-source-hashes.tsv".to_owned(),
    ]
    .into_iter()
    .collect();
    let mut files = BTreeMap::new();
    for (name, pin) in by_name {
        allowed.insert(name.to_owned());
        let data = read_regular(&dir.join(name), name)?;
        if data.len() != pin.size || sha256_hex(&data) != pin.sha256 {
            return Err(ProbeError::InvalidInput(format!(
                "selected-world snapshot {name} size/hash mismatch"
            )));
        }
        files.insert(name.to_owned(), data);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| ProbeError::InvalidInput("non-UTF8 selected-world path".into()))?;
        let ty = entry.file_type()?;
        if !allowed.contains(&name) || ty.is_symlink() || !ty.is_file() {
            return Err(ProbeError::InvalidInput(format!(
                "unsafe or unexpected selected-world entry {name}"
            )));
        }
    }
    Ok(files)
}

fn valid_region_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".mcr") else {
        return false;
    };
    let mut parts = stem.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()),
        (Some("r"), Some(x), Some(z), None, None)
            if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok()
    )
}

fn validate_terrain(path: &Path) -> Result<(), ProbeError> {
    let bytes = read_regular(path, "terrain atlas")?;
    if sha256_hex(&bytes) != TERRAIN_SHA256 {
        return Err(ProbeError::InvalidInput(
            "terrain.png SHA-256 mismatch".into(),
        ));
    }
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| ProbeError::Image(e.to_string()))?;
    if image.width() != 256 || image.height() != 256 {
        return Err(ProbeError::Image(
            "expected 256x256 original terrain atlas".into(),
        ));
    }
    Ok(())
}

fn parse_scene(path: &Path, spawn: [i32; 3]) -> Result<ParsedScene, ProbeError> {
    let raw = read_regular(path, "scene TSV")?;
    if raw.len() > 4096 {
        return Err(ProbeError::InvalidInput("scene TSV exceeds 4 KiB".into()));
    }
    let text = std::str::from_utf8(&raw)
        .map_err(|_| ProbeError::InvalidInput("scene TSV is not UTF-8".into()))?;
    let mut chunks = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let x = fields
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| {
                ProbeError::InvalidInput(format!("bad scene x at line {}", line_no + 1))
            })?;
        let z = fields
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| {
                ProbeError::InvalidInput(format!("bad scene z at line {}", line_no + 1))
            })?;
        if fields.next().is_some() {
            return Err(ProbeError::InvalidInput(format!(
                "extra scene field at line {}",
                line_no + 1
            )));
        }
        chunks.push((x, z));
    }
    let unique: BTreeSet<_> = chunks.iter().copied().collect();
    if unique.len() != chunks.len() {
        return Err(ProbeError::InvalidInput(
            "scene contains duplicate chunk coordinates".into(),
        ));
    }
    chunks.sort_unstable_by_key(|&(x, z)| (z, x));
    if chunks.len() != 25 {
        return Err(ProbeError::InvalidInput(format!(
            "scene must contain exactly 25 distinct chunks, got {}",
            chunks.len()
        )));
    }
    let min_x = spawn[0] >> 4;
    let min_z = spawn[2] >> 4;
    let expected: BTreeSet<_> = (-2..=2)
        .flat_map(|dz| (-2..=2).map(move |dx| (min_x + dx, min_z + dz)))
        .collect();
    let actual: BTreeSet<_> = chunks.iter().copied().collect();
    if actual != expected {
        return Err(ProbeError::InvalidInput(
            "scene is not the fixed spawn-centered 5x5 chunk rectangle".into(),
        ));
    }
    Ok(ParsedScene {
        chunks,
        min_x: min_x - 2,
        min_z: min_z - 2,
    })
}

fn parse_spawn_window(spawn: [i32; 3]) -> Result<ParsedScene, ProbeError> {
    let cx = spawn[0].div_euclid(16);
    let cz = spawn[2].div_euclid(16);
    let min_x = cx.checked_sub(2).ok_or_else(|| {
        ProbeError::InvalidInput("spawn chunk window exceeds coordinate range".into())
    })?;
    let min_z = cz.checked_sub(2).ok_or_else(|| {
        ProbeError::InvalidInput("spawn chunk window exceeds coordinate range".into())
    })?;
    min_x.checked_add(4).ok_or_else(|| {
        ProbeError::InvalidInput("spawn chunk window exceeds coordinate range".into())
    })?;
    min_z.checked_add(4).ok_or_else(|| {
        ProbeError::InvalidInput("spawn chunk window exceeds coordinate range".into())
    })?;
    let chunks = (-2..=2)
        .flat_map(|dz| (-2..=2).map(move |dx| (cx + dx, cz + dz)))
        .collect();
    Ok(ParsedScene {
        chunks,
        min_x,
        min_z,
    })
}

fn load_scene(
    files: &BTreeMap<String, Vec<u8>>,
    coords: &ParsedScene,
    level: WorldMetadata,
) -> Result<(Scene, WorldMetadata, Vec<SceneChunk>, Vec<BlockMapping>), ProbeError> {
    let mut regions: BTreeMap<(i32, i32), RegionFile<'_>> = BTreeMap::new();
    // Keep borrowed bytes in `files`; region values only borrow that stable map.
    for (rx, rz) in required_region_coords(coords) {
        let name = format!("r.{rx}.{rz}.mcr");
        let bytes = files
            .get(&name)
            .ok_or_else(|| ProbeError::InvalidInput(format!("missing fixed region {name}")))?;
        let region = RegionFile::open(bytes, RegionLimits::default())
            .map_err(|e| ProbeError::Core(e.to_string()))?;
        regions.insert((rx, rz), region);
    }
    let mut chunks = Vec::with_capacity(25);
    for &(cx, cz) in &coords.chunks {
        let rcoord = (region_coord(cx), region_coord(cz));
        let region = regions.get(&rcoord).ok_or_else(|| {
            ProbeError::InvalidInput("scene references an unpinned region".into())
        })?;
        let record = region
            .read_local(local_coord(cx), local_coord(cz))
            .map_err(|e| ProbeError::Core(e.to_string()))?
            .ok_or_else(|| {
                ProbeError::InvalidInput(format!("scene chunk ({cx},{cz}) is absent"))
            })?;
        let loaded = load_chunk_nbt(record.nbt, (cx, cz), Limits::default())
            .map_err(|e| ProbeError::Core(e.to_string()))?;
        chunks.push(SceneChunk {
            x: cx,
            z: cz,
            data: loaded.data,
        });
    }
    // Registry is native-confirmed and deliberately limited to observed full-cube and
    // known unsupported states in this fixed scene. Other IDs are omitted by scope.
    let mappings = mapping_registry();
    let bounds = SceneBounds::new(coords.min_x, coords.min_z, 5, 5)
        .map_err(|e| ProbeError::Core(e.to_string()))?;
    let analysis_chunks = chunks.clone();
    let scene = Scene::new(bounds, chunks, mappings.clone())
        .map_err(|e| ProbeError::Core(e.to_string()))?;
    Ok((scene, level, analysis_chunks, mappings))
}

fn required_region_coords(coords: &ParsedScene) -> BTreeSet<(i32, i32)> {
    coords
        .chunks
        .iter()
        .map(|&(x, z)| (region_coord(x), region_coord(z)))
        .collect()
}

fn load_scene_from_selected_files(
    files: BTreeMap<String, Vec<u8>>,
    terrain: &Path,
) -> Result<LoadedSceneInputs, ProbeError> {
    validate_terrain(terrain)?;
    let metadata = load_level_gzip(&files["level.dat"], WorldLimits::default())
        .map_err(|e| ProbeError::Core(e.to_string()))?;
    let parsed = parse_spawn_window(metadata.spawn)?;
    let (scene, metadata, scene_data, mappings) = load_scene(&files, &parsed, metadata)?;
    Ok((scene, metadata, scene_data, parsed, mappings))
}

fn probe(input: &Path, terrain: &Path, scene_file: &Path) -> Result<String, ProbeError> {
    let files = validate_copy(input)?;
    probe_with_files(files, terrain, scene_file)
}

fn probe_with_files(
    files: BTreeMap<String, Vec<u8>>,
    terrain: &Path,
    scene_file: &Path,
) -> Result<String, ProbeError> {
    let (scene, metadata, scene_data, parsed, _) =
        load_scene_from_files(files, terrain, scene_file)?;
    let (states, neighbors) = observed_rows(&scene_data, parsed.min_x, parsed.min_z)?;
    // CPU mesh construction validates that the same loaded scene can feed the renderer.
    let mesh = scene
        .mesh(MeshLimits::default())
        .map_err(|e| ProbeError::Core(e.to_string()))?;
    let mut output = format!(
        "#VIEW_NATIVE\t1\t{}\t{}\t{}\t{}\t{}\n#states\n",
        metadata.spawn[0], metadata.spawn[1], metadata.spawn[2], parsed.min_x, parsed.min_z
    );
    for state in states {
        let mapping = mapping_for_state(state)
            .ok_or_else(|| ProbeError::UnsupportedCase(format!("state {state:?}")))?;
        output.push_str(&format!(
            "state\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            state.id,
            state.meta,
            state.above,
            mapping.render_type,
            u8::from(mapping.opaque),
            u8::from(mapping.full_cube),
            mapping
                .alpha_opaque_faces
                .iter()
                .map(|v| if *v { '1' } else { '0' })
                .collect::<String>(),
            mapping
                .tiles
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(","),
            u8::from(mapping.supported)
        ));
    }
    output.push_str("#neighbors\n");
    for row in neighbors {
        output.push_str(&format!(
            "neighbor\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.id,
            row.meta,
            row.face,
            row.other,
            row.other_meta,
            visibility(row)?
        ));
    }
    // Keep an explicit no-op use so clippy does not encourage bypassing Scene::mesh.
    let _mesh_counts = (mesh.visible_faces, mesh.unsupported_blocks);
    Ok(output)
}

fn load_scene_inputs(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
) -> Result<LoadedSceneInputs, ProbeError> {
    let files = validate_copy(input)?;
    load_scene_from_files(files, terrain, scene_file)
}

fn load_scene_from_files(
    files: BTreeMap<String, Vec<u8>>,
    terrain: &Path,
    scene_file: &Path,
) -> Result<LoadedSceneInputs, ProbeError> {
    validate_terrain(terrain)?;
    let metadata = load_level_gzip(&files["level.dat"], WorldLimits::default())
        .map_err(|e| ProbeError::Core(e.to_string()))?;
    let parsed = parse_scene(scene_file, metadata.spawn)?;
    let (scene, metadata, scene_data, mappings) = load_scene(&files, &parsed, metadata)?;
    Ok((scene, metadata, scene_data, parsed, mappings))
}

/// Load a viewer-only local snapshot pinned by an ignored machine-local manifest.
/// This path is deliberately separate from `run_args`, whose historical source
/// hashes remain fixed for reproducible native parity tests.
pub fn load_scene_for_preview_with_edit_data(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
    pins: &[PreviewSourcePin],
) -> Result<(Scene, WorldMetadata, Vec<SceneChunk>, Vec<BlockMapping>), ProbeError> {
    let files = validate_preview_copy(input, pins)?;
    let (scene, metadata, chunks, _, mappings) = load_scene_from_files(files, terrain, scene_file)?;
    Ok((scene, metadata, chunks, mappings))
}

/// Load a hash-pinned selected-world snapshot without consulting the historical
/// fixed-scene TSV. The returned scene always contains the spawn-centered 5x5.
pub fn load_scene_for_selected_world_with_edit_data(
    input: &Path,
    terrain: &Path,
    pins: &[PreviewSourcePin],
) -> Result<(Scene, WorldMetadata, Vec<SceneChunk>, Vec<BlockMapping>), ProbeError> {
    let files = validate_selected_world_copy(input, pins)?;
    let (scene, metadata, chunks, _, mappings) = load_scene_from_selected_files(files, terrain)?;
    Ok((scene, metadata, chunks, mappings))
}

/// Load and validate the fixed scene for a graphical viewer.
pub fn load_scene_for_viewer(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
) -> Result<(Scene, [i32; 3]), ProbeError> {
    let (scene, metadata, _, _, _) = load_scene_inputs(input, terrain, scene_file)?;
    Ok((scene, metadata.spawn))
}

/// Load the fixed viewer scene and return the saved level seed for native biome tint.
pub fn load_scene_for_viewer_with_seed(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
) -> Result<(Scene, [i32; 3], i64), ProbeError> {
    let (scene, metadata, _, _, _) = load_scene_inputs(input, terrain, scene_file)?;
    Ok((scene, metadata.spawn, metadata.seed))
}

/// Load the fixed viewer scene and return complete saved metadata for static lighting.
pub fn load_scene_for_viewer_with_metadata(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
) -> Result<(Scene, WorldMetadata), ProbeError> {
    let (scene, metadata, _, _, _) = load_scene_inputs(input, terrain, scene_file)?;
    light_dimension(metadata.dimension)?;
    Ok((scene, metadata))
}

/// Loads a read-only source snapshot and returns clones of the bounded chunks
/// and native mappings for a caller-owned disposable interaction sandbox.
pub fn load_scene_for_viewer_with_edit_data(
    input: &Path,
    terrain: &Path,
    scene_file: &Path,
) -> Result<(Scene, WorldMetadata, Vec<SceneChunk>, Vec<BlockMapping>), ProbeError> {
    let (scene, metadata, chunks, _, mappings) = load_scene_inputs(input, terrain, scene_file)?;
    Ok((scene, metadata, chunks, mappings))
}

fn light_dimension(dimension: i32) -> Result<beta_core::light::LightDimension, ProbeError> {
    match dimension {
        0 => Ok(beta_core::light::LightDimension::Overworld),
        -1 => Ok(beta_core::light::LightDimension::Nether),
        other => Err(ProbeError::InvalidInput(format!(
            "viewer lighting does not support saved dimension {other}; supported dimensions are 0 and -1"
        ))),
    }
}

fn observed_rows(
    chunks: &[SceneChunk],
    min_x: i32,
    min_z: i32,
) -> Result<(BTreeSet<State>, BTreeSet<Neighbor>), ProbeError> {
    let map: BTreeMap<_, _> = chunks.iter().map(|c| ((c.x, c.z), &c.data)).collect();
    let block = |x: i32, y: i32, z: i32| -> Result<BlockData, ProbeError> {
        if !(0..128).contains(&y) {
            return Ok(BlockData::default());
        }
        let (cx, lx) = (x >> 4, (x & 15) as u8);
        let (cz, lz) = (z >> 4, (z & 15) as u8);
        match map.get(&(cx, cz)) {
            Some(data) => data
                .get(lx, y as u8, lz)
                .map_err(|e| ProbeError::Core(e.to_string())),
            None => Ok(BlockData::default()),
        }
    };
    let mut states = BTreeSet::new();
    let mut neighbors = BTreeSet::new();
    let deltas = [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ];
    for cz in min_z..min_z + 5 {
        for cx in min_x..min_x + 5 {
            let data = map[&(cx, cz)];
            for x in 0..16_u8 {
                for z in 0..16_u8 {
                    for y in 0..128_u8 {
                        let b = data
                            .get(x, y, z)
                            .map_err(|e| ProbeError::Core(e.to_string()))?;
                        if b.id == 0 {
                            continue;
                        }
                        let wx = cx * 16 + i32::from(x);
                        let wy = i32::from(y);
                        let wz = cz * 16 + i32::from(z);
                        let above = block(wx, wy + 1, wz)?.id;
                        let key = if b.id == 2 { i16::from(above) } else { -1 };
                        states.insert(State {
                            id: b.id,
                            meta: b.metadata,
                            above: key,
                        });
                        for (face, (dx, dy, dz)) in deltas.into_iter().enumerate() {
                            let n = block(wx + dx, wy + dy, wz + dz)?;
                            neighbors.insert(Neighbor {
                                id: b.id,
                                meta: b.metadata,
                                face: face as u8,
                                other: n.id,
                                other_meta: n.metadata,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok((states, neighbors))
}

fn visibility(row: Neighbor) -> Result<u8, ProbeError> {
    native_registry()
        .1
        .get(&row)
        .copied()
        .ok_or_else(|| ProbeError::UnsupportedCase(format!("neighbor {row:?}")))
}
fn mapping_for_state(state: State) -> Option<BlockMapping> {
    native_registry().0.get(&state).cloned()
}
fn mapping_registry() -> Vec<BlockMapping> {
    let mut mappings: Vec<_> = native_registry().0.values().cloned().collect();
    // Native uu instances with full [0,1] bounds and opaque terrain tiles.
    // Keep these viewer-only additions out of the strict observed-scene registry.
    for (id, tile) in [(4, 16), (5, 4), (22, 144), (45, 7), (48, 36)] {
        mappings.push(BlockMapping {
            key: BlockStateKey {
                id,
                metadata: 0,
                above_id: None,
            },
            render_type: 0,
            opaque: true,
            full_cube: true,
            alpha_opaque_faces: [true; 6],
            tiles: [tile; 6],
            supported: true,
        });
    }
    mappings
}

fn native_registry() -> &'static (BTreeMap<State, BlockMapping>, BTreeMap<Neighbor, u8>) {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<(BTreeMap<State, BlockMapping>, BTreeMap<Neighbor, u8>)> =
        OnceLock::new();
    REGISTRY.get_or_init(|| {
        // Reviewed rows from direct calls to the pinned original client's uu methods.
        // This is registry data, not a loaded expected-output fixture. Runtime results
        // are selected from actual world blocks and their six real neighbors.
        let mut states = BTreeMap::new();
        let mut neighbors = BTreeMap::new();
        for line in NATIVE_REGISTRY.lines() {
            let f: Vec<_> = line.split('\t').collect();
            if f.first() == Some(&"state") && f.len() == 10 {
                let id = f[1].parse::<u8>().expect("registry id");
                let meta = f[2].parse::<u8>().expect("registry meta");
                let above = f[3].parse::<i16>().expect("registry context");
                let render_type = f[4].parse::<u8>().expect("registry render type");
                let opaque = f[5] == "1";
                let full_cube = f[6] == "1";
                let mut alpha = [false; 6];
                for (i, ch) in f[7].bytes().enumerate() {
                    alpha[i] = ch == b'1';
                }
                let mut tiles = [0_u16; 6];
                for (i, tile) in f[8].split(',').enumerate() {
                    tiles[i] = tile.parse().expect("registry tile");
                }
                let supported = f[9] == "1";
                let key = State { id, meta, above };
                states.insert(
                    key,
                    BlockMapping {
                        key: BlockStateKey {
                            id,
                            metadata: meta,
                            above_id: (above >= 0).then_some(above as u8),
                        },
                        render_type,
                        opaque,
                        full_cube,
                        alpha_opaque_faces: alpha,
                        tiles,
                        supported,
                    },
                );
            } else if f.first() == Some(&"neighbor") && f.len() == 7 {
                let key = Neighbor {
                    id: f[1].parse().expect("source id"),
                    meta: f[2].parse().expect("source meta"),
                    face: f[3].parse().expect("face"),
                    other: f[4].parse().expect("neighbor id"),
                    other_meta: f[5].parse().expect("neighbor meta"),
                };
                neighbors.insert(key, f[6].parse().expect("visibility"));
            }
        }
        (states, neighbors)
    })
}

pub fn run_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<String, ProbeError> {
    let args: Vec<_> = args.into_iter().collect();
    if args.len() != 3 {
        return Err(ProbeError::InvalidArguments);
    }
    let input = PathBuf::from(&args[0]);
    let terrain = PathBuf::from(&args[1]);
    let scene = PathBuf::from(&args[2]);
    probe(&input, &terrain, &scene)
}

/// Compare a machine-local pinned preview copy against the native mapping registry.
/// This diagnostic path does not alter the historical strict `view_probe` inputs.
pub fn run_preview_args(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<String, ProbeError> {
    let args: Vec<_> = args.into_iter().collect();
    if args.len() != 3 {
        return Err(ProbeError::InvalidArguments);
    }
    let input = PathBuf::from(&args[0]);
    let terrain = PathBuf::from(&args[1]);
    let scene = PathBuf::from(&args[2]);
    let manifest = read_regular(&input.join(".preview-source-hashes.tsv"), "preview pins")?;
    let text = std::str::from_utf8(&manifest)
        .map_err(|_| ProbeError::InvalidInput("preview pins are not UTF-8".into()))?;
    let mut pins = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(ProbeError::InvalidInput(format!(
                "malformed preview pin at line {}",
                line_no + 1
            )));
        }
        pins.push(PreviewSourcePin {
            name: fields[0].to_owned(),
            size: fields[1]
                .parse()
                .map_err(|_| ProbeError::InvalidInput("invalid preview pin size".into()))?,
            sha256: fields[2].to_owned(),
        });
    }
    let files = validate_preview_copy(&input, &pins)?;
    probe_with_files(files, &terrain, &scene)
}

fn sha256_hex(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

const NATIVE_REGISTRY: &str = r#"state	1	0	-1	0	1	1	111111	1,1,1,1,1,1	1
state	2	0	0	0	1	1	111111	2,0,3,3,3,3	1
state	2	0	17	0	1	1	111111	2,0,3,3,3,3	1
state	2	0	18	0	1	1	111111	2,0,3,3,3,3	1
state	2	0	31	0	1	1	111111	2,0,3,3,3,3	1
state	2	0	37	0	1	1	111111	2,0,3,3,3,3	1
state	3	0	-1	0	1	1	111111	2,2,2,2,2,2	1
state	7	0	-1	0	1	1	111111	17,17,17,17,17,17	1
state	8	4	-1	4	0	0	000000	205,205,206,206,206,206	0
state	8	5	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	0	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	1	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	2	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	3	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	4	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	5	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	6	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	7	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	9	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	10	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	11	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	12	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	13	-1	4	0	0	000000	205,205,206,206,206,206	0
state	9	14	-1	4	0	0	000000	205,205,206,206,206,206	0
state	10	0	-1	4	0	0	111111	237,237,238,238,238,238	0
state	11	0	-1	4	0	0	111111	237,237,238,238,238,238	0
state	11	2	-1	4	0	0	111111	237,237,238,238,238,238	0
state	11	4	-1	4	0	0	111111	237,237,238,238,238,238	0
state	11	6	-1	4	0	0	111111	237,237,238,238,238,238	0
state	12	0	-1	0	1	1	111111	18,18,18,18,18,18	1
state	13	0	-1	0	1	1	111111	19,19,19,19,19,19	1
state	14	0	-1	0	1	1	111111	32,32,32,32,32,32	1
state	15	0	-1	0	1	1	111111	33,33,33,33,33,33	1
state	16	0	-1	0	1	1	111111	34,34,34,34,34,34	1
state	17	0	-1	0	1	1	111111	21,21,20,20,20,20	1
state	17	2	-1	0	1	1	111111	21,21,117,117,117,117	1
state	18	0	-1	0	1	1	000000	52,52,52,52,52,52	0
state	18	2	-1	0	1	1	000000	52,52,52,52,52,52	0
state	18	8	-1	0	1	1	000000	52,52,52,52,52,52	0
state	18	10	-1	0	1	1	000000	52,52,52,52,52,52	0
state	21	0	-1	0	1	1	111111	160,160,160,160,160,160	1
state	24	0	-1	0	1	1	111111	208,176,192,192,192,192	1
state	31	1	-1	1	0	0	000000	39,39,39,39,39,39	0
state	31	2	-1	1	0	0	000000	56,56,56,56,56,56	0
state	37	0	-1	1	0	0	000000	13,13,13,13,13,13	0
state	39	0	-1	1	0	0	000000	29,29,29,29,29,29	0
state	40	0	-1	1	0	0	000000	28,28,28,28,28,28	0
state	49	0	-1	0	1	1	111111	37,37,37,37,37,37	1
state	56	0	-1	0	1	1	111111	50,50,50,50,50,50	1
state	73	0	-1	0	1	1	111111	51,51,51,51,51,51	1
neighbor	1	0	0	0	0	1
neighbor	1	0	0	1	0	0
neighbor	1	0	0	3	0	0
neighbor	1	0	0	7	0	0
neighbor	1	0	0	9	0	1
neighbor	1	0	0	9	1	1
neighbor	1	0	0	9	2	1
neighbor	1	0	0	9	3	1
neighbor	1	0	0	9	4	1
neighbor	1	0	0	9	5	1
neighbor	1	0	0	11	0	1
neighbor	1	0	0	13	0	0
neighbor	1	0	0	14	0	0
neighbor	1	0	0	15	0	0
neighbor	1	0	0	16	0	0
neighbor	1	0	0	21	0	0
neighbor	1	0	0	56	0	0
neighbor	1	0	0	73	0	0
neighbor	1	0	1	0	0	1
neighbor	1	0	1	1	0	0
neighbor	1	0	1	3	0	0
neighbor	1	0	1	7	0	0
neighbor	1	0	1	8	4	1
neighbor	1	0	1	8	5	1
neighbor	1	0	1	9	0	1
neighbor	1	0	1	9	1	1
neighbor	1	0	1	9	2	1
neighbor	1	0	1	9	3	1
neighbor	1	0	1	9	4	1
neighbor	1	0	1	9	5	1
neighbor	1	0	1	9	6	1
neighbor	1	0	1	9	7	1
neighbor	1	0	1	9	9	1
neighbor	1	0	1	9	10	1
neighbor	1	0	1	9	11	1
neighbor	1	0	1	9	12	1
neighbor	1	0	1	9	13	1
neighbor	1	0	1	9	14	1
neighbor	1	0	1	10	0	1
neighbor	1	0	1	11	0	1
neighbor	1	0	1	11	2	1
neighbor	1	0	1	11	4	1
neighbor	1	0	1	11	6	1
neighbor	1	0	1	12	0	0
neighbor	1	0	1	13	0	0
neighbor	1	0	1	14	0	0
neighbor	1	0	1	15	0	0
neighbor	1	0	1	16	0	0
neighbor	1	0	1	21	0	0
neighbor	1	0	1	24	0	0
neighbor	1	0	1	40	0	1
neighbor	1	0	1	49	0	0
neighbor	1	0	1	56	0	0
neighbor	1	0	1	73	0	0
neighbor	1	0	2	0	0	1
neighbor	1	0	2	1	0	0
neighbor	1	0	2	3	0	0
neighbor	1	0	2	7	0	0
neighbor	1	0	2	8	5	1
neighbor	1	0	2	9	0	1
neighbor	1	0	2	9	1	1
neighbor	1	0	2	9	2	1
neighbor	1	0	2	9	3	1
neighbor	1	0	2	9	4	1
neighbor	1	0	2	9	5	1
neighbor	1	0	2	9	6	1
neighbor	1	0	2	9	7	1
neighbor	1	0	2	9	9	1
neighbor	1	0	2	9	10	1
neighbor	1	0	2	9	11	1
neighbor	1	0	2	9	14	1
neighbor	1	0	2	10	0	1
neighbor	1	0	2	11	0	1
neighbor	1	0	2	11	2	1
neighbor	1	0	2	11	6	1
neighbor	1	0	2	12	0	0
neighbor	1	0	2	13	0	0
neighbor	1	0	2	14	0	0
neighbor	1	0	2	15	0	0
neighbor	1	0	2	16	0	0
neighbor	1	0	2	21	0	0
neighbor	1	0	2	24	0	0
neighbor	1	0	2	49	0	0
neighbor	1	0	2	56	0	0
neighbor	1	0	2	73	0	0
neighbor	1	0	3	0	0	1
neighbor	1	0	3	1	0	0
neighbor	1	0	3	3	0	0
neighbor	1	0	3	7	0	0
neighbor	1	0	3	9	0	1
neighbor	1	0	3	9	1	1
neighbor	1	0	3	9	2	1
neighbor	1	0	3	9	3	1
neighbor	1	0	3	9	4	1
neighbor	1	0	3	9	5	1
neighbor	1	0	3	9	6	1
neighbor	1	0	3	9	7	1
neighbor	1	0	3	9	9	1
neighbor	1	0	3	9	11	1
neighbor	1	0	3	9	12	1
neighbor	1	0	3	9	13	1
neighbor	1	0	3	10	0	1
neighbor	1	0	3	11	0	1
neighbor	1	0	3	11	2	1
neighbor	1	0	3	11	4	1
neighbor	1	0	3	11	6	1
neighbor	1	0	3	12	0	0
neighbor	1	0	3	13	0	0
neighbor	1	0	3	14	0	0
neighbor	1	0	3	15	0	0
neighbor	1	0	3	16	0	0
neighbor	1	0	3	21	0	0
neighbor	1	0	3	24	0	0
neighbor	1	0	3	56	0	0
neighbor	1	0	3	73	0	0
neighbor	1	0	4	0	0	1
neighbor	1	0	4	1	0	0
neighbor	1	0	4	2	0	0
neighbor	1	0	4	3	0	0
neighbor	1	0	4	7	0	0
neighbor	1	0	4	8	5	1
neighbor	1	0	4	9	0	1
neighbor	1	0	4	9	1	1
neighbor	1	0	4	9	2	1
neighbor	1	0	4	9	3	1
neighbor	1	0	4	9	5	1
neighbor	1	0	4	9	6	1
neighbor	1	0	4	9	9	1
neighbor	1	0	4	9	12	1
neighbor	1	0	4	9	13	1
neighbor	1	0	4	10	0	1
neighbor	1	0	4	11	0	1
neighbor	1	0	4	12	0	0
neighbor	1	0	4	13	0	0
neighbor	1	0	4	14	0	0
neighbor	1	0	4	15	0	0
neighbor	1	0	4	16	0	0
neighbor	1	0	4	21	0	0
neighbor	1	0	4	24	0	0
neighbor	1	0	4	56	0	0
neighbor	1	0	4	73	0	0
neighbor	1	0	5	0	0	1
neighbor	1	0	5	1	0	0
neighbor	1	0	5	3	0	0
neighbor	1	0	5	7	0	0
neighbor	1	0	5	9	0	1
neighbor	1	0	5	9	1	1
neighbor	1	0	5	9	2	1
neighbor	1	0	5	9	3	1
neighbor	1	0	5	9	4	1
neighbor	1	0	5	9	5	1
neighbor	1	0	5	9	6	1
neighbor	1	0	5	9	9	1
neighbor	1	0	5	9	10	1
neighbor	1	0	5	9	11	1
neighbor	1	0	5	9	13	1
neighbor	1	0	5	9	14	1
neighbor	1	0	5	10	0	1
neighbor	1	0	5	11	0	1
neighbor	1	0	5	11	6	1
neighbor	1	0	5	12	0	0
neighbor	1	0	5	13	0	0
neighbor	1	0	5	14	0	0
neighbor	1	0	5	15	0	0
neighbor	1	0	5	16	0	0
neighbor	1	0	5	21	0	0
neighbor	1	0	5	24	0	0
neighbor	1	0	5	49	0	0
neighbor	1	0	5	56	0	0
neighbor	1	0	5	73	0	0
neighbor	2	0	0	3	0	0
neighbor	2	0	1	0	0	1
neighbor	2	0	1	17	0	0
neighbor	2	0	1	18	0	0
neighbor	2	0	1	18	2	0
neighbor	2	0	1	18	8	0
neighbor	2	0	1	31	1	1
neighbor	2	0	1	31	2	1
neighbor	2	0	1	37	0	1
neighbor	2	0	2	0	0	1
neighbor	2	0	2	2	0	0
neighbor	2	0	2	3	0	0
neighbor	2	0	2	17	0	0
neighbor	2	0	2	17	2	0
neighbor	2	0	2	18	0	0
neighbor	2	0	2	31	1	1
neighbor	2	0	2	31	2	1
neighbor	2	0	2	37	0	1
neighbor	2	0	3	0	0	1
neighbor	2	0	3	2	0	0
neighbor	2	0	3	3	0	0
neighbor	2	0	3	17	0	0
neighbor	2	0	3	17	2	0
neighbor	2	0	3	18	0	0
neighbor	2	0	3	18	2	0
neighbor	2	0	3	31	1	1
neighbor	2	0	3	31	2	1
neighbor	2	0	4	0	0	1
neighbor	2	0	4	2	0	0
neighbor	2	0	4	3	0	0
neighbor	2	0	4	17	0	0
neighbor	2	0	4	17	2	0
neighbor	2	0	4	18	0	0
neighbor	2	0	4	18	2	0
neighbor	2	0	4	18	8	0
neighbor	2	0	4	31	1	1
neighbor	2	0	4	31	2	1
neighbor	2	0	4	37	0	1
neighbor	2	0	5	0	0	1
neighbor	2	0	5	1	0	0
neighbor	2	0	5	2	0	0
neighbor	2	0	5	3	0	0
neighbor	2	0	5	17	0	0
neighbor	2	0	5	31	1	1
neighbor	2	0	5	31	2	1
neighbor	3	0	0	0	0	1
neighbor	3	0	0	1	0	0
neighbor	3	0	0	3	0	0
neighbor	3	0	0	7	0	0
neighbor	3	0	0	10	0	1
neighbor	3	0	0	11	0	1
neighbor	3	0	0	13	0	0
neighbor	3	0	0	14	0	0
neighbor	3	0	0	15	0	0
neighbor	3	0	0	16	0	0
neighbor	3	0	0	21	0	0
neighbor	3	0	0	56	0	0
neighbor	3	0	0	73	0	0
neighbor	3	0	1	0	0	1
neighbor	3	0	1	1	0	0
neighbor	3	0	1	2	0	0
neighbor	3	0	1	3	0	0
neighbor	3	0	1	7	0	0
neighbor	3	0	1	9	0	1
neighbor	3	0	1	9	1	1
neighbor	3	0	1	9	9	1
neighbor	3	0	1	9	10	1
neighbor	3	0	1	10	0	1
neighbor	3	0	1	11	0	1
neighbor	3	0	1	12	0	0
neighbor	3	0	1	13	0	0
neighbor	3	0	1	14	0	0
neighbor	3	0	1	15	0	0
neighbor	3	0	1	16	0	0
neighbor	3	0	1	17	0	0
neighbor	3	0	1	17	2	0
neighbor	3	0	1	21	0	0
neighbor	3	0	1	24	0	0
neighbor	3	0	1	56	0	0
neighbor	3	0	1	73	0	0
neighbor	3	0	2	0	0	1
neighbor	3	0	2	1	0	0
neighbor	3	0	2	2	0	0
neighbor	3	0	2	3	0	0
neighbor	3	0	2	7	0	0
neighbor	3	0	2	9	0	1
neighbor	3	0	2	9	1	1
neighbor	3	0	2	9	9	1
neighbor	3	0	2	10	0	1
neighbor	3	0	2	11	0	1
neighbor	3	0	2	12	0	0
neighbor	3	0	2	13	0	0
neighbor	3	0	2	14	0	0
neighbor	3	0	2	15	0	0
neighbor	3	0	2	16	0	0
neighbor	3	0	2	24	0	0
neighbor	3	0	2	31	1	1
neighbor	3	0	2	31	2	1
neighbor	3	0	2	56	0	0
neighbor	3	0	2	73	0	0
neighbor	3	0	3	0	0	1
neighbor	3	0	3	1	0	0
neighbor	3	0	3	2	0	0
neighbor	3	0	3	3	0	0
neighbor	3	0	3	7	0	0
neighbor	3	0	3	9	0	1
neighbor	3	0	3	9	10	1
neighbor	3	0	3	10	0	1
neighbor	3	0	3	11	0	1
neighbor	3	0	3	12	0	0
neighbor	3	0	3	13	0	0
neighbor	3	0	3	14	0	0
neighbor	3	0	3	15	0	0
neighbor	3	0	3	16	0	0
neighbor	3	0	3	24	0	0
neighbor	3	0	3	31	1	1
neighbor	3	0	3	56	0	0
neighbor	3	0	3	73	0	0
neighbor	3	0	4	0	0	1
neighbor	3	0	4	1	0	0
neighbor	3	0	4	2	0	0
neighbor	3	0	4	3	0	0
neighbor	3	0	4	7	0	0
neighbor	3	0	4	9	0	1
neighbor	3	0	4	9	1	1
neighbor	3	0	4	9	2	1
neighbor	3	0	4	9	3	1
neighbor	3	0	4	9	9	1
neighbor	3	0	4	9	10	1
neighbor	3	0	4	9	11	1
neighbor	3	0	4	10	0	1
neighbor	3	0	4	11	0	1
neighbor	3	0	4	12	0	0
neighbor	3	0	4	13	0	0
neighbor	3	0	4	14	0	0
neighbor	3	0	4	15	0	0
neighbor	3	0	4	16	0	0
neighbor	3	0	4	17	0	0
neighbor	3	0	4	21	0	0
neighbor	3	0	4	24	0	0
neighbor	3	0	4	31	1	1
neighbor	3	0	4	56	0	0
neighbor	3	0	4	73	0	0
neighbor	3	0	5	0	0	1
neighbor	3	0	5	1	0	0
neighbor	3	0	5	2	0	0
neighbor	3	0	5	3	0	0
neighbor	3	0	5	7	0	0
neighbor	3	0	5	9	0	1
neighbor	3	0	5	11	0	1
neighbor	3	0	5	12	0	0
neighbor	3	0	5	13	0	0
neighbor	3	0	5	14	0	0
neighbor	3	0	5	15	0	0
neighbor	3	0	5	16	0	0
neighbor	3	0	5	21	0	0
neighbor	3	0	5	24	0	0
neighbor	3	0	5	31	1	1
neighbor	3	0	5	73	0	0
neighbor	7	0	0	0	0	1
neighbor	7	0	0	1	0	0
neighbor	7	0	0	3	0	0
neighbor	7	0	0	7	0	0
neighbor	7	0	0	13	0	0
neighbor	7	0	0	15	0	0
neighbor	7	0	0	16	0	0
neighbor	7	0	0	56	0	0
neighbor	7	0	0	73	0	0
neighbor	7	0	1	0	0	1
neighbor	7	0	1	1	0	0
neighbor	7	0	1	3	0	0
neighbor	7	0	1	7	0	0
neighbor	7	0	1	9	0	1
neighbor	7	0	1	13	0	0
neighbor	7	0	1	14	0	0
neighbor	7	0	1	15	0	0
neighbor	7	0	1	16	0	0
neighbor	7	0	1	56	0	0
neighbor	7	0	1	73	0	0
neighbor	7	0	2	0	0	1
neighbor	7	0	2	1	0	0
neighbor	7	0	2	3	0	0
neighbor	7	0	2	7	0	0
neighbor	7	0	2	9	0	1
neighbor	7	0	2	13	0	0
neighbor	7	0	2	14	0	0
neighbor	7	0	2	15	0	0
neighbor	7	0	2	16	0	0
neighbor	7	0	2	56	0	0
neighbor	7	0	2	73	0	0
neighbor	7	0	3	0	0	1
neighbor	7	0	3	1	0	0
neighbor	7	0	3	3	0	0
neighbor	7	0	3	7	0	0
neighbor	7	0	3	9	0	1
neighbor	7	0	3	13	0	0
neighbor	7	0	3	14	0	0
neighbor	7	0	3	15	0	0
neighbor	7	0	3	16	0	0
neighbor	7	0	3	56	0	0
neighbor	7	0	3	73	0	0
neighbor	7	0	4	0	0	1
neighbor	7	0	4	1	0	0
neighbor	7	0	4	3	0	0
neighbor	7	0	4	7	0	0
neighbor	7	0	4	9	0	1
neighbor	7	0	4	13	0	0
neighbor	7	0	4	14	0	0
neighbor	7	0	4	15	0	0
neighbor	7	0	4	16	0	0
neighbor	7	0	4	56	0	0
neighbor	7	0	4	73	0	0
neighbor	7	0	5	0	0	1
neighbor	7	0	5	1	0	0
neighbor	7	0	5	3	0	0
neighbor	7	0	5	7	0	0
neighbor	7	0	5	9	0	1
neighbor	7	0	5	13	0	0
neighbor	7	0	5	14	0	0
neighbor	7	0	5	15	0	0
neighbor	7	0	5	16	0	0
neighbor	7	0	5	56	0	0
neighbor	7	0	5	73	0	0
neighbor	8	4	0	1	0	0
neighbor	8	4	1	0	0	1
neighbor	8	4	2	9	3	1
neighbor	8	4	3	9	5	1
neighbor	8	4	4	9	5	1
neighbor	8	4	5	9	3	1
neighbor	8	5	0	1	0	0
neighbor	8	5	1	0	0	1
neighbor	8	5	2	9	4	1
neighbor	8	5	3	0	0	1
neighbor	8	5	3	1	0	0
neighbor	8	5	4	9	4	1
neighbor	8	5	4	9	6	1
neighbor	8	5	5	1	0	0
neighbor	8	5	5	9	6	1
neighbor	9	0	0	1	0	0
neighbor	9	0	0	3	0	0
neighbor	9	0	0	7	0	0
neighbor	9	0	0	9	0	1
neighbor	9	0	0	12	0	0
neighbor	9	0	0	13	0	0
neighbor	9	0	0	15	0	0
neighbor	9	0	0	16	0	0
neighbor	9	0	0	56	0	0
neighbor	9	0	0	73	0	0
neighbor	9	0	1	0	0	1
neighbor	9	0	1	1	0	1
neighbor	9	0	1	9	0	1
neighbor	9	0	2	0	0	1
neighbor	9	0	2	1	0	0
neighbor	9	0	2	3	0	0
neighbor	9	0	2	7	0	0
neighbor	9	0	2	9	0	1
neighbor	9	0	2	9	1	1
neighbor	9	0	2	12	0	0
neighbor	9	0	2	13	0	0
neighbor	9	0	2	16	0	0
neighbor	9	0	3	0	0	1
neighbor	9	0	3	1	0	0
neighbor	9	0	3	3	0	0
neighbor	9	0	3	7	0	0
neighbor	9	0	3	9	0	1
neighbor	9	0	3	12	0	0
neighbor	9	0	3	13	0	0
neighbor	9	0	4	0	0	1
neighbor	9	0	4	1	0	0
neighbor	9	0	4	3	0	0
neighbor	9	0	4	7	0	0
neighbor	9	0	4	9	0	1
neighbor	9	0	4	9	1	1
neighbor	9	0	4	12	0	0
neighbor	9	0	4	13	0	0
neighbor	9	0	5	0	0	1
neighbor	9	0	5	1	0	0
neighbor	9	0	5	3	0	0
neighbor	9	0	5	7	0	0
neighbor	9	0	5	9	0	1
neighbor	9	0	5	9	1	1
neighbor	9	0	5	12	0	0
neighbor	9	0	5	13	0	0
neighbor	9	0	5	16	0	0
neighbor	9	1	0	1	0	0
neighbor	9	1	0	3	0	0
neighbor	9	1	0	9	2	1
neighbor	9	1	0	9	9	1
neighbor	9	1	0	49	0	0
neighbor	9	1	1	0	0	1
neighbor	9	1	1	1	0	1
neighbor	9	1	2	0	0	1
neighbor	9	1	2	1	0	0
neighbor	9	1	2	9	1	1
neighbor	9	1	2	9	2	1
neighbor	9	1	2	9	9	1
neighbor	9	1	2	9	11	1
neighbor	9	1	2	9	12	1
neighbor	9	1	2	9	13	1
neighbor	9	1	3	0	0	1
neighbor	9	1	3	1	0	0
neighbor	9	1	3	3	0	0
neighbor	9	1	3	9	0	1
neighbor	9	1	3	9	1	1
neighbor	9	1	3	9	2	1
neighbor	9	1	3	9	9	1
neighbor	9	1	3	9	10	1
neighbor	9	1	3	9	11	1
neighbor	9	1	3	9	13	1
neighbor	9	1	4	0	0	1
neighbor	9	1	4	1	0	0
neighbor	9	1	4	9	0	1
neighbor	9	1	4	9	2	1
neighbor	9	1	4	9	9	1
neighbor	9	1	4	9	10	1
neighbor	9	1	4	9	11	1
neighbor	9	1	5	0	0	1
neighbor	9	1	5	1	0	0
neighbor	9	1	5	3	0	0
neighbor	9	1	5	9	0	1
neighbor	9	1	5	9	2	1
neighbor	9	1	5	9	9	1
neighbor	9	1	5	9	10	1
neighbor	9	1	5	9	11	1
neighbor	9	1	5	9	12	1
neighbor	9	1	5	9	13	1
neighbor	9	1	5	21	0	0
neighbor	9	2	0	1	0	0
neighbor	9	2	0	9	10	1
neighbor	9	2	0	11	0	1
neighbor	9	2	0	13	0	0
neighbor	9	2	0	14	0	0
neighbor	9	2	0	49	0	0
neighbor	9	2	1	0	0	1
neighbor	9	2	1	1	0	1
neighbor	9	2	1	9	1	1
neighbor	9	2	1	9	5	1
neighbor	9	2	2	0	0	1
neighbor	9	2	2	1	0	0
neighbor	9	2	2	9	1	1
neighbor	9	2	2	9	2	1
neighbor	9	2	2	9	3	1
neighbor	9	2	3	0	0	1
neighbor	9	2	3	1	0	0
neighbor	9	2	3	9	1	1
neighbor	9	2	3	9	2	1
neighbor	9	2	3	9	3	1
neighbor	9	2	3	9	5	1
neighbor	9	2	4	0	0	1
neighbor	9	2	4	1	0	0
neighbor	9	2	4	9	1	1
neighbor	9	2	4	9	3	1
neighbor	9	2	5	0	0	1
neighbor	9	2	5	1	0	0
neighbor	9	2	5	3	0	0
neighbor	9	2	5	9	1	1
neighbor	9	2	5	9	3	1
neighbor	9	3	0	1	0	0
neighbor	9	3	0	9	11	1
neighbor	9	3	0	11	0	1
neighbor	9	3	0	13	0	0
neighbor	9	3	0	49	0	0
neighbor	9	3	1	0	0	1
neighbor	9	3	1	1	0	1
neighbor	9	3	2	0	0	1
neighbor	9	3	2	1	0	0
neighbor	9	3	2	9	2	1
neighbor	9	3	2	9	4	1
neighbor	9	3	3	0	0	1
neighbor	9	3	3	1	0	0
neighbor	9	3	3	8	4	1
neighbor	9	3	3	9	2	1
neighbor	9	3	3	9	4	1
neighbor	9	3	4	0	0	1
neighbor	9	3	4	1	0	0
neighbor	9	3	4	8	4	1
neighbor	9	3	4	9	2	1
neighbor	9	3	4	9	4	1
neighbor	9	3	4	9	13	1
neighbor	9	3	5	0	0	1
neighbor	9	3	5	1	0	0
neighbor	9	3	5	3	0	0
neighbor	9	3	5	9	2	1
neighbor	9	3	5	9	4	1
neighbor	9	4	0	1	0	0
neighbor	9	4	0	9	12	1
neighbor	9	4	0	11	0	1
neighbor	9	4	0	13	0	0
neighbor	9	4	0	49	0	0
neighbor	9	4	1	0	0	1
neighbor	9	4	1	1	0	1
neighbor	9	4	2	0	0	1
neighbor	9	4	2	1	0	0
neighbor	9	4	2	9	3	1
neighbor	9	4	2	9	4	1
neighbor	9	4	2	9	5	1
neighbor	9	4	3	0	0	1
neighbor	9	4	3	1	0	0
neighbor	9	4	3	8	5	1
neighbor	9	4	3	9	3	1
neighbor	9	4	3	9	4	1
neighbor	9	4	3	9	5	1
neighbor	9	4	4	0	0	1
neighbor	9	4	4	1	0	0
neighbor	9	4	4	9	3	1
neighbor	9	4	4	9	5	1
neighbor	9	4	4	9	14	1
neighbor	9	4	5	0	0	1
neighbor	9	4	5	8	5	1
neighbor	9	4	5	9	3	1
neighbor	9	4	5	9	5	1
neighbor	9	4	5	13	0	0
neighbor	9	5	0	1	0	0
neighbor	9	5	0	9	2	1
neighbor	9	5	0	9	13	1
neighbor	9	5	0	11	0	1
neighbor	9	5	0	13	0	0
neighbor	9	5	1	0	0	1
neighbor	9	5	1	1	0	1
neighbor	9	5	2	0	0	1
neighbor	9	5	2	1	0	0
neighbor	9	5	2	8	4	1
neighbor	9	5	2	9	2	1
neighbor	9	5	2	9	4	1
neighbor	9	5	2	9	5	1
neighbor	9	5	2	9	6	1
neighbor	9	5	3	0	0	1
neighbor	9	5	3	1	0	0
neighbor	9	5	3	9	4	1
neighbor	9	5	3	9	5	1
neighbor	9	5	3	9	6	1
neighbor	9	5	3	15	0	0
neighbor	9	5	4	0	0	1
neighbor	9	5	4	1	0	0
neighbor	9	5	4	9	4	1
neighbor	9	5	4	9	6	1
neighbor	9	5	5	0	0	1
neighbor	9	5	5	1	0	0
neighbor	9	5	5	8	4	1
neighbor	9	5	5	9	4	1
neighbor	9	5	5	9	6	1
neighbor	9	6	0	1	0	0
neighbor	9	6	0	9	14	1
neighbor	9	6	0	13	0	0
neighbor	9	6	1	0	0	1
neighbor	9	6	2	1	0	0
neighbor	9	6	2	9	5	1
neighbor	9	6	2	9	6	1
neighbor	9	6	2	9	7	1
neighbor	9	6	2	13	0	0
neighbor	9	6	3	0	0	1
neighbor	9	6	3	1	0	0
neighbor	9	6	3	9	5	1
neighbor	9	6	3	9	6	1
neighbor	9	6	3	9	7	1
neighbor	9	6	3	13	0	0
neighbor	9	6	4	1	0	0
neighbor	9	6	4	8	5	1
neighbor	9	6	4	9	5	1
neighbor	9	6	4	9	7	1
neighbor	9	6	4	13	0	0
neighbor	9	6	5	0	0	1
neighbor	9	6	5	1	0	0
neighbor	9	6	5	8	5	1
neighbor	9	6	5	9	5	1
neighbor	9	6	5	9	7	1
neighbor	9	7	0	1	0	0
neighbor	9	7	0	13	0	0
neighbor	9	7	1	0	0	1
neighbor	9	7	2	0	0	1
neighbor	9	7	2	1	0	0
neighbor	9	7	2	9	6	1
neighbor	9	7	3	0	0	1
neighbor	9	7	3	1	0	0
neighbor	9	7	3	9	6	1
neighbor	9	7	3	13	0	0
neighbor	9	7	4	0	0	1
neighbor	9	7	4	9	6	1
neighbor	9	7	4	13	0	0
neighbor	9	7	5	0	0	1
neighbor	9	7	5	9	6	1
neighbor	9	9	0	1	0	0
neighbor	9	9	0	3	0	0
neighbor	9	9	0	9	9	1
neighbor	9	9	1	9	1	1
neighbor	9	9	1	9	9	1
neighbor	9	9	2	0	0	1
neighbor	9	9	2	1	0	0
neighbor	9	9	2	9	1	1
neighbor	9	9	2	9	10	1
neighbor	9	9	3	0	0	1
neighbor	9	9	3	1	0	0
neighbor	9	9	3	3	0	0
neighbor	9	9	3	9	1	1
neighbor	9	9	4	0	0	1
neighbor	9	9	4	1	0	0
neighbor	9	9	4	9	1	1
neighbor	9	9	5	0	0	1
neighbor	9	9	5	1	0	0
neighbor	9	9	5	3	0	0
neighbor	9	9	5	9	1	1
neighbor	9	9	5	21	0	0
neighbor	9	10	0	1	0	0
neighbor	9	10	0	3	0	0
neighbor	9	10	1	9	2	1
neighbor	9	10	2	3	0	0
neighbor	9	10	2	9	1	1
neighbor	9	10	3	1	0	0
neighbor	9	10	3	9	9	1
neighbor	9	10	4	1	0	0
neighbor	9	10	4	9	1	1
neighbor	9	10	5	3	0	0
neighbor	9	10	5	9	1	1
neighbor	9	11	0	1	0	0
neighbor	9	11	0	14	0	0
neighbor	9	11	1	9	3	1
neighbor	9	11	2	1	0	0
neighbor	9	11	2	9	1	1
neighbor	9	11	2	14	0	0
neighbor	9	11	3	1	0	0
neighbor	9	11	3	9	1	1
neighbor	9	11	4	1	0	0
neighbor	9	11	4	9	1	1
neighbor	9	11	5	0	0	1
neighbor	9	11	5	3	0	0
neighbor	9	11	5	9	1	1
neighbor	9	11	5	14	0	0
neighbor	9	12	0	1	0	0
neighbor	9	12	0	9	12	1
neighbor	9	12	1	9	4	1
neighbor	9	12	1	9	12	1
neighbor	9	12	2	1	0	0
neighbor	9	12	3	0	0	1
neighbor	9	12	3	9	1	1
neighbor	9	12	4	0	0	1
neighbor	9	12	4	9	1	1
neighbor	9	12	5	1	0	0
neighbor	9	13	0	1	0	0
neighbor	9	13	1	9	5	1
neighbor	9	13	2	1	0	0
neighbor	9	13	2	9	1	1
neighbor	9	13	3	9	1	1
neighbor	9	13	3	9	14	1
neighbor	9	13	4	1	0	0
neighbor	9	13	4	9	1	1
neighbor	9	13	5	1	0	0
neighbor	9	13	5	9	3	1
neighbor	9	14	0	1	0	0
neighbor	9	14	1	9	6	1
neighbor	9	14	2	9	13	1
neighbor	9	14	3	1	0	0
neighbor	9	14	4	1	0	0
neighbor	9	14	5	9	4	1
neighbor	10	0	0	1	0	0
neighbor	10	0	0	3	0	0
neighbor	10	0	0	11	0	1
neighbor	10	0	1	0	0	1
neighbor	10	0	1	3	0	1
neighbor	10	0	1	11	0	1
neighbor	10	0	2	1	0	0
neighbor	10	0	2	3	0	0
neighbor	10	0	2	10	0	1
neighbor	10	0	2	11	0	1
neighbor	10	0	3	1	0	0
neighbor	10	0	3	3	0	0
neighbor	10	0	3	10	0	1
neighbor	10	0	3	11	0	1
neighbor	10	0	4	1	0	0
neighbor	10	0	4	10	0	1
neighbor	10	0	4	11	0	1
neighbor	10	0	5	1	0	0
neighbor	10	0	5	3	0	0
neighbor	10	0	5	10	0	1
neighbor	10	0	5	11	0	1
neighbor	11	0	0	1	0	0
neighbor	11	0	0	3	0	0
neighbor	11	0	0	10	0	1
neighbor	11	0	0	11	0	1
neighbor	11	0	0	13	0	0
neighbor	11	0	0	16	0	0
neighbor	11	0	0	73	0	0
neighbor	11	0	1	0	0	1
neighbor	11	0	1	1	0	1
neighbor	11	0	1	3	0	1
neighbor	11	0	1	9	2	1
neighbor	11	0	1	9	3	1
neighbor	11	0	1	9	4	1
neighbor	11	0	1	9	5	1
neighbor	11	0	1	10	0	1
neighbor	11	0	1	11	0	1
neighbor	11	0	2	1	0	0
neighbor	11	0	2	3	0	0
neighbor	11	0	2	10	0	1
neighbor	11	0	2	11	0	1
neighbor	11	0	2	13	0	0
neighbor	11	0	2	15	0	0
neighbor	11	0	2	16	0	0
neighbor	11	0	2	73	0	0
neighbor	11	0	3	1	0	0
neighbor	11	0	3	3	0	0
neighbor	11	0	3	10	0	1
neighbor	11	0	3	11	0	1
neighbor	11	0	3	13	0	0
neighbor	11	0	3	15	0	0
neighbor	11	0	3	49	0	0
neighbor	11	0	3	73	0	0
neighbor	11	0	4	1	0	0
neighbor	11	0	4	3	0	0
neighbor	11	0	4	10	0	1
neighbor	11	0	4	11	0	1
neighbor	11	0	4	13	0	0
neighbor	11	0	4	15	0	0
neighbor	11	0	4	16	0	0
neighbor	11	0	4	49	0	0
neighbor	11	0	4	73	0	0
neighbor	11	0	5	1	0	0
neighbor	11	0	5	3	0	0
neighbor	11	0	5	10	0	1
neighbor	11	0	5	11	0	1
neighbor	11	0	5	13	0	0
neighbor	11	2	0	1	0	0
neighbor	11	2	1	0	0	1
neighbor	11	2	2	1	0	0
neighbor	11	2	3	1	0	0
neighbor	11	2	4	0	0	1
neighbor	11	2	5	11	4	1
neighbor	11	4	0	1	0	0
neighbor	11	4	1	0	0	1
neighbor	11	4	2	1	0	0
neighbor	11	4	3	11	6	1
neighbor	11	4	4	11	2	1
neighbor	11	4	5	11	6	1
neighbor	11	6	0	1	0	0
neighbor	11	6	1	0	0	1
neighbor	11	6	2	1	0	0
neighbor	11	6	2	11	4	1
neighbor	11	6	3	0	0	1
neighbor	11	6	3	1	0	0
neighbor	11	6	4	1	0	0
neighbor	11	6	4	11	4	1
neighbor	11	6	5	0	0	1
neighbor	12	0	0	1	0	0
neighbor	12	0	0	3	0	0
neighbor	12	0	0	12	0	0
neighbor	12	0	0	13	0	0
neighbor	12	0	0	15	0	0
neighbor	12	0	0	16	0	0
neighbor	12	0	0	24	0	0
neighbor	12	0	1	0	0	1
neighbor	12	0	1	9	0	1
neighbor	12	0	1	12	0	0
neighbor	12	0	2	0	0	1
neighbor	12	0	2	1	0	0
neighbor	12	0	2	3	0	0
neighbor	12	0	2	9	0	1
neighbor	12	0	2	12	0	0
neighbor	12	0	2	16	0	0
neighbor	12	0	2	24	0	0
neighbor	12	0	3	0	0	1
neighbor	12	0	3	1	0	0
neighbor	12	0	3	3	0	0
neighbor	12	0	3	9	0	1
neighbor	12	0	3	12	0	0
neighbor	12	0	3	13	0	0
neighbor	12	0	3	16	0	0
neighbor	12	0	3	24	0	0
neighbor	12	0	4	0	0	1
neighbor	12	0	4	1	0	0
neighbor	12	0	4	3	0	0
neighbor	12	0	4	9	0	1
neighbor	12	0	4	12	0	0
neighbor	12	0	4	16	0	0
neighbor	12	0	4	24	0	0
neighbor	12	0	5	0	0	1
neighbor	12	0	5	1	0	0
neighbor	12	0	5	3	0	0
neighbor	12	0	5	9	0	1
neighbor	12	0	5	12	0	0
neighbor	12	0	5	13	0	0
neighbor	12	0	5	16	0	0
neighbor	12	0	5	24	0	0
neighbor	13	0	0	0	0	1
neighbor	13	0	0	1	0	0
neighbor	13	0	0	3	0	0
neighbor	13	0	0	7	0	0
neighbor	13	0	0	13	0	0
neighbor	13	0	0	14	0	0
neighbor	13	0	0	15	0	0
neighbor	13	0	0	16	0	0
neighbor	13	0	0	73	0	0
neighbor	13	0	1	0	0	1
neighbor	13	0	1	1	0	0
neighbor	13	0	1	3	0	0
neighbor	13	0	1	7	0	0
neighbor	13	0	1	9	0	1
neighbor	13	0	1	9	2	1
neighbor	13	0	1	9	3	1
neighbor	13	0	1	9	4	1
neighbor	13	0	1	9	5	1
neighbor	13	0	1	9	6	1
neighbor	13	0	1	9	7	1
neighbor	13	0	1	11	0	1
neighbor	13	0	1	12	0	0
neighbor	13	0	1	13	0	0
neighbor	13	0	1	15	0	0
neighbor	13	0	1	16	0	0
neighbor	13	0	1	24	0	0
neighbor	13	0	1	73	0	0
neighbor	13	0	2	0	0	1
neighbor	13	0	2	1	0	0
neighbor	13	0	2	3	0	0
neighbor	13	0	2	7	0	0
neighbor	13	0	2	9	0	1
neighbor	13	0	2	9	6	1
neighbor	13	0	2	9	7	1
neighbor	13	0	2	11	0	1
neighbor	13	0	2	12	0	0
neighbor	13	0	2	13	0	0
neighbor	13	0	2	15	0	0
neighbor	13	0	2	16	0	0
neighbor	13	0	2	24	0	0
neighbor	13	0	2	73	0	0
neighbor	13	0	3	0	0	1
neighbor	13	0	3	1	0	0
neighbor	13	0	3	3	0	0
neighbor	13	0	3	7	0	0
neighbor	13	0	3	9	0	1
neighbor	13	0	3	9	6	1
neighbor	13	0	3	11	0	1
neighbor	13	0	3	13	0	0
neighbor	13	0	3	14	0	0
neighbor	13	0	3	15	0	0
neighbor	13	0	3	16	0	0
neighbor	13	0	3	24	0	0
neighbor	13	0	3	56	0	0
neighbor	13	0	3	73	0	0
neighbor	13	0	4	0	0	1
neighbor	13	0	4	1	0	0
neighbor	13	0	4	3	0	0
neighbor	13	0	4	7	0	0
neighbor	13	0	4	9	0	1
neighbor	13	0	4	9	4	1
neighbor	13	0	4	11	0	1
neighbor	13	0	4	12	0	0
neighbor	13	0	4	13	0	0
neighbor	13	0	4	14	0	0
neighbor	13	0	4	15	0	0
neighbor	13	0	4	16	0	0
neighbor	13	0	4	24	0	0
neighbor	13	0	4	73	0	0
neighbor	13	0	5	0	0	1
neighbor	13	0	5	1	0	0
neighbor	13	0	5	3	0	0
neighbor	13	0	5	7	0	0
neighbor	13	0	5	9	0	1
neighbor	13	0	5	9	6	1
neighbor	13	0	5	9	7	1
neighbor	13	0	5	11	0	1
neighbor	13	0	5	13	0	0
neighbor	13	0	5	15	0	0
neighbor	13	0	5	16	0	0
neighbor	13	0	5	21	0	0
neighbor	13	0	5	24	0	0
neighbor	13	0	5	73	0	0
neighbor	14	0	0	0	0	1
neighbor	14	0	0	1	0	0
neighbor	14	0	0	3	0	0
neighbor	14	0	0	7	0	0
neighbor	14	0	0	14	0	0
neighbor	14	0	0	15	0	0
neighbor	14	0	0	16	0	0
neighbor	14	0	1	1	0	0
neighbor	14	0	1	3	0	0
neighbor	14	0	1	9	2	1
neighbor	14	0	1	9	11	1
neighbor	14	0	1	13	0	0
neighbor	14	0	1	14	0	0
neighbor	14	0	1	15	0	0
neighbor	14	0	1	16	0	0
neighbor	14	0	2	0	0	1
neighbor	14	0	2	1	0	0
neighbor	14	0	2	3	0	0
neighbor	14	0	2	7	0	0
neighbor	14	0	2	13	0	0
neighbor	14	0	2	14	0	0
neighbor	14	0	2	15	0	0
neighbor	14	0	2	16	0	0
neighbor	14	0	3	0	0	1
neighbor	14	0	3	1	0	0
neighbor	14	0	3	3	0	0
neighbor	14	0	3	7	0	0
neighbor	14	0	3	9	11	1
neighbor	14	0	3	14	0	0
neighbor	14	0	3	15	0	0
neighbor	14	0	3	16	0	0
neighbor	14	0	4	0	0	1
neighbor	14	0	4	1	0	0
neighbor	14	0	4	3	0	0
neighbor	14	0	4	7	0	0
neighbor	14	0	4	9	11	1
neighbor	14	0	4	14	0	0
neighbor	14	0	4	15	0	0
neighbor	14	0	4	16	0	0
neighbor	14	0	5	0	0	1
neighbor	14	0	5	1	0	0
neighbor	14	0	5	3	0	0
neighbor	14	0	5	7	0	0
neighbor	14	0	5	13	0	0
neighbor	14	0	5	14	0	0
neighbor	14	0	5	16	0	0
neighbor	15	0	0	0	0	1
neighbor	15	0	0	1	0	0
neighbor	15	0	0	3	0	0
neighbor	15	0	0	7	0	0
neighbor	15	0	0	13	0	0
neighbor	15	0	0	14	0	0
neighbor	15	0	0	15	0	0
neighbor	15	0	0	16	0	0
neighbor	15	0	0	21	0	0
neighbor	15	0	0	73	0	0
neighbor	15	0	1	0	0	1
neighbor	15	0	1	1	0	0
neighbor	15	0	1	3	0	0
neighbor	15	0	1	7	0	0
neighbor	15	0	1	9	0	1
neighbor	15	0	1	12	0	0
neighbor	15	0	1	13	0	0
neighbor	15	0	1	14	0	0
neighbor	15	0	1	15	0	0
neighbor	15	0	1	16	0	0
neighbor	15	0	1	24	0	0
neighbor	15	0	1	56	0	0
neighbor	15	0	1	73	0	0
neighbor	15	0	2	0	0	1
neighbor	15	0	2	1	0	0
neighbor	15	0	2	3	0	0
neighbor	15	0	2	7	0	0
neighbor	15	0	2	9	5	1
neighbor	15	0	2	11	0	1
neighbor	15	0	2	13	0	0
neighbor	15	0	2	14	0	0
neighbor	15	0	2	15	0	0
neighbor	15	0	2	16	0	0
neighbor	15	0	2	24	0	0
neighbor	15	0	2	56	0	0
neighbor	15	0	2	73	0	0
neighbor	15	0	3	0	0	1
neighbor	15	0	3	1	0	0
neighbor	15	0	3	3	0	0
neighbor	15	0	3	7	0	0
neighbor	15	0	3	11	0	1
neighbor	15	0	3	13	0	0
neighbor	15	0	3	14	0	0
neighbor	15	0	3	15	0	0
neighbor	15	0	3	16	0	0
neighbor	15	0	3	21	0	0
neighbor	15	0	3	24	0	0
neighbor	15	0	3	73	0	0
neighbor	15	0	4	0	0	1
neighbor	15	0	4	1	0	0
neighbor	15	0	4	3	0	0
neighbor	15	0	4	7	0	0
neighbor	15	0	4	13	0	0
neighbor	15	0	4	15	0	0
neighbor	15	0	4	16	0	0
neighbor	15	0	4	24	0	0
neighbor	15	0	4	56	0	0
neighbor	15	0	4	73	0	0
neighbor	15	0	5	0	0	1
neighbor	15	0	5	1	0	0
neighbor	15	0	5	3	0	0
neighbor	15	0	5	7	0	0
neighbor	15	0	5	11	0	1
neighbor	15	0	5	13	0	0
neighbor	15	0	5	14	0	0
neighbor	15	0	5	15	0	0
neighbor	15	0	5	16	0	0
neighbor	15	0	5	21	0	0
neighbor	15	0	5	24	0	0
neighbor	15	0	5	56	0	0
neighbor	15	0	5	73	0	0
neighbor	16	0	0	0	0	1
neighbor	16	0	0	1	0	0
neighbor	16	0	0	3	0	0
neighbor	16	0	0	7	0	0
neighbor	16	0	0	13	0	0
neighbor	16	0	0	14	0	0
neighbor	16	0	0	15	0	0
neighbor	16	0	0	16	0	0
neighbor	16	0	0	21	0	0
neighbor	16	0	0	73	0	0
neighbor	16	0	1	0	0	1
neighbor	16	0	1	1	0	0
neighbor	16	0	1	3	0	0
neighbor	16	0	1	7	0	0
neighbor	16	0	1	9	0	1
neighbor	16	0	1	11	0	1
neighbor	16	0	1	12	0	0
neighbor	16	0	1	13	0	0
neighbor	16	0	1	14	0	0
neighbor	16	0	1	15	0	0
neighbor	16	0	1	16	0	0
neighbor	16	0	1	21	0	0
neighbor	16	0	1	24	0	0
neighbor	16	0	1	73	0	0
neighbor	16	0	2	0	0	1
neighbor	16	0	2	1	0	0
neighbor	16	0	2	3	0	0
neighbor	16	0	2	7	0	0
neighbor	16	0	2	12	0	0
neighbor	16	0	2	13	0	0
neighbor	16	0	2	14	0	0
neighbor	16	0	2	15	0	0
neighbor	16	0	2	16	0	0
neighbor	16	0	2	24	0	0
neighbor	16	0	2	73	0	0
neighbor	16	0	3	0	0	1
neighbor	16	0	3	1	0	0
neighbor	16	0	3	3	0	0
neighbor	16	0	3	7	0	0
neighbor	16	0	3	9	0	1
neighbor	16	0	3	11	0	1
neighbor	16	0	3	12	0	0
neighbor	16	0	3	13	0	0
neighbor	16	0	3	14	0	0
neighbor	16	0	3	15	0	0
neighbor	16	0	3	16	0	0
neighbor	16	0	3	24	0	0
neighbor	16	0	3	73	0	0
neighbor	16	0	4	0	0	1
neighbor	16	0	4	1	0	0
neighbor	16	0	4	3	0	0
neighbor	16	0	4	7	0	0
neighbor	16	0	4	9	0	1
neighbor	16	0	4	12	0	0
neighbor	16	0	4	13	0	0
neighbor	16	0	4	14	0	0
neighbor	16	0	4	15	0	0
neighbor	16	0	4	16	0	0
neighbor	16	0	4	21	0	0
neighbor	16	0	4	24	0	0
neighbor	16	0	4	73	0	0
neighbor	16	0	5	0	0	1
neighbor	16	0	5	1	0	0
neighbor	16	0	5	3	0	0
neighbor	16	0	5	7	0	0
neighbor	16	0	5	11	0	1
neighbor	16	0	5	12	0	0
neighbor	16	0	5	13	0	0
neighbor	16	0	5	14	0	0
neighbor	16	0	5	15	0	0
neighbor	16	0	5	16	0	0
neighbor	16	0	5	21	0	0
neighbor	16	0	5	24	0	0
neighbor	16	0	5	73	0	0
neighbor	17	0	0	0	0	1
neighbor	17	0	0	2	0	0
neighbor	17	0	0	3	0	0
neighbor	17	0	0	17	0	0
neighbor	17	0	0	18	0	0
neighbor	17	0	0	18	8	0
neighbor	17	0	1	0	0	1
neighbor	17	0	1	17	0	0
neighbor	17	0	1	18	0	0
neighbor	17	0	1	18	8	0
neighbor	17	0	2	0	0	1
neighbor	17	0	2	2	0	0
neighbor	17	0	2	17	0	0
neighbor	17	0	2	18	0	0
neighbor	17	0	2	18	2	0
neighbor	17	0	2	18	8	0
neighbor	17	0	2	18	10	0
neighbor	17	0	2	31	1	1
neighbor	17	0	2	31	2	1
neighbor	17	0	2	37	0	1
neighbor	17	0	3	0	0	1
neighbor	17	0	3	2	0	0
neighbor	17	0	3	17	0	0
neighbor	17	0	3	18	0	0
neighbor	17	0	3	18	8	0
neighbor	17	0	3	31	1	1
neighbor	17	0	3	31	2	1
neighbor	17	0	4	0	0	1
neighbor	17	0	4	2	0	0
neighbor	17	0	4	17	0	0
neighbor	17	0	4	18	0	0
neighbor	17	0	4	18	8	0
neighbor	17	0	4	31	1	1
neighbor	17	0	4	31	2	1
neighbor	17	0	5	0	0	1
neighbor	17	0	5	2	0	0
neighbor	17	0	5	3	0	0
neighbor	17	0	5	17	0	0
neighbor	17	0	5	18	0	0
neighbor	17	0	5	18	8	0
neighbor	17	0	5	31	1	1
neighbor	17	0	5	31	2	1
neighbor	17	0	5	37	0	1
neighbor	17	2	0	3	0	0
neighbor	17	2	0	17	2	0
neighbor	17	2	1	17	2	0
neighbor	17	2	1	18	2	0
neighbor	17	2	1	18	10	0
neighbor	17	2	2	0	0	1
neighbor	17	2	2	2	0	0
neighbor	17	2	2	18	0	0
neighbor	17	2	2	18	2	0
neighbor	17	2	2	18	10	0
neighbor	17	2	3	0	0	1
neighbor	17	2	3	2	0	0
neighbor	17	2	3	18	2	0
neighbor	17	2	3	18	10	0
neighbor	17	2	3	37	0	1
neighbor	17	2	4	0	0	1
neighbor	17	2	4	18	0	0
neighbor	17	2	4	18	2	0
neighbor	17	2	4	18	8	0
neighbor	17	2	4	18	10	0
neighbor	17	2	4	31	1	1
neighbor	17	2	4	37	0	1
neighbor	17	2	5	0	0	1
neighbor	17	2	5	2	0	0
neighbor	17	2	5	18	0	0
neighbor	17	2	5	18	2	0
neighbor	17	2	5	18	10	0
neighbor	18	0	0	0	0	1
neighbor	18	0	0	2	0	0
neighbor	18	0	0	17	0	0
neighbor	18	0	0	18	0	0
neighbor	18	0	0	18	2	0
neighbor	18	0	0	18	8	0
neighbor	18	0	0	18	10	0
neighbor	18	0	0	31	1	1
neighbor	18	0	0	31	2	1
neighbor	18	0	0	37	0	1
neighbor	18	0	1	0	0	1
neighbor	18	0	1	17	0	0
neighbor	18	0	1	18	0	0
neighbor	18	0	1	18	2	0
neighbor	18	0	1	18	8	0
neighbor	18	0	1	18	10	0
neighbor	18	0	1	39	0	1
neighbor	18	0	2	0	0	1
neighbor	18	0	2	2	0	0
neighbor	18	0	2	17	0	0
neighbor	18	0	2	18	0	0
neighbor	18	0	2	18	2	0
neighbor	18	0	2	18	8	0
neighbor	18	0	2	18	10	0
neighbor	18	0	2	31	1	1
neighbor	18	0	3	0	0	1
neighbor	18	0	3	2	0	0
neighbor	18	0	3	17	0	0
neighbor	18	0	3	17	2	0
neighbor	18	0	3	18	0	0
neighbor	18	0	3	18	2	0
neighbor	18	0	3	18	8	0
neighbor	18	0	3	31	1	1
neighbor	18	0	3	37	0	1
neighbor	18	0	3	39	0	1
neighbor	18	0	4	0	0	1
neighbor	18	0	4	17	0	0
neighbor	18	0	4	17	2	0
neighbor	18	0	4	18	0	0
neighbor	18	0	4	18	2	0
neighbor	18	0	4	18	8	0
neighbor	18	0	4	18	10	0
neighbor	18	0	4	39	0	1
neighbor	18	0	5	0	0	1
neighbor	18	0	5	2	0	0
neighbor	18	0	5	17	0	0
neighbor	18	0	5	17	2	0
neighbor	18	0	5	18	0	0
neighbor	18	0	5	18	2	0
neighbor	18	0	5	18	8	0
neighbor	18	0	5	18	10	0
neighbor	18	0	5	31	1	1
neighbor	18	0	5	39	0	1
neighbor	18	2	0	0	0	1
neighbor	18	2	0	2	0	0
neighbor	18	2	0	17	2	0
neighbor	18	2	0	18	0	0
neighbor	18	2	0	18	2	0
neighbor	18	2	0	18	10	0
neighbor	18	2	1	0	0	1
neighbor	18	2	1	18	0	0
neighbor	18	2	1	18	2	0
neighbor	18	2	1	18	8	0
neighbor	18	2	1	18	10	0
neighbor	18	2	2	0	0	1
neighbor	18	2	2	2	0	0
neighbor	18	2	2	17	2	0
neighbor	18	2	2	18	0	0
neighbor	18	2	2	18	2	0
neighbor	18	2	2	18	8	0
neighbor	18	2	2	18	10	0
neighbor	18	2	3	0	0	1
neighbor	18	2	3	17	0	0
neighbor	18	2	3	17	2	0
neighbor	18	2	3	18	0	0
neighbor	18	2	3	18	2	0
neighbor	18	2	3	18	8	0
neighbor	18	2	3	18	10	0
neighbor	18	2	4	0	0	1
neighbor	18	2	4	17	2	0
neighbor	18	2	4	18	0	0
neighbor	18	2	4	18	2	0
neighbor	18	2	4	18	8	0
neighbor	18	2	4	18	10	0
neighbor	18	2	5	0	0	1
neighbor	18	2	5	2	0	0
neighbor	18	2	5	17	2	0
neighbor	18	2	5	18	0	0
neighbor	18	2	5	18	2	0
neighbor	18	2	5	18	8	0
neighbor	18	2	5	18	10	0
neighbor	18	8	0	0	0	1
neighbor	18	8	0	2	0	0
neighbor	18	8	0	17	0	0
neighbor	18	8	0	18	0	0
neighbor	18	8	0	18	2	0
neighbor	18	8	0	18	8	0
neighbor	18	8	0	31	1	1
neighbor	18	8	1	0	0	1
neighbor	18	8	1	17	0	0
neighbor	18	8	1	18	0	0
neighbor	18	8	1	18	8	0
neighbor	18	8	1	18	10	0
neighbor	18	8	2	0	0	1
neighbor	18	8	2	17	0	0
neighbor	18	8	2	18	0	0
neighbor	18	8	2	18	2	0
neighbor	18	8	2	18	8	0
neighbor	18	8	2	18	10	0
neighbor	18	8	3	0	0	1
neighbor	18	8	3	17	0	0
neighbor	18	8	3	18	0	0
neighbor	18	8	3	18	2	0
neighbor	18	8	3	18	8	0
neighbor	18	8	4	0	0	1
neighbor	18	8	4	17	0	0
neighbor	18	8	4	18	0	0
neighbor	18	8	4	18	2	0
neighbor	18	8	4	18	8	0
neighbor	18	8	5	0	0	1
neighbor	18	8	5	2	0	0
neighbor	18	8	5	17	0	0
neighbor	18	8	5	17	2	0
neighbor	18	8	5	18	0	0
neighbor	18	8	5	18	2	0
neighbor	18	8	5	18	8	0
neighbor	18	10	0	0	0	1
neighbor	18	10	0	17	2	0
neighbor	18	10	0	18	0	0
neighbor	18	10	0	18	2	0
neighbor	18	10	0	18	8	0
neighbor	18	10	0	18	10	0
neighbor	18	10	1	0	0	1
neighbor	18	10	1	18	0	0
neighbor	18	10	1	18	2	0
neighbor	18	10	1	18	10	0
neighbor	18	10	2	0	0	1
neighbor	18	10	2	17	2	0
neighbor	18	10	2	18	2	0
neighbor	18	10	2	18	10	0
neighbor	18	10	3	0	0	1
neighbor	18	10	3	17	0	0
neighbor	18	10	3	17	2	0
neighbor	18	10	3	18	0	0
neighbor	18	10	3	18	2	0
neighbor	18	10	3	18	8	0
neighbor	18	10	3	18	10	0
neighbor	18	10	4	0	0	1
neighbor	18	10	4	17	2	0
neighbor	18	10	4	18	0	0
neighbor	18	10	4	18	2	0
neighbor	18	10	4	18	10	0
neighbor	18	10	5	0	0	1
neighbor	18	10	5	17	2	0
neighbor	18	10	5	18	0	0
neighbor	18	10	5	18	2	0
neighbor	18	10	5	18	10	0
neighbor	21	0	0	0	0	1
neighbor	21	0	0	1	0	0
neighbor	21	0	0	3	0	0
neighbor	21	0	0	16	0	0
neighbor	21	0	0	21	0	0
neighbor	21	0	1	1	0	0
neighbor	21	0	1	3	0	0
neighbor	21	0	1	15	0	0
neighbor	21	0	1	16	0	0
neighbor	21	0	1	21	0	0
neighbor	21	0	2	1	0	0
neighbor	21	0	2	15	0	0
neighbor	21	0	2	21	0	0
neighbor	21	0	3	0	0	1
neighbor	21	0	3	1	0	0
neighbor	21	0	3	21	0	0
neighbor	21	0	4	1	0	0
neighbor	21	0	4	3	0	0
neighbor	21	0	4	9	1	1
neighbor	21	0	4	9	9	1
neighbor	21	0	4	13	0	0
neighbor	21	0	4	15	0	0
neighbor	21	0	4	16	0	0
neighbor	21	0	4	21	0	0
neighbor	21	0	5	1	0	0
neighbor	21	0	5	3	0	0
neighbor	21	0	5	16	0	0
neighbor	21	0	5	21	0	0
neighbor	21	0	5	73	0	0
neighbor	24	0	0	0	0	1
neighbor	24	0	0	1	0	0
neighbor	24	0	0	3	0	0
neighbor	24	0	0	13	0	0
neighbor	24	0	0	15	0	0
neighbor	24	0	0	16	0	0
neighbor	24	0	0	24	0	0
neighbor	24	0	1	12	0	0
neighbor	24	0	1	24	0	0
neighbor	24	0	2	0	0	1
neighbor	24	0	2	1	0	0
neighbor	24	0	2	3	0	0
neighbor	24	0	2	12	0	0
neighbor	24	0	2	13	0	0
neighbor	24	0	2	15	0	0
neighbor	24	0	2	16	0	0
neighbor	24	0	2	24	0	0
neighbor	24	0	3	0	0	1
neighbor	24	0	3	1	0	0
neighbor	24	0	3	3	0	0
neighbor	24	0	3	12	0	0
neighbor	24	0	3	13	0	0
neighbor	24	0	3	15	0	0
neighbor	24	0	3	16	0	0
neighbor	24	0	3	24	0	0
neighbor	24	0	4	0	0	1
neighbor	24	0	4	1	0	0
neighbor	24	0	4	3	0	0
neighbor	24	0	4	12	0	0
neighbor	24	0	4	13	0	0
neighbor	24	0	4	15	0	0
neighbor	24	0	4	16	0	0
neighbor	24	0	4	24	0	0
neighbor	24	0	5	0	0	1
neighbor	24	0	5	1	0	0
neighbor	24	0	5	3	0	0
neighbor	24	0	5	12	0	0
neighbor	24	0	5	13	0	0
neighbor	24	0	5	15	0	0
neighbor	24	0	5	16	0	0
neighbor	24	0	5	24	0	0
neighbor	31	1	0	2	0	0
neighbor	31	1	1	0	0	1
neighbor	31	1	1	18	0	1
neighbor	31	1	1	18	8	1
neighbor	31	1	2	0	0	1
neighbor	31	1	2	2	0	1
neighbor	31	1	2	3	0	1
neighbor	31	1	2	17	0	1
neighbor	31	1	2	18	0	1
neighbor	31	1	2	31	1	1
neighbor	31	1	2	31	2	1
neighbor	31	1	3	0	0	1
neighbor	31	1	3	2	0	1
neighbor	31	1	3	3	0	1
neighbor	31	1	3	17	0	1
neighbor	31	1	3	18	0	1
neighbor	31	1	3	31	1	1
neighbor	31	1	3	31	2	1
neighbor	31	1	4	0	0	1
neighbor	31	1	4	2	0	1
neighbor	31	1	4	3	0	1
neighbor	31	1	4	17	0	1
neighbor	31	1	4	18	0	1
neighbor	31	1	4	31	1	1
neighbor	31	1	4	31	2	1
neighbor	31	1	5	0	0	1
neighbor	31	1	5	2	0	1
neighbor	31	1	5	3	0	1
neighbor	31	1	5	17	0	1
neighbor	31	1	5	17	2	1
neighbor	31	1	5	31	1	1
neighbor	31	1	5	31	2	1
neighbor	31	2	0	2	0	0
neighbor	31	2	1	0	0	1
neighbor	31	2	1	18	0	1
neighbor	31	2	2	0	0	1
neighbor	31	2	2	2	0	1
neighbor	31	2	2	17	0	1
neighbor	31	2	2	31	1	1
neighbor	31	2	2	31	2	1
neighbor	31	2	3	0	0	1
neighbor	31	2	3	2	0	1
neighbor	31	2	3	3	0	1
neighbor	31	2	3	17	0	1
neighbor	31	2	3	31	1	1
neighbor	31	2	3	31	2	1
neighbor	31	2	4	0	0	1
neighbor	31	2	4	2	0	1
neighbor	31	2	4	17	0	1
neighbor	31	2	4	31	1	1
neighbor	31	2	4	31	2	1
neighbor	31	2	5	0	0	1
neighbor	31	2	5	2	0	1
neighbor	31	2	5	17	0	1
neighbor	31	2	5	31	1	1
neighbor	31	2	5	31	2	1
neighbor	37	0	0	2	0	0
neighbor	37	0	1	0	0	1
neighbor	37	0	1	18	0	1
neighbor	37	0	2	0	0	1
neighbor	37	0	2	17	2	1
neighbor	37	0	2	18	0	1
neighbor	37	0	2	37	0	1
neighbor	37	0	3	0	0	1
neighbor	37	0	3	2	0	1
neighbor	37	0	3	17	0	1
neighbor	37	0	3	37	0	1
neighbor	37	0	4	0	0	1
neighbor	37	0	4	17	0	1
neighbor	37	0	4	37	0	1
neighbor	37	0	5	0	0	1
neighbor	37	0	5	2	0	1
neighbor	37	0	5	17	2	1
neighbor	37	0	5	37	0	1
neighbor	39	0	0	18	0	0
neighbor	39	0	1	0	0	1
neighbor	39	0	2	18	0	1
neighbor	39	0	3	0	0	1
neighbor	39	0	4	0	0	1
neighbor	39	0	4	18	0	1
neighbor	39	0	5	0	0	1
neighbor	39	0	5	18	0	1
neighbor	40	0	0	1	0	0
neighbor	40	0	1	0	0	1
neighbor	40	0	2	0	0	1
neighbor	40	0	3	0	0	1
neighbor	40	0	4	0	0	1
neighbor	40	0	5	0	0	1
neighbor	49	0	0	1	0	0
neighbor	49	0	1	9	1	1
neighbor	49	0	1	9	2	1
neighbor	49	0	1	9	3	1
neighbor	49	0	1	9	4	1
neighbor	49	0	2	11	0	1
neighbor	49	0	2	49	0	0
neighbor	49	0	3	1	0	0
neighbor	49	0	3	49	0	0
neighbor	49	0	4	1	0	0
neighbor	49	0	5	11	0	1
neighbor	56	0	0	0	0	1
neighbor	56	0	0	1	0	0
neighbor	56	0	0	3	0	0
neighbor	56	0	0	7	0	0
neighbor	56	0	0	15	0	0
neighbor	56	0	0	56	0	0
neighbor	56	0	1	0	0	1
neighbor	56	0	1	1	0	0
neighbor	56	0	1	3	0	0
neighbor	56	0	1	7	0	0
neighbor	56	0	1	9	0	1
neighbor	56	0	1	56	0	0
neighbor	56	0	1	73	0	0
neighbor	56	0	2	0	0	1
neighbor	56	0	2	1	0	0
neighbor	56	0	2	3	0	0
neighbor	56	0	2	7	0	0
neighbor	56	0	2	13	0	0
neighbor	56	0	2	56	0	0
neighbor	56	0	2	73	0	0
neighbor	56	0	3	0	0	1
neighbor	56	0	3	1	0	0
neighbor	56	0	3	3	0	0
neighbor	56	0	3	7	0	0
neighbor	56	0	3	15	0	0
neighbor	56	0	3	56	0	0
neighbor	56	0	3	73	0	0
neighbor	56	0	4	1	0	0
neighbor	56	0	4	7	0	0
neighbor	56	0	4	15	0	0
neighbor	56	0	4	56	0	0
neighbor	56	0	4	73	0	0
neighbor	56	0	5	0	0	1
neighbor	56	0	5	1	0	0
neighbor	56	0	5	3	0	0
neighbor	56	0	5	7	0	0
neighbor	56	0	5	15	0	0
neighbor	56	0	5	56	0	0
neighbor	73	0	0	0	0	1
neighbor	73	0	0	1	0	0
neighbor	73	0	0	3	0	0
neighbor	73	0	0	7	0	0
neighbor	73	0	0	13	0	0
neighbor	73	0	0	15	0	0
neighbor	73	0	0	16	0	0
neighbor	73	0	0	56	0	0
neighbor	73	0	0	73	0	0
neighbor	73	0	1	0	0	1
neighbor	73	0	1	1	0	0
neighbor	73	0	1	3	0	0
neighbor	73	0	1	7	0	0
neighbor	73	0	1	9	0	1
neighbor	73	0	1	11	0	1
neighbor	73	0	1	13	0	0
neighbor	73	0	1	15	0	0
neighbor	73	0	1	16	0	0
neighbor	73	0	1	73	0	0
neighbor	73	0	2	0	0	1
neighbor	73	0	2	1	0	0
neighbor	73	0	2	3	0	0
neighbor	73	0	2	7	0	0
neighbor	73	0	2	11	0	1
neighbor	73	0	2	13	0	0
neighbor	73	0	2	15	0	0
neighbor	73	0	2	16	0	0
neighbor	73	0	2	56	0	0
neighbor	73	0	2	73	0	0
neighbor	73	0	3	0	0	1
neighbor	73	0	3	1	0	0
neighbor	73	0	3	3	0	0
neighbor	73	0	3	7	0	0
neighbor	73	0	3	11	0	1
neighbor	73	0	3	13	0	0
neighbor	73	0	3	15	0	0
neighbor	73	0	3	16	0	0
neighbor	73	0	3	56	0	0
neighbor	73	0	3	73	0	0
neighbor	73	0	4	0	0	1
neighbor	73	0	4	1	0	0
neighbor	73	0	4	3	0	0
neighbor	73	0	4	7	0	0
neighbor	73	0	4	13	0	0
neighbor	73	0	4	15	0	0
neighbor	73	0	4	16	0	0
neighbor	73	0	4	21	0	0
neighbor	73	0	4	73	0	0
neighbor	73	0	5	0	0	1
neighbor	73	0	5	1	0	0
neighbor	73	0	5	3	0	0
neighbor	73	0	5	7	0	0
neighbor	73	0	5	11	0	1
neighbor	73	0	5	13	0	0
neighbor	73	0	5	15	0	0
neighbor	73	0	5	16	0	0
neighbor	73	0	5	56	0	0
neighbor	73	0	5	73	0	0
neighbor	3	0	2	9	6	1
neighbor	9	6	3	3	0	0
"#;

#[cfg(test)]
mod tests {
    use super::*;
    fn make_preview_copy() -> (PathBuf, Vec<PreviewSourcePin>) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "view-preview-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".world-input-copy"), WORLD_MARKER).unwrap();
        fs::write(dir.join(".region-input-copy"), REGION_MARKER).unwrap();
        let pins: Vec<_> = FILES
            .iter()
            .map(|(name, _, _)| {
                let bytes = format!("preview bytes for {name}").into_bytes();
                fs::write(dir.join(name), &bytes).unwrap();
                PreviewSourcePin {
                    name: (*name).to_owned(),
                    size: bytes.len(),
                    sha256: sha256_hex(&bytes),
                }
            })
            .collect();
        let manifest = pins
            .iter()
            .map(|pin| format!("{}\t{}\t{}\n", pin.name, pin.size, pin.sha256))
            .collect::<String>();
        fs::write(dir.join(".preview-source-hashes.tsv"), manifest).unwrap();
        (dir, pins)
    }

    fn make_selected_copy() -> (PathBuf, Vec<PreviewSourcePin>) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "view-selected-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".world-input-copy"), WORLD_MARKER).unwrap();
        fs::write(dir.join(".region-input-copy"), REGION_MARKER).unwrap();
        let pins = ["level.dat", "r.12.-3.mcr", "r.13.-3.mcr"]
            .into_iter()
            .map(|name| {
                let bytes = format!("selected snapshot bytes for {name}").into_bytes();
                fs::write(dir.join(name), &bytes).unwrap();
                PreviewSourcePin {
                    name: name.to_owned(),
                    size: bytes.len(),
                    sha256: sha256_hex(&bytes),
                }
            })
            .collect::<Vec<_>>();
        let manifest = pins
            .iter()
            .map(|pin| format!("{}\t{}\t{}\n", pin.name, pin.size, pin.sha256))
            .collect::<String>();
        fs::write(dir.join(".preview-source-hashes.tsv"), manifest).unwrap();
        (dir, pins)
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn viewer_light_dimension_accepts_only_overworld_and_nether() {
        assert_eq!(
            light_dimension(0).unwrap(),
            beta_core::light::LightDimension::Overworld
        );
        assert_eq!(
            light_dimension(-1).unwrap(),
            beta_core::light::LightDimension::Nether
        );
        let error = light_dimension(1).unwrap_err().to_string();
        assert!(error.contains("does not support saved dimension 1"));
        assert!(error.contains("0 and -1"));
    }
    #[test]
    fn scene_coordinates_require_complete_fixed_rectangle() {
        let p = std::env::temp_dir().join(format!("view-scene-{}.tsv", std::process::id()));
        let good = (-3..=1)
            .flat_map(|z| (-3..=1).map(move |x| format!("{x}\t{z}\n")))
            .collect::<String>();
        fs::write(&p, good).unwrap();
        assert!(parse_scene(&p, [-9, 64, -5]).is_ok());
        fs::write(&p, "0\t0\n").unwrap();
        assert!(parse_scene(&p, [-9, 64, -5]).is_err());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn selected_spawn_window_uses_floor_division_and_checked_edges() {
        let scene = parse_spawn_window([-1, 64, -17]).unwrap();
        assert_eq!((scene.min_x, scene.min_z), (-3, -4));
        assert_eq!(scene.chunks.len(), 25);
        assert!(scene.chunks.contains(&(-3, -4)));
        assert!(scene.chunks.contains(&(-3, 0)));
        assert!(scene.chunks.contains(&(1, 0)));
        let edge = parse_spawn_window([i32::MIN, 64, i32::MAX]).unwrap();
        assert_eq!((edge.min_x, edge.min_z), (-134_217_730, 134_217_725));
        let regions = required_region_coords(&scene);
        assert_eq!(
            regions,
            BTreeSet::from([(-1, -1), (-1, 0), (0, -1), (0, 0)])
        );
        let positive = required_region_coords(&parse_spawn_window([600, 64, 600]).unwrap());
        assert_eq!(positive, BTreeSet::from([(1, 1)]));
    }

    #[test]
    fn selected_world_copy_accepts_dynamic_regions_and_rejects_unsafe_names() {
        assert!(valid_region_filename("r.12.-3.mcr"));
        assert!(valid_region_filename("r.-2147483648.2147483647.mcr"));
        for invalid in [
            "r.1.mcr",
            "r.1.2.mcr/../level.dat",
            "../r.1.2.mcr",
            "r.x.2.mcr",
            "r.1.2.mca",
        ] {
            assert!(!valid_region_filename(invalid), "{invalid}");
        }
        let (dir, pins) = make_selected_copy();
        let files = validate_selected_world_copy(&dir, &pins).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.contains_key("r.12.-3.mcr"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn selected_world_copy_rejects_manifest_mismatch_and_unexpected_entries() {
        let (dir, pins) = make_selected_copy();
        fs::write(dir.join("extra.mcr"), b"unexpected").unwrap();
        assert!(validate_selected_world_copy(&dir, &pins).is_err());
        let _ = fs::remove_file(dir.join("extra.mcr"));
        fs::write(dir.join(".preview-source-hashes.tsv"), "changed\n").unwrap();
        assert!(validate_selected_world_copy(&dir, &pins).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn marker_or_copy_without_all_members_fails() {
        let p = std::env::temp_dir().join(format!("view-copy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir(&p).unwrap();
        assert!(validate_copy(&p).is_err());
        let _ = fs::remove_dir_all(p);
    }

    #[test]
    fn preview_copy_accepts_only_exact_machine_local_pins() {
        let (dir, pins) = make_preview_copy();
        let files = validate_preview_copy(&dir, &pins).unwrap();
        assert_eq!(files.len(), FILES.len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_copy_rejects_mutated_hash_manifest_and_unexpected_entries() {
        let (dir, mut pins) = make_preview_copy();
        let replacement = if pins[0].sha256.starts_with('0') {
            "1"
        } else {
            "0"
        };
        pins[0].sha256.replace_range(..1, replacement);
        assert!(validate_preview_copy(&dir, &pins).is_err());
        let _ = fs::remove_file(dir.join(".preview-source-hashes.tsv"));
        fs::write(dir.join(".preview-source-hashes.tsv"), "bad\n").unwrap();
        fs::write(dir.join("unexpected"), "x").unwrap();
        assert!(validate_preview_copy(&dir, &pins).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn native_registry_has_unique_states_and_complete_neighbor_schema() {
        let (states, neighbors) = native_registry();
        assert_eq!(states.len(), 50);
        assert_eq!(neighbors.len(), 1619);
        assert_eq!(
            mapping_for_state(State {
                id: 2,
                meta: 0,
                above: 17
            })
            .unwrap()
            .tiles,
            [2, 0, 3, 3, 3, 3]
        );
        assert_eq!(
            visibility(Neighbor {
                id: 2,
                meta: 0,
                face: 2,
                other: 0,
                other_meta: 0
            })
            .unwrap(),
            1
        );
        assert_eq!(
            visibility(Neighbor {
                id: 3,
                meta: 0,
                face: 2,
                other: 9,
                other_meta: 6,
            })
            .unwrap(),
            1
        );
        assert_eq!(
            visibility(Neighbor {
                id: 9,
                meta: 6,
                face: 3,
                other: 3,
                other_meta: 0,
            })
            .unwrap(),
            0
        );
        assert!(
            visibility(Neighbor {
                id: 255,
                meta: 0,
                face: 0,
                other: 0,
                other_meta: 0
            })
            .is_err()
        );
    }

    #[test]
    fn viewer_extra_cubes_cull_shared_faces_without_expanding_strict_registry() {
        let mappings = mapping_registry();
        let fixture = include_str!("../../../tests/fixtures/view/block-cubes-meta0.tsv");
        let rows: Vec<_> = fixture
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(rows.len(), 5);
        for row in rows {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 9);
            let id = fields[0].parse::<u8>().unwrap();
            let key = BlockStateKey {
                id,
                metadata: 0,
                above_id: None,
            };
            let mapping = mappings.iter().find(|mapping| mapping.key == key).unwrap();
            assert!(mapping.is_renderable_cube());
            assert_eq!(mapping.render_type.to_string(), fields[1]);
            assert_eq!(u8::from(mapping.opaque).to_string(), fields[2]);
            assert_eq!(fields[3], "0.0,0.0,0.0,1.0,1.0,1.0");
            assert_eq!(
                mapping.tiles.map(|tile| tile.to_string()).join(","),
                fields[4]
            );
            assert_eq!(fields[5], "111111");
            assert_eq!(fields[6], "000000");
            assert_eq!(fields[7], "000000");
            assert_eq!(fields[8], "000000,000000,000000,000000,000000");
            assert!(
                mapping_for_state(State {
                    id,
                    meta: 0,
                    above: -1
                })
                .is_none()
            );
            let mut data = beta_core::chunk::ChunkData::new();
            for (x, block_id) in [(1, 1), (2, id)] {
                data.set(
                    x,
                    1,
                    1,
                    BlockData {
                        id: block_id,
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            let scene = Scene::new(
                SceneBounds::new(0, 0, 1, 1).unwrap(),
                vec![SceneChunk { x: 0, z: 0, data }],
                mappings.clone(),
            )
            .unwrap();
            let mesh = scene.mesh(MeshLimits::default()).unwrap();
            assert_eq!(mesh.unsupported_blocks, 0);
            assert_eq!(mesh.visible_faces, 10, "stone adjacent to {id}");
        }
    }

    #[test]
    fn scene_observation_is_deterministic_and_keeps_grass_context() {
        let mut data = beta_core::chunk::ChunkData::new();
        data.set(
            1,
            1,
            1,
            BlockData {
                id: 2,
                metadata: 0,
                ..Default::default()
            },
        )
        .unwrap();
        data.set(
            1,
            2,
            1,
            BlockData {
                id: 17,
                metadata: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let mut chunks = Vec::new();
        for z in -2..=2 {
            for x in -2..=2 {
                chunks.push(SceneChunk {
                    x,
                    z,
                    data: if x == 0 && z == 0 {
                        data.clone()
                    } else {
                        beta_core::chunk::ChunkData::new()
                    },
                });
            }
        }
        let first = observed_rows(&chunks, -2, -2).unwrap();
        let second = observed_rows(&chunks, -2, -2).unwrap();
        assert_eq!(first, second);
        assert!(first.0.contains(&State {
            id: 2,
            meta: 0,
            above: 17
        }));
        assert_eq!(
            visibility(Neighbor {
                id: 2,
                meta: 0,
                face: 2,
                other: 0,
                other_meta: 0
            })
            .unwrap(),
            1
        );
    }
}
