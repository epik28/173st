//! Safe reader and deterministic writer for the uncompressed NBT wire format.
//!
//! Strings use Java's modified UTF-8 encoding (UTF-16 code units, with NUL encoded
//! as `C0 80`). This module handles only the NBT payload; gzip and region files
//! belong to higher-level storage adapters.

use std::{error::Error, fmt};

/// Default recursion depth limit. The root payload is at depth zero.
pub const DEFAULT_MAX_DEPTH: usize = 512;
/// Maximum modified UTF-8 byte length of one NBT string.
pub const DEFAULT_MAX_STRING_BYTES: usize = u16::MAX as usize;
/// Maximum aggregate number of list/compound entries and array elements.
pub const DEFAULT_MAX_COLLECTION_ELEMENTS: usize = 2_000_000;
/// Maximum total encoded NBT payload, including names and tag headers.
pub const DEFAULT_MAX_TOTAL_PAYLOAD: usize = 64 * 1024 * 1024;

/// Resource limits applied by both the reader and writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_depth: usize,
    pub max_string_bytes: usize,
    pub max_collection_elements: usize,
    pub max_total_payload: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_string_bytes: DEFAULT_MAX_STRING_BYTES,
            max_collection_elements: DEFAULT_MAX_COLLECTION_ELEMENTS,
            max_total_payload: DEFAULT_MAX_TOTAL_PAYLOAD,
        }
    }
}

/// NBT wire tag identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TagType {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
}

impl TryFrom<u8> for TagType {
    type Error = NbtError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::End),
            1 => Ok(Self::Byte),
            2 => Ok(Self::Short),
            3 => Ok(Self::Int),
            4 => Ok(Self::Long),
            5 => Ok(Self::Float),
            6 => Ok(Self::Double),
            7 => Ok(Self::ByteArray),
            8 => Ok(Self::String),
            9 => Ok(Self::List),
            10 => Ok(Self::Compound),
            11 => Ok(Self::IntArray),
            other => Err(NbtError::UnknownTag(other)),
        }
    }
}

/// A complete NBT tag value.
#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List {
        element_type: TagType,
        values: Vec<Tag>,
    },
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
}

impl Tag {
    #[must_use]
    pub const fn tag_type(&self) -> TagType {
        match self {
            Self::End => TagType::End,
            Self::Byte(_) => TagType::Byte,
            Self::Short(_) => TagType::Short,
            Self::Int(_) => TagType::Int,
            Self::Long(_) => TagType::Long,
            Self::Float(_) => TagType::Float,
            Self::Double(_) => TagType::Double,
            Self::ByteArray(_) => TagType::ByteArray,
            Self::String(_) => TagType::String,
            Self::List { .. } => TagType::List,
            Self::Compound(_) => TagType::Compound,
            Self::IntArray(_) => TagType::IntArray,
        }
    }
}

/// The required named root tag.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedTag {
    pub name: String,
    pub value: Tag,
}

/// NBT parsing, validation, or limit error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NbtError {
    UnexpectedEof,
    UnknownTag(u8),
    InvalidRootType,
    InvalidEndTag,
    InvalidListElementType,
    InvalidListElement,
    InvalidDuplicateName(String),
    NegativeLength(i32),
    LengthOverflow,
    TrailingBytes(usize),
    InvalidModifiedUtf8,
    UnpairedSurrogate,
    LimitExceeded(&'static str),
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => f.write_str("unexpected end of NBT input"),
            Self::UnknownTag(id) => write!(f, "unknown NBT tag type {id}"),
            Self::InvalidRootType => f.write_str("NBT root cannot be End"),
            Self::InvalidEndTag => f.write_str("End tag has no named payload"),
            Self::InvalidListElementType => f.write_str("invalid NBT list element type"),
            Self::InvalidListElement => f.write_str("NBT list element type mismatch"),
            Self::InvalidDuplicateName(name) => write!(f, "duplicate compound name {name:?}"),
            Self::NegativeLength(len) => write!(f, "negative NBT collection length {len}"),
            Self::LengthOverflow => f.write_str("NBT length overflow"),
            Self::TrailingBytes(count) => write!(f, "{count} trailing byte(s) after NBT root"),
            Self::InvalidModifiedUtf8 => f.write_str("invalid Java modified UTF-8"),
            Self::UnpairedSurrogate => f.write_str("unpaired UTF-16 surrogate in NBT string"),
            Self::LimitExceeded(name) => write!(f, "NBT {name} limit exceeded"),
        }
    }
}

impl Error for NbtError {}

/// Parses one named root and requires it to consume the complete input.
///
/// On error no partial tree is returned. The returned byte count is therefore
/// exactly `input.len()` for successful calls.
pub fn read_named(input: &[u8], limits: Limits) -> Result<(NamedTag, usize), NbtError> {
    if input.len() > limits.max_total_payload {
        return Err(NbtError::LimitExceeded("total payload"));
    }
    let mut reader = Reader {
        input,
        offset: 0,
        limits,
        elements: 0,
    };
    let tag_type = reader.read_type()?;
    if tag_type == TagType::End {
        return Err(NbtError::InvalidRootType);
    }
    let name = reader.read_string()?;
    let value = reader.read_payload(tag_type, 0)?;
    if reader.offset != input.len() {
        return Err(NbtError::TrailingBytes(input.len() - reader.offset));
    }
    Ok((NamedTag { name, value }, reader.offset))
}

/// Encodes a named root using big-endian numeric fields and modified UTF-8.
pub fn write_named(value: &NamedTag, limits: Limits) -> Result<Vec<u8>, NbtError> {
    let tag_type = value.value.tag_type();
    if tag_type == TagType::End {
        return Err(NbtError::InvalidRootType);
    }
    let mut writer = Writer {
        output: Vec::new(),
        limits,
        elements: 0,
    };
    writer.write_type(tag_type)?;
    writer.write_string(&value.name)?;
    writer.write_payload(&value.value, 0)?;
    Ok(writer.output)
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
    limits: Limits,
    elements: usize,
}

impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], NbtError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(NbtError::LengthOverflow)?;
        if end > self.input.len() {
            return Err(NbtError::UnexpectedEof);
        }
        let bytes = &self.input[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn read_type(&mut self) -> Result<TagType, NbtError> {
        TagType::try_from(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, NbtError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("length checked"),
        ))
    }

    fn read_i32(&mut self) -> Result<i32, NbtError> {
        Ok(i32::from_be_bytes(
            self.take(4)?.try_into().expect("length checked"),
        ))
    }

    fn read_i64(&mut self) -> Result<i64, NbtError> {
        Ok(i64::from_be_bytes(
            self.take(8)?.try_into().expect("length checked"),
        ))
    }

    fn read_string(&mut self) -> Result<String, NbtError> {
        let len = usize::from(self.read_u16()?);
        self.check_string_len(len)?;
        decode_modified_utf8(self.take(len)?)
    }

    fn check_string_len(&self, len: usize) -> Result<(), NbtError> {
        if len > self.limits.max_string_bytes {
            return Err(NbtError::LimitExceeded("string bytes"));
        }
        Ok(())
    }

    fn reserve_elements(&mut self, count: usize) -> Result<(), NbtError> {
        self.elements = self
            .elements
            .checked_add(count)
            .ok_or(NbtError::LengthOverflow)?;
        if self.elements > self.limits.max_collection_elements {
            return Err(NbtError::LimitExceeded("collection elements"));
        }
        Ok(())
    }

    fn read_length(&mut self) -> Result<usize, NbtError> {
        let signed = self.read_i32()?;
        if signed < 0 {
            return Err(NbtError::NegativeLength(signed));
        }
        let len = usize::try_from(signed).map_err(|_| NbtError::LengthOverflow)?;
        self.reserve_elements(len)?;
        Ok(len)
    }

    fn read_payload(&mut self, ty: TagType, depth: usize) -> Result<Tag, NbtError> {
        if depth > self.limits.max_depth {
            return Err(NbtError::LimitExceeded("depth"));
        }
        Ok(match ty {
            TagType::End => return Err(NbtError::InvalidEndTag),
            TagType::Byte => Tag::Byte(self.take(1)?[0] as i8),
            TagType::Short => Tag::Short(i16::from_be_bytes(
                self.take(2)?.try_into().expect("length checked"),
            )),
            TagType::Int => Tag::Int(self.read_i32()?),
            TagType::Long => Tag::Long(self.read_i64()?),
            TagType::Float => Tag::Float(f32::from_bits(u32::from_be_bytes(
                self.take(4)?.try_into().expect("length checked"),
            ))),
            TagType::Double => Tag::Double(f64::from_bits(u64::from_be_bytes(
                self.take(8)?.try_into().expect("length checked"),
            ))),
            TagType::ByteArray => {
                let len = self.read_length()?;
                let bytes = self.take(len)?;
                Tag::ByteArray(bytes.iter().map(|&byte| byte as i8).collect())
            }
            TagType::String => Tag::String(self.read_string()?),
            TagType::List => {
                let element_type = self.read_type()?;
                let len = self.read_length()?;
                if element_type == TagType::End && len != 0 {
                    return Err(NbtError::InvalidListElementType);
                }
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.read_payload(element_type, depth + 1)?);
                }
                Tag::List {
                    element_type,
                    values,
                }
            }
            TagType::Compound => {
                let mut fields = Vec::new();
                let mut names = std::collections::HashSet::new();
                loop {
                    let field_type = self.read_type()?;
                    if field_type == TagType::End {
                        break;
                    }
                    self.reserve_elements(1)?;
                    let name = self.read_string()?;
                    if !names.insert(name.clone()) {
                        return Err(NbtError::InvalidDuplicateName(name));
                    }
                    let field = self.read_payload(field_type, depth + 1)?;
                    fields.push((name, field));
                }
                Tag::Compound(fields)
            }
            TagType::IntArray => {
                let len = self.read_length()?;
                let bytes_len = len.checked_mul(4).ok_or(NbtError::LengthOverflow)?;
                if bytes_len > self.input.len().saturating_sub(self.offset) {
                    return Err(NbtError::UnexpectedEof);
                }
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.read_i32()?);
                }
                Tag::IntArray(values)
            }
        })
    }
}

struct Writer {
    output: Vec<u8>,
    limits: Limits,
    elements: usize,
}

impl Writer {
    fn append(&mut self, bytes: &[u8]) -> Result<(), NbtError> {
        let size = self
            .output
            .len()
            .checked_add(bytes.len())
            .ok_or(NbtError::LengthOverflow)?;
        if size > self.limits.max_total_payload {
            return Err(NbtError::LimitExceeded("total payload"));
        }
        self.output.extend_from_slice(bytes);
        Ok(())
    }

    fn write_type(&mut self, ty: TagType) -> Result<(), NbtError> {
        self.append(&[ty as u8])
    }

    fn reserve_elements(&mut self, count: usize) -> Result<(), NbtError> {
        self.elements = self
            .elements
            .checked_add(count)
            .ok_or(NbtError::LengthOverflow)?;
        if self.elements > self.limits.max_collection_elements {
            return Err(NbtError::LimitExceeded("collection elements"));
        }
        Ok(())
    }

    fn write_length(&mut self, len: usize) -> Result<(), NbtError> {
        self.reserve_elements(len)?;
        let signed = i32::try_from(len).map_err(|_| NbtError::LengthOverflow)?;
        self.append(&signed.to_be_bytes())
    }

    fn write_string(&mut self, value: &str) -> Result<(), NbtError> {
        let encoded_len = modified_utf8_len(value)?;
        if encoded_len > self.limits.max_string_bytes || encoded_len > usize::from(u16::MAX) {
            return Err(NbtError::LimitExceeded("string bytes"));
        }
        let encoded = encode_modified_utf8(value)?;
        self.append(&(encoded.len() as u16).to_be_bytes())?;
        self.append(&encoded)
    }

    fn write_payload(&mut self, value: &Tag, depth: usize) -> Result<(), NbtError> {
        if depth > self.limits.max_depth {
            return Err(NbtError::LimitExceeded("depth"));
        }
        match value {
            Tag::End => return Err(NbtError::InvalidEndTag),
            Tag::Byte(v) => self.append(&[*v as u8])?,
            Tag::Short(v) => self.append(&v.to_be_bytes())?,
            Tag::Int(v) => self.append(&v.to_be_bytes())?,
            Tag::Long(v) => self.append(&v.to_be_bytes())?,
            Tag::Float(v) => self.append(&v.to_bits().to_be_bytes())?,
            Tag::Double(v) => self.append(&v.to_bits().to_be_bytes())?,
            Tag::ByteArray(values) => {
                self.write_length(values.len())?;
                for chunk in values.chunks(8192) {
                    let bytes: Vec<u8> = chunk.iter().map(|&value| value as u8).collect();
                    self.append(&bytes)?;
                }
            }
            Tag::String(value) => self.write_string(value)?,
            Tag::List {
                element_type,
                values,
            } => {
                if *element_type == TagType::End && !values.is_empty() {
                    return Err(NbtError::InvalidListElementType);
                }
                for item in values {
                    if item.tag_type() != *element_type {
                        return Err(NbtError::InvalidListElement);
                    }
                }
                self.write_type(*element_type)?;
                self.write_length(values.len())?;
                for item in values {
                    self.write_payload(item, depth + 1)?;
                }
            }
            Tag::Compound(fields) => {
                self.reserve_elements(fields.len())?;
                let mut names = std::collections::HashSet::new();
                for (name, field) in fields {
                    if field.tag_type() == TagType::End {
                        return Err(NbtError::InvalidEndTag);
                    }
                    if !names.insert(name) {
                        return Err(NbtError::InvalidDuplicateName(name.clone()));
                    }
                    self.write_type(field.tag_type())?;
                    self.write_string(name)?;
                    self.write_payload(field, depth + 1)?;
                }
                self.write_type(TagType::End)?;
            }
            Tag::IntArray(values) => {
                self.write_length(values.len())?;
                for value in values {
                    self.append(&value.to_be_bytes())?;
                }
            }
        }
        Ok(())
    }
}

fn encode_modified_utf8(value: &str) -> Result<Vec<u8>, NbtError> {
    let mut output = Vec::with_capacity(modified_utf8_len(value)?);
    for unit in value.encode_utf16() {
        match unit {
            0x0001..=0x007f => output.push(unit as u8),
            0x0000..=0x07ff => {
                output.push((0xc0 | (unit >> 6)) as u8);
                output.push((0x80 | (unit & 0x3f)) as u8);
            }
            _ => {
                output.push((0xe0 | (unit >> 12)) as u8);
                output.push((0x80 | ((unit >> 6) & 0x3f)) as u8);
                output.push((0x80 | (unit & 0x3f)) as u8);
            }
        }
    }
    if output.len() > usize::from(u16::MAX) {
        return Err(NbtError::LimitExceeded("string bytes"));
    }
    Ok(output)
}

fn modified_utf8_len(value: &str) -> Result<usize, NbtError> {
    value.encode_utf16().try_fold(0usize, |size, unit| {
        size.checked_add(if (1..=0x7f).contains(&unit) {
            1
        } else if unit <= 0x07ff {
            2
        } else {
            3
        })
        .ok_or(NbtError::LengthOverflow)
    })
}

fn decode_modified_utf8(input: &[u8]) -> Result<String, NbtError> {
    let mut units = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let first = input[i];
        let unit = if first & 0x80 == 0 {
            i += 1;
            u16::from(first)
        } else if first & 0xe0 == 0xc0 {
            if i + 1 >= input.len() || input[i + 1] & 0xc0 != 0x80 {
                return Err(NbtError::InvalidModifiedUtf8);
            }
            let unit = (u16::from(first & 0x1f) << 6) | u16::from(input[i + 1] & 0x3f);
            i += 2;
            unit
        } else if first & 0xf0 == 0xe0 {
            if i + 2 >= input.len() || input[i + 1] & 0xc0 != 0x80 || input[i + 2] & 0xc0 != 0x80 {
                return Err(NbtError::InvalidModifiedUtf8);
            }
            let unit = (u16::from(first & 0x0f) << 12)
                | (u16::from(input[i + 1] & 0x3f) << 6)
                | u16::from(input[i + 2] & 0x3f);
            i += 3;
            unit
        } else {
            // Java modified UTF-8 encodes supplementary characters as a UTF-16
            // surrogate pair, never as a four-byte UTF-8 sequence.
            return Err(NbtError::InvalidModifiedUtf8);
        };
        units.push(unit);
    }
    String::from_utf16(&units).map_err(|_| NbtError::UnpairedSurrogate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(value: Tag) -> NamedTag {
        NamedTag {
            name: "root".into(),
            value,
        }
    }

    fn round_trip(value: NamedTag) {
        let bytes = write_named(&value, Limits::default()).unwrap();
        let (decoded, consumed) = read_named(&bytes, Limits::default()).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(write_named(&decoded, Limits::default()).unwrap(), bytes);
        if !contains_nan(&value.value) {
            assert_eq!(decoded, value);
        }
    }

    fn contains_nan(value: &Tag) -> bool {
        match value {
            Tag::Float(number) => number.is_nan(),
            Tag::Double(number) => number.is_nan(),
            Tag::List { values, .. } => values.iter().any(contains_nan),
            Tag::Compound(fields) => fields.iter().any(|(_, value)| contains_nan(value)),
            _ => false,
        }
    }

    #[test]
    fn all_tag_types_round_trip_and_write_deterministically() {
        let tree = named(Tag::Compound(vec![
            ("byte".into(), Tag::Byte(i8::MIN)),
            ("short".into(), Tag::Short(i16::MAX)),
            ("int".into(), Tag::Int(i32::MIN)),
            ("long".into(), Tag::Long(i64::MAX)),
            ("float".into(), Tag::Float(f32::from_bits(0x7fc0_1234))),
            (
                "double".into(),
                Tag::Double(f64::from_bits(0x7ff8_0000_0000_1234)),
            ),
            ("bytes".into(), Tag::ByteArray(vec![i8::MIN, 0, i8::MAX])),
            ("string".into(), Tag::String("nul\0 and 🧭".into())),
            (
                "empty".into(),
                Tag::List {
                    element_type: TagType::Long,
                    values: vec![],
                },
            ),
            (
                "list".into(),
                Tag::List {
                    element_type: TagType::Short,
                    values: vec![Tag::Short(-1), Tag::Short(2)],
                },
            ),
            ("compound".into(), Tag::Compound(vec![])),
            ("ints".into(), Tag::IntArray(vec![i32::MIN, 0, i32::MAX])),
        ]));
        round_trip(tree);
    }

    #[test]
    fn accepts_empty_root_compound_and_exact_numeric_bits() {
        round_trip(named(Tag::Compound(vec![])));
        for bits in [0x8000_0000, 0x7f80_0000, 0x7fc0_0123] {
            let bytes =
                write_named(&named(Tag::Float(f32::from_bits(bits))), Limits::default()).unwrap();
            let (parsed, _) = read_named(&bytes, Limits::default()).unwrap();
            let Tag::Float(value) = parsed.value else {
                panic!("expected float")
            };
            assert_eq!(value.to_bits(), bits);
        }
    }

    #[test]
    fn rejects_invalid_roots_lengths_utf_and_trailing_bytes() {
        assert_eq!(
            read_named(&[0], Limits::default()),
            Err(NbtError::InvalidRootType)
        );
        let mut negative = vec![7, 0, 0, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(
            read_named(&negative, Limits::default()),
            Err(NbtError::NegativeLength(-1))
        );
        negative = vec![8, 0, 0, 0, 1, 0xc2]; // truncated two-byte sequence
        assert_eq!(
            read_named(&negative, Limits::default()),
            Err(NbtError::InvalidModifiedUtf8)
        );
        let mut valid = write_named(&named(Tag::Int(1)), Limits::default()).unwrap();
        valid.push(0);
        assert!(matches!(
            read_named(&valid, Limits::default()),
            Err(NbtError::TrailingBytes(1))
        ));
    }

    #[test]
    fn rejects_duplicate_compound_names_and_mismatched_lists() {
        let duplicate = named(Tag::Compound(vec![
            ("x".into(), Tag::Byte(1)),
            ("x".into(), Tag::Byte(2)),
        ]));
        assert!(matches!(
            write_named(&duplicate, Limits::default()),
            Err(NbtError::InvalidDuplicateName(_))
        ));
        let mismatch = named(Tag::List {
            element_type: TagType::Byte,
            values: vec![Tag::Int(1)],
        });
        assert_eq!(
            write_named(&mismatch, Limits::default()),
            Err(NbtError::InvalidListElement)
        );
    }

    #[test]
    fn applies_depth_string_collection_and_total_limits() {
        let limits = Limits {
            max_depth: 0,
            ..Limits::default()
        };
        let nested = named(Tag::List {
            element_type: TagType::List,
            values: vec![Tag::List {
                element_type: TagType::Byte,
                values: vec![],
            }],
        });
        assert_eq!(
            write_named(&nested, limits),
            Err(NbtError::LimitExceeded("depth"))
        );
        let limits = Limits {
            max_string_bytes: 2,
            ..Limits::default()
        };
        assert_eq!(
            write_named(&named(Tag::String("abc".into())), limits),
            Err(NbtError::LimitExceeded("string bytes"))
        );
        let limits = Limits {
            max_collection_elements: 1,
            ..Limits::default()
        };
        assert_eq!(
            write_named(&named(Tag::IntArray(vec![1, 2])), limits),
            Err(NbtError::LimitExceeded("collection elements"))
        );
        let limits = Limits {
            max_total_payload: 4,
            ..Limits::default()
        };
        assert_eq!(
            write_named(&named(Tag::Int(1)), limits),
            Err(NbtError::LimitExceeded("total payload"))
        );
    }

    #[test]
    fn empty_list_keeps_element_type_and_writer_rejects_unpaired_surrogate_on_decode() {
        round_trip(named(Tag::List {
            element_type: TagType::Double,
            values: vec![],
        }));
        // Root String, empty name, then one modified UTF-8 high surrogate.
        let bytes = [8, 0, 0, 0, 3, 0xed, 0xa0, 0x80];
        assert_eq!(
            read_named(&bytes, Limits::default()),
            Err(NbtError::UnpairedSurrogate)
        );
    }

    #[test]
    fn modified_utf8_handles_nul_and_supplementary_code_points() {
        let value = named(Tag::String("\0🧭".into()));
        let bytes = write_named(&value, Limits::default()).unwrap();
        assert!(bytes.windows(2).any(|pair| pair == [0xc0, 0x80]));
        assert!(
            bytes
                .windows(3)
                .any(|triple| triple[0] == 0xed && triple[1] == 0xa0)
        );
        assert_eq!(read_named(&bytes, Limits::default()).unwrap().0, value);

        // DataInput.readUTF accepts the one-byte NUL spelling even though the
        // Java modified UTF-8 writer emits C0 80 for NUL.
        let raw_nul = [8, 0, 0, 0, 1, 0];
        assert_eq!(
            read_named(&raw_nul, Limits::default()).unwrap().0.value,
            Tag::String("\0".into())
        );
    }
}
