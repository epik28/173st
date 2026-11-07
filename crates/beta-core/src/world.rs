//! Bounded, read-only loading of Beta world metadata and chunk NBT.
//!
//! This module only adapts already parsed NBT into typed data. It does not
//! create game objects, repair damaged chunks, or access the filesystem.

use std::{error::Error, fmt};

use crate::{chunk, nbt, region};

const LEVEL_COMPRESSED_LIMIT: usize = 16 * 1024 * 1024;
const LEVEL_DECOMPRESSED_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldLimits {
    pub nbt: nbt::Limits,
    pub max_level_compressed_bytes: usize,
    pub max_level_decompressed_bytes: usize,
}

impl Default for WorldLimits {
    fn default() -> Self {
        Self {
            nbt: nbt::Limits::default(),
            max_level_compressed_bytes: LEVEL_COMPRESSED_LIMIT,
            max_level_decompressed_bytes: LEVEL_DECOMPRESSED_LIMIT,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldMetadata {
    pub seed: i64,
    pub spawn: [i32; 3],
    pub time: i64,
    pub last_played: i64,
    pub size_on_disk: i64,
    pub name: String,
    pub version: i32,
    pub dimension: i32,
    pub rain_time: i32,
    pub raining: bool,
    pub thunder_time: i32,
    pub thundering: bool,
    pub player: Option<nbt::Tag>,
    pub root: nbt::NamedTag,
    pub raw_nbt: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedChunk {
    pub x: i32,
    pub z: i32,
    pub last_update: i64,
    pub terrain_populated: bool,
    pub data: chunk::ChunkData,
    pub height_map: [u8; 256],
    pub entities: Vec<nbt::Tag>,
    pub tile_entities: Vec<nbt::Tag>,
    pub root: nbt::NamedTag,
    pub raw_nbt: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    Nbt(nbt::NbtError),
    Compression(region::RegionError),
    MissingField(&'static str),
    InvalidFieldType(&'static str),
    InvalidArrayLength {
        field: &'static str,
        actual: usize,
        expected: usize,
    },
    CoordinateMismatch {
        expected: (i32, i32),
        actual: (i32, i32),
    },
    LimitExceeded(&'static str),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nbt(error) => write!(f, "NBT: {error}"),
            Self::Compression(error) => write!(f, "compression: {error}"),
            Self::MissingField(name) => write!(f, "missing required field {name}"),
            Self::InvalidFieldType(name) => write!(f, "invalid type for field {name}"),
            Self::InvalidArrayLength {
                field,
                actual,
                expected,
            } => {
                write!(f, "{field} has length {actual}, expected {expected}")
            }
            Self::CoordinateMismatch { expected, actual } => write!(
                f,
                "chunk coordinates ({}, {}) do not match requested ({}, {})",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::LimitExceeded(name) => write!(f, "world {name} limit exceeded"),
        }
    }
}

impl Error for WorldError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Nbt(error) => Some(error),
            Self::Compression(error) => Some(error),
            _ => None,
        }
    }
}

impl From<nbt::NbtError> for WorldError {
    fn from(value: nbt::NbtError) -> Self {
        Self::Nbt(value)
    }
}

impl From<region::RegionError> for WorldError {
    fn from(value: region::RegionError) -> Self {
        Self::Compression(value)
    }
}

/// Decompress and parse a gzip-compressed level.dat payload without rewriting it.
pub fn load_level_gzip(bytes: &[u8], limits: WorldLimits) -> Result<WorldMetadata, WorldError> {
    if bytes.len() > limits.max_level_compressed_bytes {
        return Err(WorldError::LimitExceeded("compressed level"));
    }
    // The NBT reader rejects payloads above this cap before allocating its tree.
    let output_limit = limits
        .max_level_decompressed_bytes
        .min(limits.nbt.max_total_payload);
    let raw_nbt = region::decompress(1, bytes, output_limit)?;
    let (root, _) = nbt::read_named(&raw_nbt, limits.nbt)?;
    let data = named_compound_field(&root, "Data")?;

    let player = optional_compound(data, "Player")?.cloned();
    let dimension = match &player {
        Some(value) => optional_i32_field(value, "Dimension")?.unwrap_or(0),
        None => 0,
    };
    Ok(WorldMetadata {
        seed: scalar(data, "RandomSeed", 0_i64)?,
        spawn: [
            scalar(data, "SpawnX", 0_i32)?,
            scalar(data, "SpawnY", 0_i32)?,
            scalar(data, "SpawnZ", 0_i32)?,
        ],
        time: scalar(data, "Time", 0_i64)?,
        last_played: scalar(data, "LastPlayed", 0_i64)?,
        size_on_disk: scalar(data, "SizeOnDisk", 0_i64)?,
        name: scalar(data, "LevelName", String::new())?,
        version: scalar(data, "version", 0_i32)?,
        dimension,
        rain_time: scalar(data, "rainTime", 0_i32)?,
        raining: scalar(data, "raining", false)?,
        thunder_time: scalar(data, "thunderTime", 0_i32)?,
        thundering: scalar(data, "thundering", false)?,
        player,
        root,
        raw_nbt,
    })
}

/// Parse a chunk's complete raw NBT and validate its coordinates and arrays.
pub fn load_chunk_nbt(
    bytes: Vec<u8>,
    expected: (i32, i32),
    limits: nbt::Limits,
) -> Result<LoadedChunk, WorldError> {
    let (root, _) = nbt::read_named(&bytes, limits)?;
    let level = named_compound_field(&root, "Level")?;
    let x = required::<i32>(level, "xPos")?;
    let z = required::<i32>(level, "zPos")?;
    if (x, z) != expected {
        return Err(WorldError::CoordinateMismatch {
            expected,
            actual: (x, z),
        });
    }

    let blocks = required_array(level, "Blocks", chunk::BLOCKS_LEN)?;
    let data = required_array(level, "Data", chunk::NIBBLES_LEN)?;
    let block_light = required_array(level, "BlockLight", chunk::NIBBLES_LEN)?;
    let sky_light = required_array(level, "SkyLight", chunk::NIBBLES_LEN)?;
    let height = required_array(level, "HeightMap", 256)?;
    let height_map: [u8; 256] = height.try_into().expect("length checked");
    let entities = optional_list(level, "Entities")?;
    let tile_entities = optional_list(level, "TileEntities")?;

    let data = chunk::ChunkData::from_raw(blocks, data, block_light, sky_light)
        .expect("all raw array lengths checked");
    Ok(LoadedChunk {
        x,
        z,
        last_update: scalar(level, "LastUpdate", 0_i64)?,
        terrain_populated: scalar(level, "TerrainPopulated", false)?,
        data,
        height_map,
        entities,
        tile_entities,
        root,
        raw_nbt: bytes,
    })
}

fn compound_field<'a>(
    tag: &'a nbt::Tag,
    field: &'static str,
) -> Result<&'a [(String, nbt::Tag)], WorldError> {
    match tag {
        nbt::Tag::Compound(fields) => Ok(fields),
        _ => Err(WorldError::InvalidFieldType(field)),
    }
}

fn named_compound_field<'a>(
    root: &'a nbt::NamedTag,
    name: &'static str,
) -> Result<&'a [(String, nbt::Tag)], WorldError> {
    let fields = compound_field(&root.value, "root")?;
    match fields.iter().find(|(key, _)| key == name) {
        Some((_, nbt::Tag::Compound(value))) => Ok(value),
        Some(_) => Err(WorldError::InvalidFieldType(name)),
        None => Err(WorldError::MissingField(name)),
    }
}

fn get_field<'a>(fields: &'a [(String, nbt::Tag)], name: &'static str) -> Option<&'a nbt::Tag> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn scalar<T: Scalar>(
    fields: &[(String, nbt::Tag)],
    name: &'static str,
    default: T,
) -> Result<T, WorldError> {
    get_field(fields, name)
        .map_or(Ok(default), T::from_tag)
        .map_err(|()| WorldError::InvalidFieldType(name))
}

trait Scalar: Sized {
    fn from_tag(tag: &nbt::Tag) -> Result<Self, ()>;
}

macro_rules! scalar_impl {
    ($type:ty, $variant:ident) => {
        impl Scalar for $type {
            fn from_tag(tag: &nbt::Tag) -> Result<Self, ()> {
                if let nbt::Tag::$variant(value) = tag {
                    Ok(*value)
                } else {
                    Err(())
                }
            }
        }
    };
}

scalar_impl!(i32, Int);
scalar_impl!(i64, Long);

impl Scalar for String {
    fn from_tag(tag: &nbt::Tag) -> Result<Self, ()> {
        if let nbt::Tag::String(value) = tag {
            Ok(value.clone())
        } else {
            Err(())
        }
    }
}

impl Scalar for bool {
    fn from_tag(tag: &nbt::Tag) -> Result<Self, ()> {
        if let nbt::Tag::Byte(value) = tag {
            Ok(*value != 0)
        } else {
            Err(())
        }
    }
}

fn required<T: Scalar>(fields: &[(String, nbt::Tag)], name: &'static str) -> Result<T, WorldError> {
    let value = get_field(fields, name).ok_or(WorldError::MissingField(name))?;
    T::from_tag(value).map_err(|()| WorldError::InvalidFieldType(name))
}

fn required_array(
    fields: &[(String, nbt::Tag)],
    name: &'static str,
    expected: usize,
) -> Result<Vec<u8>, WorldError> {
    let value = get_field(fields, name).ok_or(WorldError::MissingField(name))?;
    match value {
        nbt::Tag::ByteArray(bytes) => {
            if bytes.len() != expected {
                return Err(WorldError::InvalidArrayLength {
                    field: name,
                    actual: bytes.len(),
                    expected,
                });
            }
            Ok(bytes.iter().map(|byte| *byte as u8).collect())
        }
        _ => Err(WorldError::InvalidFieldType(name)),
    }
}

fn optional_compound<'a>(
    fields: &'a [(String, nbt::Tag)],
    name: &'static str,
) -> Result<Option<&'a nbt::Tag>, WorldError> {
    match get_field(fields, name) {
        None => Ok(None),
        Some(tag @ nbt::Tag::Compound(_)) => Ok(Some(tag)),
        Some(_) => Err(WorldError::InvalidFieldType(name)),
    }
}

fn optional_i32_field(tag: &nbt::Tag, name: &'static str) -> Result<Option<i32>, WorldError> {
    let fields = compound_field(tag, "Player")?;
    match get_field(fields, name) {
        None => Ok(None),
        Some(value) => i32::from_tag(value)
            .map(Some)
            .map_err(|()| WorldError::InvalidFieldType(name)),
    }
}

fn optional_list(
    fields: &[(String, nbt::Tag)],
    name: &'static str,
) -> Result<Vec<nbt::Tag>, WorldError> {
    match get_field(fields, name) {
        None => Ok(Vec::new()),
        Some(nbt::Tag::List { values, .. }) => {
            if values
                .iter()
                .any(|value| !matches!(value, nbt::Tag::Compound(_)))
            {
                return Err(WorldError::InvalidFieldType(name));
            }
            Ok(values.clone())
        }
        Some(_) => Err(WorldError::InvalidFieldType(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    fn compound(fields: Vec<(&str, nbt::Tag)>) -> nbt::Tag {
        nbt::Tag::Compound(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn encode(tag: nbt::Tag) -> Vec<u8> {
        nbt::write_named(
            &nbt::NamedTag {
                name: String::new(),
                value: tag,
            },
            nbt::Limits::default(),
        )
        .unwrap()
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn level(data: nbt::Tag) -> Vec<u8> {
        gzip(&encode(compound(vec![("Data", data)])))
    }

    fn chunk(fields: Vec<(&str, nbt::Tag)>) -> Vec<u8> {
        encode(compound(vec![("Level", compound(fields))]))
    }

    fn byte_array(length: usize, value: i8) -> nbt::Tag {
        nbt::Tag::ByteArray(vec![value; length])
    }

    fn valid_chunk_fields() -> Vec<(&'static str, nbt::Tag)> {
        vec![
            ("xPos", nbt::Tag::Int(-2)),
            ("zPos", nbt::Tag::Int(7)),
            ("Blocks", byte_array(chunk::BLOCKS_LEN, -128)),
            ("Data", byte_array(chunk::NIBBLES_LEN, -1)),
            ("BlockLight", byte_array(chunk::NIBBLES_LEN, 0x12)),
            ("SkyLight", byte_array(chunk::NIBBLES_LEN, 0)),
            ("HeightMap", byte_array(256, -1)),
        ]
    }

    #[test]
    fn metadata_uses_exact_types_defaults_and_preserves_original_tree() {
        let player = compound(vec![
            ("Dimension", nbt::Tag::Int(-1)),
            (
                "Pos",
                nbt::Tag::List {
                    element_type: nbt::TagType::Double,
                    values: vec![nbt::Tag::Double(f64::from_bits(0x7ff8_0000_0000_0042))],
                },
            ),
            ("custom", nbt::Tag::String("kept".into())),
        ]);
        let raw = level(compound(vec![
            ("RandomSeed", nbt::Tag::Long(-42)),
            ("SpawnX", nbt::Tag::Int(-9)),
            ("raining", nbt::Tag::Byte(-128)),
            ("Player", player),
            (
                "future",
                nbt::Tag::List {
                    element_type: nbt::TagType::Int,
                    values: vec![nbt::Tag::Int(1)],
                },
            ),
        ]));
        let plain = flate2::read::GzDecoder::new(raw.as_slice());
        let mut expected_raw = Vec::new();
        use std::io::Read;
        plain.take(1024).read_to_end(&mut expected_raw).unwrap();
        let loaded = load_level_gzip(&raw, WorldLimits::default()).unwrap();
        assert_eq!(loaded.seed, -42);
        assert_eq!(loaded.spawn, [-9, 0, 0]);
        assert!(loaded.raining);
        assert!(!loaded.thundering);
        assert_eq!(loaded.dimension, -1);
        assert_eq!(loaded.raw_nbt, expected_raw);
        let Some(nbt::Tag::Compound(player_fields)) = &loaded.player else {
            panic!("player compound must be preserved");
        };
        let Some(nbt::Tag::List { values, .. }) = player_fields
            .iter()
            .find(|(name, _)| name == "Pos")
            .map(|(_, tag)| tag)
        else {
            panic!("player Pos list must be preserved");
        };
        let nbt::Tag::Double(position) = values[0] else {
            panic!("player NaN value must be preserved");
        };
        // NBT writer follows Java DataOutput's canonical NaN encoding; loader
        // retains those exact source bytes instead of re-encoding the tree.
        assert_eq!(position.to_bits(), 0x7ff8_0000_0000_0000);
    }

    #[test]
    fn metadata_rejects_wrong_types_and_enforces_both_limits() {
        assert_eq!(
            load_level_gzip(
                &level(compound(vec![("Time", nbt::Tag::Int(1))])),
                WorldLimits::default()
            ),
            Err(WorldError::InvalidFieldType("Time"))
        );
        assert_eq!(
            load_level_gzip(&[], WorldLimits::default()),
            Err(WorldError::Compression(region::RegionError::Decompression))
        );
        let gzip = level(compound(vec![]));
        let compressed_limits = WorldLimits {
            max_level_compressed_bytes: gzip.len() - 1,
            ..WorldLimits::default()
        };
        assert_eq!(
            load_level_gzip(&gzip, compressed_limits),
            Err(WorldError::LimitExceeded("compressed level"))
        );
        let output_limits = WorldLimits {
            max_level_decompressed_bytes: 4,
            ..WorldLimits::default()
        };
        assert!(matches!(
            load_level_gzip(&gzip, output_limits),
            Err(WorldError::Compression(region::RegionError::LimitExceeded))
        ));
        let nbt_limits = WorldLimits {
            nbt: nbt::Limits {
                max_total_payload: 3,
                ..nbt::Limits::default()
            },
            ..WorldLimits::default()
        };
        assert!(matches!(
            load_level_gzip(&gzip, nbt_limits),
            Err(WorldError::Compression(region::RegionError::LimitExceeded))
        ));
        let mut corrupt = gzip.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        assert!(matches!(
            load_level_gzip(&corrupt, WorldLimits::default()),
            Err(WorldError::Compression(region::RegionError::Decompression))
        ));
        assert!(matches!(
            load_level_gzip(&gzip[..gzip.len() - 1], WorldLimits::default()),
            Err(WorldError::Compression(region::RegionError::Decompression))
        ));
    }

    #[test]
    fn chunk_validates_coordinates_lengths_and_preserves_signed_bytes_and_lists() {
        let mut fields = valid_chunk_fields();
        fields.push(("LastUpdate", nbt::Tag::Long(99)));
        fields.push(("TerrainPopulated", nbt::Tag::Byte(-128)));
        fields.push((
            "Entities",
            nbt::Tag::List {
                element_type: nbt::TagType::Compound,
                values: vec![compound(vec![("id", nbt::Tag::String("FutureMob".into()))])],
            },
        ));
        fields.push((
            "TileEntities",
            nbt::Tag::List {
                element_type: nbt::TagType::Byte,
                values: vec![],
            },
        ));
        fields.push(("future", nbt::Tag::String("untouched".into())));
        let raw = chunk(fields);
        let loaded = load_chunk_nbt(raw.clone(), (-2, 7), nbt::Limits::default()).unwrap();
        assert_eq!(loaded.raw_nbt, raw);
        assert_eq!((loaded.x, loaded.z, loaded.last_update), (-2, 7, 99));
        assert!(loaded.terrain_populated);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.tile_entities, Vec::<nbt::Tag>::new());
        assert_eq!(loaded.height_map, [255; 256]);
        assert_eq!(loaded.data.raw_arrays().0[0], 128);
        assert_eq!(loaded.data.raw_arrays().1[0], 255);
        assert!(matches!(
            load_chunk_nbt(raw, (-2, 8), nbt::Limits::default()),
            Err(WorldError::CoordinateMismatch {
                expected: (-2, 8),
                actual: (-2, 7)
            })
        ));
    }

    #[test]
    fn chunk_rejects_missing_wrong_type_bad_lengths_and_bad_lists() {
        let mut fields = valid_chunk_fields();
        fields.pop();
        assert!(matches!(
            load_chunk_nbt(chunk(fields), (-2, 7), nbt::Limits::default()),
            Err(WorldError::MissingField("HeightMap"))
        ));

        let mut fields = valid_chunk_fields();
        fields[0].1 = nbt::Tag::Long(-2);
        assert!(matches!(
            load_chunk_nbt(chunk(fields), (-2, 7), nbt::Limits::default()),
            Err(WorldError::InvalidFieldType("xPos"))
        ));

        let mut fields = valid_chunk_fields();
        fields[2].1 = byte_array(chunk::BLOCKS_LEN - 1, 0);
        assert!(matches!(
            load_chunk_nbt(chunk(fields), (-2, 7), nbt::Limits::default()),
            Err(WorldError::InvalidArrayLength {
                field: "Blocks",
                actual: 32767,
                expected: 32768
            })
        ));

        let mut fields = valid_chunk_fields();
        fields.push((
            "Entities",
            nbt::Tag::List {
                element_type: nbt::TagType::Byte,
                values: vec![nbt::Tag::Byte(1)],
            },
        ));
        assert!(matches!(
            load_chunk_nbt(chunk(fields), (-2, 7), nbt::Limits::default()),
            Err(WorldError::InvalidFieldType("Entities"))
        ));

        for index in 2..7 {
            let mut fields = valid_chunk_fields();
            let (name, value) = &mut fields[index];
            let nbt::Tag::ByteArray(bytes) = value else {
                unreachable!();
            };
            bytes.pop();
            let field = *name;
            assert!(matches!(
                load_chunk_nbt(chunk(fields), (-2, 7), nbt::Limits::default()),
                Err(WorldError::InvalidArrayLength { field: got, .. }) if got == field
            ));
        }

        let raw = chunk(valid_chunk_fields());
        let limits = nbt::Limits {
            max_total_payload: raw.len() - 1,
            ..nbt::Limits::default()
        };
        assert!(matches!(
            load_chunk_nbt(raw, (-2, 7), limits),
            Err(WorldError::Nbt(nbt::NbtError::LimitExceeded(
                "total payload"
            )))
        ));
    }
}
