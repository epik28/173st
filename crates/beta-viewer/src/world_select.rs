//! Read-only catalog and snapshot support for existing Beta saves.

use beta_core::{
    region::{RegionFile, RegionLimits, local_coord, region_coord},
    world::{WorldLimits, load_chunk_nbt, load_level_gzip},
};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_SAVES: usize = 256;
const MAX_FILE: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePin {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntry {
    pub folder: String,
    pub display_name: String,
    pub last_played: i64,
    pub spawn: [i32; 3],
    pub regions: Vec<String>,
    pub launch_reason: Option<String>,
    pub pins: Vec<SourcePin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooserAction {
    Select,
    Activate,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooserState {
    pub selected: Option<usize>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChooserGeometry {
    pub width: f32,
    pub row_top: f32,
    pub row_height: f32,
    pub select_top: f32,
    pub cancel_top: f32,
    pub button_height: f32,
}

impl ChooserGeometry {
    pub fn new(width: f32) -> Self {
        Self {
            width,
            row_top: 56.0,
            row_height: 42.0,
            select_top: 410.0,
            cancel_top: 410.0,
            button_height: 36.0,
        }
    }
    pub fn row_at(self, y: f32, count: usize) -> Option<usize> {
        if y < self.row_top {
            return None;
        }
        let index = ((y - self.row_top) / self.row_height).floor() as usize;
        (index < count).then_some(index)
    }
    pub fn select_at(self, x: f32, y: f32) -> bool {
        (420.0..570.0).contains(&x)
            && (self.select_top..self.select_top + self.button_height).contains(&y)
    }
    pub fn cancel_at(self, x: f32, y: f32) -> bool {
        (580.0..self.width - 20.0).contains(&x)
            && (self.cancel_top..self.cancel_top + self.button_height).contains(&y)
    }
}

impl ChooserState {
    pub fn new() -> Self {
        Self {
            selected: None,
            cancelled: false,
        }
    }
    pub fn apply(
        &mut self,
        action: ChooserAction,
        row: Option<usize>,
        count: usize,
    ) -> Option<usize> {
        if action == ChooserAction::Cancel {
            self.cancelled = true;
            return None;
        }
        let index = row.filter(|i| *i < count)?;
        self.selected = Some(index);
        if action == ChooserAction::Activate {
            Some(index)
        } else {
            None
        }
    }

    pub fn move_selection(&mut self, entries: &[WorldEntry], delta: i32) {
        if entries.is_empty() {
            self.selected = None;
            return;
        }
        if delta == 0 {
            return;
        }
        let start = self.selected.map_or_else(
            || if delta > 0 { -1 } else { 0 },
            |selected| selected as i32,
        );
        let mut index = start;
        for _ in 0..entries.len() {
            index = (index + delta).rem_euclid(entries.len() as i32);
            if entries[index as usize].launch_reason.is_none() {
                self.selected = Some(index as usize);
                return;
            }
        }
    }

    pub fn activate(&mut self, entries: &[WorldEntry]) -> Option<usize> {
        let index = self.selected?;
        (index < entries.len() && entries[index].launch_reason.is_none()).then_some(index)
    }
}

impl Default for ChooserState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum SelectError {
    Io(io::Error),
    Invalid(String),
}
impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Invalid(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for SelectError {}
impl From<io::Error> for SelectError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn pin(path: &Path, name: String) -> Result<SourcePin, SelectError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(SelectError::Invalid(format!(
            "{name} is not a regular file"
        )));
    }
    if meta.len() == 0 || meta.len() > MAX_FILE {
        return Err(SelectError::Invalid(format!("{name} has invalid size")));
    }
    let bytes = fs::read(path)?;
    let second = fs::symlink_metadata(path)?;
    if second.len() != meta.len() || second.modified().ok() != meta.modified().ok() {
        return Err(SelectError::Invalid(format!(
            "{name} changed during discovery"
        )));
    }
    Ok(SourcePin {
        name,
        size: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

fn required_chunks(spawn: [i32; 3]) -> Vec<(i32, i32)> {
    let cx = spawn[0].div_euclid(16);
    let cz = spawn[2].div_euclid(16);
    (-2..=2)
        .flat_map(|z| (-2..=2).map(move |x| (cx + x, cz + z)))
        .collect()
}

fn valid_pin_name(name: &str) -> bool {
    if name == "level.dat" {
        return true;
    }
    let Some(coords) = name
        .strip_prefix("r.")
        .and_then(|name| name.strip_suffix(".mcr"))
    else {
        return false;
    };
    let Some((x, z)) = coords.split_once('.') else {
        return false;
    };
    let (Ok(x), Ok(z)) = (x.parse::<i32>(), z.parse::<i32>()) else {
        return false;
    };
    name == format!("r.{x}.{z}.mcr")
}

fn invalid_entry(
    folder: String,
    display_name: &str,
    reason: String,
    pins: Vec<SourcePin>,
) -> WorldEntry {
    WorldEntry {
        folder,
        display_name: display_name.into(),
        last_played: 0,
        spawn: [0; 3],
        regions: vec![],
        launch_reason: Some(reason),
        pins,
    }
}

pub fn discover(root: &Path) -> Result<Vec<WorldEntry>, SelectError> {
    let initial = fs::symlink_metadata(root)?;
    if initial.file_type().is_symlink() || !initial.is_dir() {
        return Err(SelectError::Invalid("saves root is unsafe".into()));
    }
    let root = fs::canonicalize(root)?;
    let meta = fs::symlink_metadata(&root)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(SelectError::Invalid("saves root is unsafe".into()));
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&root)?.take(MAX_SAVES) {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() || !ty.is_dir() {
            continue;
        }
        let folder = entry
            .file_name()
            .into_string()
            .map_err(|_| SelectError::Invalid("non-UTF8 save folder".into()))?;
        let dir = entry.path();
        let level = dir.join("level.dat");
        let level_pin = match pin(&level, "level.dat".into()) {
            Ok(p) => p,
            Err(e) => {
                out.push(WorldEntry {
                    folder,
                    display_name: "Unreadable world".into(),
                    last_played: 0,
                    spawn: [0; 3],
                    regions: vec![],
                    launch_reason: Some(e.to_string()),
                    pins: vec![],
                });
                continue;
            }
        };
        let bytes = match fs::read(&level) {
            Ok(bytes) => bytes,
            Err(error) => {
                out.push(invalid_entry(
                    folder,
                    "Unreadable world",
                    error.to_string(),
                    vec![],
                ));
                continue;
            }
        };
        if format!("{:x}", Sha256::digest(&bytes)) != level_pin.sha256 {
            out.push(invalid_entry(
                folder,
                "Changed world",
                "level.dat changed during discovery".into(),
                vec![level_pin],
            ));
            continue;
        }
        let metadata = match load_level_gzip(&bytes, WorldLimits::default()) {
            Ok(m) => m,
            Err(e) => {
                out.push(WorldEntry {
                    folder,
                    display_name: "Malformed world".into(),
                    last_played: 0,
                    spawn: [0; 3],
                    regions: vec![],
                    launch_reason: Some(e.to_string()),
                    pins: vec![level_pin],
                });
                continue;
            }
        };
        let mut regions: Vec<String> = required_chunks(metadata.spawn)
            .into_iter()
            .map(|(x, z)| format!("r.{}.{}.mcr", region_coord(x), region_coord(z)))
            .collect();
        regions.sort();
        regions.dedup();
        let mut pins = vec![level_pin];
        let region_root = dir.join("region");
        let mut reason = match fs::symlink_metadata(&region_root) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                Some("region directory is unsafe".to_owned())
            }
            Ok(_) => None,
            Err(error) => Some(format!("region directory: {error}")),
        };
        if reason.is_none() {
            for name in &regions {
                let p = region_root.join(name);
                match pin(&p, name.clone()) {
                    Ok(v) => match fs::read(&p) {
                        Ok(data) if format!("{:x}", Sha256::digest(&data)) == v.sha256 => {
                            match RegionFile::open(&data, RegionLimits::default()) {
                                Ok(r) => {
                                    for (x, z) in required_chunks(metadata.spawn) {
                                        if format!("r.{}.{}.mcr", region_coord(x), region_coord(z))
                                            == *name
                                            && r.read_local(local_coord(x), local_coord(z))
                                                .ok()
                                                .flatten()
                                                .and_then(|c| {
                                                    load_chunk_nbt(
                                                        c.nbt,
                                                        (x, z),
                                                        beta_core::nbt::Limits::default(),
                                                    )
                                                    .ok()
                                                })
                                                .is_none()
                                        {
                                            reason =
                                                Some(format!("missing or invalid chunk ({x},{z})"));
                                        }
                                    }
                                }
                                Err(_) => reason = Some(format!("invalid region {name}")),
                            }
                            pins.push(v);
                        }
                        Ok(_) => reason = Some(format!("region {name} changed during discovery")),
                        Err(error) => reason = Some(format!("region {name}: {error}")),
                    },
                    Err(e) => reason = Some(e.to_string()),
                }
                if reason.is_some() {
                    break;
                }
            }
        }
        out.push(WorldEntry {
            folder,
            display_name: metadata.name,
            last_played: metadata.last_played,
            spawn: metadata.spawn,
            regions,
            launch_reason: reason,
            pins,
        });
    }
    out.sort_by(|a, b| {
        b.last_played
            .cmp(&a.last_played)
            .then_with(|| a.folder.cmp(&b.folder))
    });
    Ok(out)
}

pub fn snapshot(
    entry: &WorldEntry,
    saves_root: &Path,
    destination_root: &Path,
) -> Result<PathBuf, SelectError> {
    if entry.launch_reason.is_some() {
        return Err(SelectError::Invalid("world is not launchable".into()));
    }
    let saves_meta = fs::symlink_metadata(saves_root)?;
    if saves_meta.file_type().is_symlink() || !saves_meta.is_dir() {
        return Err(SelectError::Invalid("saves root is unsafe".into()));
    }
    let root = fs::canonicalize(saves_root)?;
    let folder_path = Path::new(&entry.folder);
    if folder_path.components().count() != 1
        || !matches!(
            folder_path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(SelectError::Invalid("save folder name is unsafe".into()));
    }
    let source_raw = root.join(&entry.folder);
    let source_meta = fs::symlink_metadata(&source_raw)?;
    if source_meta.file_type().is_symlink() || !source_meta.is_dir() {
        return Err(SelectError::Invalid("save directory is unsafe".into()));
    }
    let region_meta = fs::symlink_metadata(source_raw.join("region"))?;
    if region_meta.file_type().is_symlink() || !region_meta.is_dir() {
        return Err(SelectError::Invalid("region directory is unsafe".into()));
    }
    let source = fs::canonicalize(source_raw)?;
    if !source.starts_with(&root) {
        return Err(SelectError::Invalid("save escapes saves root".into()));
    }
    let mut names = std::collections::HashSet::new();
    for pin in &entry.pins {
        if !valid_pin_name(&pin.name) {
            return Err(SelectError::Invalid(format!(
                "unsafe snapshot pin name: {}",
                pin.name
            )));
        }
        if !names.insert(pin.name.as_str()) {
            return Err(SelectError::Invalid(format!(
                "duplicate snapshot pin: {}",
                pin.name
            )));
        }
    }
    if !names.contains("level.dat") {
        return Err(SelectError::Invalid(
            "snapshot pins must include level.dat".into(),
        ));
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SelectError::Invalid("clock error".into()))?
        .as_nanos();
    let out = destination_root.join(format!("world-select-{stamp}"));
    fs::create_dir_all(&out)?;
    for p in &entry.pins {
        let src = if p.name == "level.dat" {
            source.join(&p.name)
        } else {
            source.join("region").join(&p.name)
        };
        let before = pin(&src, p.name.clone())?;
        if before != *p {
            return Err(SelectError::Invalid(format!(
                "source changed before copy: {}",
                p.name
            )));
        }
        let bytes = fs::read(&src)?;
        let dst = out.join(&p.name);
        fs::write(&dst, &bytes)?;
        let after = pin(&src, p.name.clone())?;
        if after != *p {
            return Err(SelectError::Invalid(format!(
                "source changed after copy: {}",
                p.name
            )));
        }
        let copied = pin(&dst, p.name.clone())?;
        if copied != *p {
            return Err(SelectError::Invalid(format!(
                "snapshot hash mismatch: {}",
                p.name
            )));
        }
    }
    fs::write(
        out.join(".world-input-copy"),
        "WORLD-LOAD-001 copied inputs\n",
    )?;
    fs::write(out.join(".region-input-copy"), "REGION-001 copied inputs\n")?;
    let manifest = entry
        .pins
        .iter()
        .map(|p| format!("{}\t{}\t{}\n", p.name, p.size, p.sha256))
        .collect::<String>();
    fs::write(out.join(".preview-source-hashes.tsv"), manifest)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_core::nbt::{Limits as NbtLimits, NamedTag, Tag, write_named};
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_dir(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "world-select-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_pin(name: &str, bytes: &[u8]) -> SourcePin {
        SourcePin {
            name: name.into(),
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn write_level(path: &Path, name: &str, last_played: i64) {
        let raw = write_named(
            &NamedTag {
                name: String::new(),
                value: Tag::Compound(vec![(
                    "Data".into(),
                    Tag::Compound(vec![
                        ("LevelName".into(), Tag::String(name.into())),
                        ("LastPlayed".into(), Tag::Long(last_played)),
                        ("SpawnX".into(), Tag::Int(0)),
                        ("SpawnY".into(), Tag::Int(64)),
                        ("SpawnZ".into(), Tag::Int(0)),
                    ]),
                )]),
            },
            NbtLimits::default(),
        )
        .unwrap();
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        fs::write(path, encoder.finish().unwrap()).unwrap();
    }
    #[test]
    fn negative_chunk_window_uses_floor_division() {
        assert_eq!(required_chunks([-1, 64, -17])[0], (-3, -4));
    }
    #[test]
    fn chooser_actions() {
        let mut s = ChooserState::new();
        assert_eq!(s.apply(ChooserAction::Select, Some(1), 2), None);
        assert_eq!(s.apply(ChooserAction::Activate, Some(1), 2), Some(1));
        assert_eq!(s.apply(ChooserAction::Cancel, None, 2), None);
        assert!(s.cancelled);
    }

    #[test]
    fn chooser_skips_disabled_rows_and_requires_explicit_activation() {
        let entries = vec![
            WorldEntry {
                folder: "bad".into(),
                display_name: "Bad".into(),
                last_played: 0,
                spawn: [0; 3],
                regions: vec![],
                launch_reason: Some("missing chunk".into()),
                pins: vec![],
            },
            WorldEntry {
                folder: "good".into(),
                display_name: "Good".into(),
                last_played: 0,
                spawn: [0; 3],
                regions: vec![],
                launch_reason: None,
                pins: vec![],
            },
        ];
        let mut state = ChooserState::default();
        assert_eq!(state.selected, None);
        assert_eq!(state.activate(&entries), None);
        state.move_selection(&entries, 1);
        assert_eq!(state.selected, Some(1));
        assert_eq!(state.activate(&entries), Some(1));
    }

    #[test]
    fn chooser_geometry_maps_rows_and_buttons() {
        let g = ChooserGeometry::new(720.0);
        assert_eq!(g.row_at(56.0, 3), Some(0));
        assert_eq!(g.row_at(97.9, 3), Some(0));
        assert_eq!(g.row_at(98.0, 3), Some(1));
        assert!(g.select_at(450.0, 420.0));
        assert!(g.cancel_at(650.0, 420.0));
        assert_eq!(g.row_at(300.0, 2), None);
    }

    #[test]
    fn discovery_retains_invalid_saves_and_sorts_ties_by_folder() {
        let root = temp_dir("sort");
        for folder in ["zeta", "alpha", "middle"] {
            let save = root.join(folder);
            fs::create_dir(&save).unwrap();
            fs::write(save.join("level.dat"), b"not gzip").unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(root.join("alpha"), root.join("linked-save")).unwrap();
            symlink(
                root.join("missing-level.dat"),
                root.join("middle/level.dat.link"),
            )
            .unwrap();
        }

        let entries = discover(&root).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.folder.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "middle", "zeta"]
        );
        assert!(entries.iter().all(|entry| entry.launch_reason.is_some()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discovery_sorts_valid_compressed_metadata_by_last_played() {
        let root = temp_dir("metadata-sort");
        for (folder, played) in [("old", 10), ("newer", 30), ("middle", 20)] {
            let save = root.join(folder);
            fs::create_dir(&save).unwrap();
            write_level(&save.join("level.dat"), folder, played);
        }

        let entries = discover(&root).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.folder.as_str())
                .collect::<Vec<_>>(),
            ["newer", "middle", "old"]
        );
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.last_played)
                .collect::<Vec<_>>(),
            [30, 20, 10]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn discovery_rejects_symlinked_region_root_without_reading_through_it() {
        use std::os::unix::fs::symlink;
        let base = temp_dir("region-symlink");
        let root = base.join("saves");
        let outside = base.join("outside");
        let world = root.join("World");
        fs::create_dir_all(&world).unwrap();
        fs::create_dir_all(&outside).unwrap();
        write_level(&world.join("level.dat"), "World", 1);
        fs::write(outside.join("r.-1.-1.mcr"), b"outside region data").unwrap();
        symlink(&outside, world.join("region")).unwrap();

        let entries = discover(&root).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0]
                .launch_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("region directory is unsafe"))
        );
        assert_eq!(entries[0].pins.len(), 1);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn snapshot_rejects_changed_sources_and_never_writes_them() {
        let base = temp_dir("changed");
        let saves = base.join("saves");
        let destination = base.join("artifacts");
        let world = saves.join("World");
        fs::create_dir_all(world.join("region")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let original = b"original level bytes";
        fs::write(world.join("level.dat"), original).unwrap();
        let entry = WorldEntry {
            folder: "World".into(),
            display_name: "World".into(),
            last_played: 0,
            spawn: [0; 3],
            regions: vec![],
            launch_reason: None,
            pins: vec![test_pin("level.dat", original)],
        };
        fs::write(world.join("level.dat"), b"changed level bytes").unwrap();
        let changed = fs::read(world.join("level.dat")).unwrap();

        let error = snapshot(&entry, &saves, &destination).unwrap_err();
        assert!(error.to_string().contains("source changed before copy"));
        assert_eq!(fs::read(world.join("level.dat")).unwrap(), changed);
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn snapshot_copies_to_disposable_output_without_changing_source() {
        let base = temp_dir("copy");
        let saves = base.join("saves");
        let destination = base.join("artifacts");
        let world = saves.join("World");
        fs::create_dir_all(world.join("region")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let original = b"pinned level bytes";
        fs::write(world.join("level.dat"), original).unwrap();
        let entry = WorldEntry {
            folder: "World".into(),
            display_name: "World".into(),
            last_played: 0,
            spawn: [0; 3],
            regions: vec![],
            launch_reason: None,
            pins: vec![test_pin("level.dat", original)],
        };

        let snapshot = snapshot(&entry, &saves, &destination).unwrap();
        assert_eq!(fs::read(snapshot.join("level.dat")).unwrap(), original);
        assert!(snapshot.join(".preview-source-hashes.tsv").is_file());
        assert_eq!(fs::read(world.join("level.dat")).unwrap(), original);
        assert_eq!(fs::read_dir(&world).unwrap().count(), 2);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn snapshot_rejects_traversal_and_symlinked_world_directories() {
        let base = temp_dir("unsafe");
        let saves = base.join("saves");
        let destination = base.join("artifacts");
        fs::create_dir_all(&saves).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let traversal = WorldEntry {
            folder: "../outside".into(),
            display_name: "outside".into(),
            last_played: 0,
            spawn: [0; 3],
            regions: vec![],
            launch_reason: None,
            pins: vec![],
        };
        assert!(
            snapshot(&traversal, &saves, &destination)
                .unwrap_err()
                .to_string()
                .contains("folder name is unsafe")
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = base.join("outside");
            fs::create_dir(&outside).unwrap();
            symlink(&outside, saves.join("Link")).unwrap();
            let linked = WorldEntry {
                folder: "Link".into(),
                display_name: "Link".into(),
                last_played: 0,
                spawn: [0; 3],
                regions: vec![],
                launch_reason: None,
                pins: vec![],
            };
            assert!(
                snapshot(&linked, &saves, &destination)
                    .unwrap_err()
                    .to_string()
                    .contains("save directory is unsafe")
            );
        }
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn snapshot_rejects_unsafe_or_duplicate_pin_names_before_creating_output() {
        let base = temp_dir("pin-names");
        let saves = base.join("saves");
        let destination = base.join("artifacts");
        let world = saves.join("World");
        fs::create_dir_all(world.join("region")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let level = b"level";
        fs::write(world.join("level.dat"), level).unwrap();
        for unsafe_name in ["../escape", "/tmp/escape", "r.01.2.mcr", "r.1.2.mca"] {
            let entry = WorldEntry {
                folder: "World".into(),
                display_name: "World".into(),
                last_played: 0,
                spawn: [0; 3],
                regions: vec![],
                launch_reason: None,
                pins: vec![test_pin("level.dat", level), test_pin(unsafe_name, b"x")],
            };
            assert!(
                snapshot(&entry, &saves, &destination)
                    .unwrap_err()
                    .to_string()
                    .contains("unsafe snapshot pin name")
            );
        }
        let duplicate = WorldEntry {
            folder: "World".into(),
            display_name: "World".into(),
            last_played: 0,
            spawn: [0; 3],
            regions: vec![],
            launch_reason: None,
            pins: vec![test_pin("level.dat", level), test_pin("level.dat", level)],
        };
        assert!(
            snapshot(&duplicate, &saves, &destination)
                .unwrap_err()
                .to_string()
                .contains("duplicate snapshot pin")
        );
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        fs::remove_dir_all(base).unwrap();
    }
}
