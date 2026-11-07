//! Read-only reader for the Beta-era McRegion (`.mcr`) container.
//!
//! The input bytes are borrowed for the lifetime of [`RegionFile`]. Chunk
//! payloads are decompressed on demand and are never written back to the file.

use std::io::{self, BufRead, Cursor, Read};

use flate2::bufread::GzDecoder;
use flate2::{Decompress, FlushDecompress, Status};

const SECTOR_BYTES: usize = 4096;
const HEADER_BYTES: usize = 2 * SECTOR_BYTES;
const SLOT_COUNT: usize = 1024;

/// Default limits for untrusted region data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionLimits {
    pub max_input_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_decompressed_bytes: usize,
}

impl Default for RegionLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_compressed_bytes: 255 * SECTOR_BYTES - 5,
            max_decompressed_bytes: 64 * 1024 * 1024,
        }
    }
}

/// A decompressed chunk payload and its region metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRecord {
    pub timestamp: u32,
    pub compression: u8,
    pub nbt: Vec<u8>,
}

/// Errors detected while opening a region or reading one of its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionError {
    InvalidHeader,
    InvalidLocation,
    OverlappingSectors,
    InvalidLength,
    UnsupportedCompression(u8),
    Decompression,
    LimitExceeded,
}

impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader => f.write_str("invalid region header"),
            Self::InvalidLocation => f.write_str("invalid region location"),
            Self::OverlappingSectors => f.write_str("overlapping region sectors"),
            Self::InvalidLength => f.write_str("invalid chunk record length"),
            Self::UnsupportedCompression(id) => write!(f, "unsupported compression id {id}"),
            Self::Decompression => f.write_str("chunk decompression failed"),
            Self::LimitExceeded => f.write_str("region resource limit exceeded"),
        }
    }
}

impl std::error::Error for RegionError {}

#[derive(Debug, Clone, Copy)]
struct Location {
    offset: usize,
}

/// Parsed metadata borrowing the original region bytes.
#[derive(Debug)]
pub struct RegionFile<'a> {
    bytes: &'a [u8],
    locations: [Option<Location>; SLOT_COUNT],
    timestamps: [u32; SLOT_COUNT],
    limits: RegionLimits,
}

impl<'a> RegionFile<'a> {
    /// Validate a complete McRegion header and all record ranges.
    pub fn open(bytes: &'a [u8], limits: RegionLimits) -> Result<Self, RegionError> {
        if bytes.len() > limits.max_input_bytes {
            return Err(RegionError::LimitExceeded);
        }
        if bytes.len() < HEADER_BYTES || !bytes.len().is_multiple_of(SECTOR_BYTES) {
            return Err(RegionError::InvalidHeader);
        }

        let file_sectors = bytes.len() / SECTOR_BYTES;
        let mut locations = [None; SLOT_COUNT];
        let mut timestamps = [0; SLOT_COUNT];
        let mut ranges = Vec::with_capacity(SLOT_COUNT);

        for (slot, location) in locations.iter_mut().enumerate() {
            let at = slot * 4;
            let packed = u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
            let offset = (packed >> 8) as usize;
            let sectors = (packed & 0xff) as usize;
            if offset == 0 && sectors == 0 {
                continue;
            }
            if offset < 2 || sectors == 0 || offset >= file_sectors {
                return Err(RegionError::InvalidLocation);
            }
            let end_sector = offset
                .checked_add(sectors)
                .ok_or(RegionError::InvalidLocation)?;
            if end_sector > file_sectors {
                return Err(RegionError::InvalidLocation);
            }

            let record_at = offset
                .checked_mul(SECTOR_BYTES)
                .ok_or(RegionError::InvalidLocation)?;
            let length = read_u32(bytes, record_at).ok_or(RegionError::InvalidLocation)? as usize;
            if length < 1 {
                return Err(RegionError::InvalidLength);
            }
            let max_record_bytes = sectors
                .checked_mul(SECTOR_BYTES)
                .and_then(|v| v.checked_sub(4))
                .ok_or(RegionError::InvalidLocation)?;
            if length > max_record_bytes {
                return Err(RegionError::InvalidLength);
            }
            if length - 1 > limits.max_compressed_bytes {
                return Err(RegionError::LimitExceeded);
            }
            let data_end = record_at
                .checked_add(4)
                .and_then(|v| v.checked_add(length))
                .ok_or(RegionError::InvalidLocation)?;
            if data_end > bytes.len() {
                return Err(RegionError::InvalidLocation);
            }
            *location = Some(Location { offset: record_at });
            ranges.push((offset, end_sector));
        }

        for (slot, timestamp) in timestamps.iter_mut().enumerate() {
            let at = SECTOR_BYTES + slot * 4;
            *timestamp = u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
        }
        ranges.sort_unstable_by_key(|range| range.0);
        if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(RegionError::OverlappingSectors);
        }

        Ok(Self {
            bytes,
            locations,
            timestamps,
            limits,
        })
    }

    /// Return a chunk for local coordinates 0..32, or `None` for an empty slot.
    pub fn read_local(&self, x: u8, z: u8) -> Result<Option<ChunkRecord>, RegionError> {
        if x >= 32 || z >= 32 {
            return Err(RegionError::InvalidLocation);
        }
        let slot = x as usize + z as usize * 32;
        let Some(location) = self.locations[slot] else {
            return Ok(None);
        };
        let length =
            read_u32(self.bytes, location.offset).ok_or(RegionError::InvalidLocation)? as usize;
        let record_end = location
            .offset
            .checked_add(4)
            .and_then(|v| v.checked_add(length))
            .ok_or(RegionError::InvalidLocation)?;
        let record = self
            .bytes
            .get(location.offset + 4..record_end)
            .ok_or(RegionError::InvalidLocation)?;
        let (&compression, compressed) = record.split_first().ok_or(RegionError::InvalidLength)?;
        let nbt = decompress(compression, compressed, self.limits.max_decompressed_bytes)?;
        Ok(Some(ChunkRecord {
            timestamp: self.timestamps[slot],
            compression,
            nbt,
        }))
    }
}

/// Region coordinate using floor division, including negative chunks.
#[must_use]
pub const fn region_coord(chunk_coord: i32) -> i32 {
    chunk_coord.div_euclid(32)
}

/// Local coordinate in a region, in the range 0..32.
#[must_use]
pub const fn local_coord(chunk_coord: i32) -> u8 {
    chunk_coord.rem_euclid(32) as u8
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_be_bytes(bytes.get(at..end)?.try_into().ok()?))
}

fn decompress(id: u8, input: &[u8], limit: usize) -> Result<Vec<u8>, RegionError> {
    let mut output = Vec::with_capacity(limit.min(8192));
    let result = match id {
        1 => read_gzip_members(input, &mut output, limit),
        2 => read_zlib(input, &mut output, limit),
        other => return Err(RegionError::UnsupportedCompression(other)),
    };
    result?;
    Ok(output)
}

fn read_zlib(input: &[u8], output: &mut Vec<u8>, limit: usize) -> Result<(), RegionError> {
    let mut decoder = Decompress::new(true);
    let mut input_offset = 0;
    loop {
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let remaining = limit
            .checked_sub(output.len())
            .ok_or(RegionError::LimitExceeded)?;
        let output_capacity = remaining.saturating_add(1).min(8192);
        let mut chunk = [0_u8; 8192];
        let status = decoder
            .decompress(
                &input[input_offset..],
                &mut chunk[..output_capacity],
                FlushDecompress::None,
            )
            .map_err(|_| RegionError::Decompression)?;
        let consumed =
            usize::try_from(decoder.total_in()).map_err(|_| RegionError::Decompression)?;
        let produced = usize::try_from(decoder.total_out() - before_out)
            .map_err(|_| RegionError::Decompression)?;
        if consumed < input_offset || consumed > input.len() || produced > output_capacity {
            return Err(RegionError::Decompression);
        }
        input_offset = consumed;
        output.extend_from_slice(&chunk[..produced]);
        if output.len() > limit {
            return Err(RegionError::LimitExceeded);
        }
        if status == Status::StreamEnd {
            return Ok(());
        }
        if decoder.total_in() == before_in && decoder.total_out() == before_out {
            return Err(RegionError::Decompression);
        }
    }
}

fn read_gzip_members(input: &[u8], output: &mut Vec<u8>, limit: usize) -> Result<(), RegionError> {
    let mut source = Cursor::new(input);
    let mut members = 0;
    loop {
        let next = source.fill_buf().map_err(map_decompression_error)?;
        if next.is_empty() {
            return if members == 0 {
                Err(RegionError::Decompression)
            } else {
                Ok(())
            };
        }
        if !next.starts_with(&[0x1f, 0x8b]) {
            // The original GZIPInputStream stops when the next bytes are not a
            // gzip member, so record padding/trailing bytes are ignored.
            return if members == 0 {
                Err(RegionError::Decompression)
            } else {
                Ok(())
            };
        }
        let remaining = limit
            .checked_sub(output.len())
            .ok_or(RegionError::LimitExceeded)?;
        {
            let mut decoder = GzDecoder::new(&mut source);
            if decoder.header().is_none() {
                return if members == 0 {
                    Err(RegionError::Decompression)
                } else {
                    Ok(())
                };
            }
            read_bounded(&mut decoder, output, remaining)?;
        }
        members += 1;
    }
}

fn read_bounded(
    reader: &mut impl Read,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), RegionError> {
    let initial_len = output.len();
    let cap = limit.checked_add(1).ok_or(RegionError::LimitExceeded)?;
    let mut bounded = reader.take(cap as u64);
    bounded
        .read_to_end(output)
        .map_err(map_decompression_error)?;
    if output.len() - initial_len > limit {
        return Err(RegionError::LimitExceeded);
    }
    Ok(())
}

fn map_decompression_error(_error: io::Error) -> RegionError {
    RegionError::Decompression
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{
        Compression,
        write::{GzEncoder, ZlibEncoder},
    };
    use std::io::Write;

    fn region(record: Option<(u8, Vec<u8>)>) -> Vec<u8> {
        let mut bytes = vec![0; 3 * SECTOR_BYTES];
        if let Some((compression, compressed)) = record {
            let length = compressed.len() + 1;
            bytes[0..4].copy_from_slice(&(2_u32 << 8 | 1).to_be_bytes());
            bytes[SECTOR_BYTES..SECTOR_BYTES + 4].copy_from_slice(&1234_u32.to_be_bytes());
            bytes[2 * SECTOR_BYTES..2 * SECTOR_BYTES + 4]
                .copy_from_slice(&(length as u32).to_be_bytes());
            bytes[2 * SECTOR_BYTES + 4] = compression;
            bytes[2 * SECTOR_BYTES + 5..2 * SECTOR_BYTES + 5 + compressed.len()]
                .copy_from_slice(&compressed);
        }
        bytes
    }

    fn compress_zlib(input: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        encoder.finish().unwrap()
    }

    fn compress_gzip(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn reads_zlib_record_and_metadata() {
        let input = b"\x0a\0\0";
        let bytes = region(Some((2, compress_zlib(input))));
        let file = RegionFile::open(&bytes, RegionLimits::default()).unwrap();
        assert_eq!(
            file.read_local(0, 0).unwrap(),
            Some(ChunkRecord {
                timestamp: 1234,
                compression: 2,
                nbt: input.to_vec(),
            })
        );
        assert_eq!(file.read_local(1, 0).unwrap(), None);
    }

    #[test]
    fn reads_gzip_and_multiple_members() {
        let mut compressed = compress_gzip(b"first");
        compressed.extend(compress_gzip(b"second"));
        compressed.extend_from_slice(b"trailing garbage");
        let bytes = region(Some((1, compressed)));
        let file = RegionFile::open(&bytes, RegionLimits::default()).unwrap();
        assert_eq!(file.read_local(0, 0).unwrap().unwrap().nbt, b"firstsecond");
    }

    #[test]
    fn rejects_empty_truncated_and_bad_checksum_streams() {
        for (compression, complete) in [
            (1, compress_gzip(b"payload")),
            (2, compress_zlib(b"payload")),
        ] {
            assert_eq!(
                decompress(compression, &[], 64),
                Err(RegionError::Decompression)
            );
            for cut in 1..=complete.len() {
                let truncated = &complete[..complete.len() - cut];
                assert_eq!(
                    decompress(compression, truncated, 64),
                    Err(RegionError::Decompression),
                    "compression={compression}, removed trailing {cut} bytes"
                );
            }
            let checksum_bytes = if compression == 1 { 8 } else { 4 };
            for index in complete.len() - checksum_bytes..complete.len() {
                let mut bad_checksum = complete.clone();
                bad_checksum[index] ^= 0x80;
                assert_eq!(
                    decompress(compression, &bad_checksum, 64),
                    Err(RegionError::Decompression),
                    "compression={compression}, corrupted trailer byte {index}"
                );
            }
        }
    }

    #[test]
    fn zlib_ignores_bytes_after_a_complete_stream() {
        let mut compressed = compress_zlib(b"payload");
        compressed.extend_from_slice(b"trailing bytes");
        assert_eq!(decompress(2, &compressed, 64).unwrap(), b"payload");
    }

    fn gzip_member_of_exact_size(target_size: usize) -> (Vec<u8>, Vec<u8>) {
        (8100..8300)
            .find_map(|size| {
                let mut raw = Vec::with_capacity(size);
                let mut state = 0x1234_5678_u32;
                for _ in 0..size {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    raw.push(state as u8);
                }
                let encoded = compress_gzip(&raw);
                (encoded.len() == target_size).then_some((raw, encoded))
            })
            .expect("find incompressible gzip member of the requested size")
    }

    #[test]
    fn concatenates_three_gzip_members_across_repeated_8191_byte_boundaries() {
        let (first_raw, mut compressed) = gzip_member_of_exact_size(8191);
        let (second_raw, second_member) = gzip_member_of_exact_size(8191);
        let third_raw = b"third member";
        compressed.extend(second_member);
        compressed.extend(compress_gzip(third_raw));
        let mut expected = first_raw;
        expected.extend(second_raw);
        expected.extend_from_slice(third_raw);
        assert_eq!(
            decompress(1, &compressed, expected.len()).unwrap(),
            expected
        );
    }

    #[test]
    fn accepts_trailing_garbage_including_partial_gzip_header() {
        let mut one_byte_tail = compress_gzip(b"payload");
        one_byte_tail.push(0x1f);
        assert_eq!(decompress(1, &one_byte_tail, 64).unwrap(), b"payload");

        let mut partial_header = compress_gzip(b"payload");
        partial_header.extend_from_slice(&[0x1f, 0x8b]);
        assert_eq!(decompress(1, &partial_header, 64).unwrap(), b"payload");
    }

    #[test]
    fn rejects_initial_invalid_header_and_full_header_with_broken_body_or_trailer() {
        assert_eq!(decompress(1, &[], 64), Err(RegionError::Decompression));
        assert_eq!(
            decompress(1, &[0x1f, 0x8b], 64),
            Err(RegionError::Decompression)
        );
        assert_eq!(
            decompress(1, b"not gzip", 64),
            Err(RegionError::Decompression)
        );

        let first = compress_gzip(b"first");
        let second = compress_gzip(b"second");
        let mut truncated_trailer = first.clone();
        truncated_trailer.extend_from_slice(&second[..second.len() - 4]);
        assert_eq!(
            decompress(1, &truncated_trailer, 64),
            Err(RegionError::Decompression)
        );

        let mut corrupt_trailer = first;
        let mut damaged_second = second;
        let last = damaged_second.len() - 1;
        damaged_second[last] ^= 1;
        corrupt_trailer.extend(damaged_second);
        assert_eq!(
            decompress(1, &corrupt_trailer, 64),
            Err(RegionError::Decompression)
        );
    }

    #[test]
    fn zlib_streams_larger_than_output_buffer_and_output_limit_are_handled() {
        let raw: Vec<u8> = (0..25_000).map(|index| (index * 31) as u8).collect();
        let compressed = compress_zlib(&raw);
        assert_eq!(decompress(2, &compressed, raw.len()).unwrap(), raw);
        assert_eq!(
            decompress(2, &compressed, raw.len() - 1),
            Err(RegionError::LimitExceeded)
        );
    }

    #[test]
    fn rejects_header_location_length_overlap_and_compression_errors() {
        assert_eq!(
            RegionFile::open(&[0; 100], RegionLimits::default()).unwrap_err(),
            RegionError::InvalidHeader
        );
        let mut bad = region(None);
        bad[0..4].copy_from_slice(&(1_u32 << 8 | 1).to_be_bytes());
        assert_eq!(
            RegionFile::open(&bad, RegionLimits::default()).unwrap_err(),
            RegionError::InvalidLocation
        );

        let mut bad = region(Some((2, compress_zlib(b"x"))));
        // The record length exceeds its one-sector allocation.
        bad[2 * SECTOR_BYTES..2 * SECTOR_BYTES + 4].copy_from_slice(&4093_u32.to_be_bytes());
        assert_eq!(
            RegionFile::open(&bad, RegionLimits::default()).unwrap_err(),
            RegionError::InvalidLength
        );

        let mut overlap = region(Some((2, compress_zlib(b"x"))));
        overlap[4..8].copy_from_slice(&(2_u32 << 8 | 1).to_be_bytes());
        assert_eq!(
            RegionFile::open(&overlap, RegionLimits::default()).unwrap_err(),
            RegionError::OverlappingSectors
        );

        let bytes = region(Some((3, vec![0])));
        let file = RegionFile::open(&bytes, RegionLimits::default()).unwrap();
        assert_eq!(
            file.read_local(0, 0),
            Err(RegionError::UnsupportedCompression(3))
        );
        let bytes = region(Some((2, vec![0, 1, 2])));
        let file = RegionFile::open(&bytes, RegionLimits::default()).unwrap();
        assert_eq!(file.read_local(0, 0), Err(RegionError::Decompression));
    }

    #[test]
    fn bounds_output_and_checks_coordinates() {
        let bytes = region(Some((2, compress_zlib(&vec![b'x'; 1024]))));
        let limits = RegionLimits {
            max_decompressed_bytes: 128,
            ..RegionLimits::default()
        };
        let file = RegionFile::open(&bytes, limits).unwrap();
        assert_eq!(file.read_local(0, 0), Err(RegionError::LimitExceeded));
        let file = RegionFile::open(&bytes, RegionLimits::default()).unwrap();
        assert_eq!(file.read_local(32, 0), Err(RegionError::InvalidLocation));
    }

    #[test]
    fn coordinate_math_floors_for_negative_values_and_boundaries() {
        for (input, region, local) in [
            (-33, -2, 31),
            (-32, -1, 0),
            (-31, -1, 1),
            (-1, -1, 31),
            (0, 0, 0),
            (31, 0, 31),
            (32, 1, 0),
            (33, 1, 1),
            (i32::MIN, -67_108_864, 0),
            (i32::MAX, 67_108_863, 31),
        ] {
            assert_eq!((region_coord(input), local_coord(input)), (region, local));
        }
    }
}
