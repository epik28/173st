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

pub fn discover(root: &Path) -> Result<Vec<WorldEntry>, SelectError> {
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
        let bytes = fs::read(&level)?;
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
        let mut reason = None;
        for name in &regions {
            let p = dir.join("region").join(name);
            match pin(&p, name.clone()) {
                Ok(v) => {
                    if let Ok(data) = fs::read(&p) {
                        if let Ok(r) = RegionFile::open(&data, RegionLimits::default()) {
                            for (x, z) in required_chunks(metadata.spawn) {
                                if format!("r.{}.{}.mcr", region_coord(x), region_coord(z)) == *name
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
                                    reason = Some(format!("missing or invalid chunk ({x},{z})"));
                                }
                            }
                        } else {
                            reason = Some(format!("invalid region {name}"));
                        }
                    }
                    pins.push(v);
                }
                Err(e) => reason = Some(e.to_string()),
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
    let source = fs::canonicalize(saves_root)?.join(&entry.folder);
    let root = fs::canonicalize(saves_root)?;
    if !source.starts_with(&root) {
        return Err(SelectError::Invalid("save escapes saves root".into()));
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
}
