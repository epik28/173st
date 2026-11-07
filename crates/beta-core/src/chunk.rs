//! Minecraft Beta 1.7.3 chunk data storage and block coordinate conversion.

use std::error::Error;
use std::fmt;

/// Length of the block IDs array in bytes (16 * 128 * 16).
pub const BLOCKS_LEN: usize = 32_768;

/// Length of each 4-bit nibble array in bytes (32768 / 2).
pub const NIBBLES_LEN: usize = 16_384;

/// Data for a single block within a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockData {
    /// Block ID (0..=255).
    pub id: u8,
    /// Block metadata (0..=15).
    pub metadata: u8,
    /// Block light level (0..=15).
    pub block_light: u8,
    /// Sky light level (0..=15).
    pub sky_light: u8,
}

/// Errors that can occur when accessing or constructing chunk data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkError {
    /// Coordinate out of bounds (x < 16, y < 128, z < 16).
    InvalidCoordinate,
    /// Nibble value exceeds 15 (must be in 0..=15).
    InvalidNibble,
    /// One or more array lengths do not match the expected chunk dimensions.
    InvalidLength,
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoordinate => write!(f, "coordinate out of bounds"),
            Self::InvalidNibble => write!(f, "nibble value exceeds 15"),
            Self::InvalidLength => write!(f, "raw array length mismatch"),
        }
    }
}

impl Error for ChunkError {}

/// Splits a world block coordinate into `(chunk_coord, local_coord)`.
///
/// Returns `floor(value / 16)` and local offset in `0..=15`.
/// Matches Java Beta arithmetic: `(value >> 4, (value & 15) as u8)`.
#[inline]
#[must_use]
pub fn split_block_coord(value: i32) -> (i32, u8) {
    (value >> 4, (value & 15) as u8)
}

/// Computes the linear block index for local coordinates `(x, y, z)`.
///
/// Formula: `y + z * 128 + x * 2048`.
/// Returns `Err(ChunkError::InvalidCoordinate)` if `x >= 16 || y >= 128 || z >= 16`.
#[inline]
pub fn block_index(x: u8, y: u8, z: u8) -> Result<usize, ChunkError> {
    if x >= 16 || y >= 128 || z >= 16 {
        return Err(ChunkError::InvalidCoordinate);
    }
    Ok((y as usize) + (z as usize) * 128 + (x as usize) * 2048)
}

/// Container for a 16×128×16 chunk's raw block data, metadata, and lighting.
///
/// Does not perform NBT serialization, light spreading ticks, or world generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkData {
    blocks: Vec<u8>,
    data: Vec<u8>,
    block_light: Vec<u8>,
    sky_light: Vec<u8>,
}

impl Default for ChunkData {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkData {
    /// Creates a new zero-initialized chunk container.
    ///
    /// All arrays are filled with zeroes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: vec![0; BLOCKS_LEN],
            data: vec![0; NIBBLES_LEN],
            block_light: vec![0; NIBBLES_LEN],
            sky_light: vec![0; NIBBLES_LEN],
        }
    }

    /// Creates a chunk container from pre-existing raw byte arrays.
    ///
    /// Returns `Err(ChunkError::InvalidLength)` if any array does not match
    /// its required length (32768 for blocks, 16384 for nibble arrays).
    pub fn from_raw(
        blocks: Vec<u8>,
        data: Vec<u8>,
        block_light: Vec<u8>,
        sky_light: Vec<u8>,
    ) -> Result<Self, ChunkError> {
        if blocks.len() != BLOCKS_LEN
            || data.len() != NIBBLES_LEN
            || block_light.len() != NIBBLES_LEN
            || sky_light.len() != NIBBLES_LEN
        {
            return Err(ChunkError::InvalidLength);
        }

        Ok(Self {
            blocks,
            data,
            block_light,
            sky_light,
        })
    }

    /// Retrieves the block data at the local chunk coordinate `(x, y, z)`.
    ///
    /// Returns `Err(ChunkError::InvalidCoordinate)` if coordinates are out of bounds.
    pub fn get(&self, x: u8, y: u8, z: u8) -> Result<BlockData, ChunkError> {
        let idx = block_index(x, y, z)?;
        let id = self.blocks[idx];

        let byte_idx = idx >> 1;
        let (metadata, block_light, sky_light) = if (idx & 1) == 0 {
            (
                self.data[byte_idx] & 0x0F,
                self.block_light[byte_idx] & 0x0F,
                self.sky_light[byte_idx] & 0x0F,
            )
        } else {
            (
                (self.data[byte_idx] >> 4) & 0x0F,
                (self.block_light[byte_idx] >> 4) & 0x0F,
                (self.sky_light[byte_idx] >> 4) & 0x0F,
            )
        };

        Ok(BlockData {
            id,
            metadata,
            block_light,
            sky_light,
        })
    }

    /// Sets the block data at the local chunk coordinate `(x, y, z)`.
    ///
    /// Validates all parameters before modifying any array:
    /// - Checks coordinates first (`Err(ChunkError::InvalidCoordinate)`);
    /// - Checks that all nibbles (`metadata`, `block_light`, `sky_light`) are `<= 15`
    ///   (`Err(ChunkError::InvalidNibble)`);
    /// - If any check fails, no bytes are altered.
    pub fn set(&mut self, x: u8, y: u8, z: u8, value: BlockData) -> Result<(), ChunkError> {
        let idx = block_index(x, y, z)?;

        if value.metadata > 15 || value.block_light > 15 || value.sky_light > 15 {
            return Err(ChunkError::InvalidNibble);
        }

        self.blocks[idx] = value.id;

        let byte_idx = idx >> 1;
        if (idx & 1) == 0 {
            // Even index: lower 4 bits
            self.data[byte_idx] = (self.data[byte_idx] & 0xF0) | value.metadata;
            self.block_light[byte_idx] = (self.block_light[byte_idx] & 0xF0) | value.block_light;
            self.sky_light[byte_idx] = (self.sky_light[byte_idx] & 0xF0) | value.sky_light;
        } else {
            // Odd index: upper 4 bits
            self.data[byte_idx] = (self.data[byte_idx] & 0x0F) | (value.metadata << 4);
            self.block_light[byte_idx] =
                (self.block_light[byte_idx] & 0x0F) | (value.block_light << 4);
            self.sky_light[byte_idx] = (self.sky_light[byte_idx] & 0x0F) | (value.sky_light << 4);
        }

        Ok(())
    }

    /// Returns references to the four raw internal byte arrays:
    /// `(blocks, data, block_light, sky_light)`.
    #[inline]
    #[must_use]
    pub fn raw_arrays(&self) -> (&[u8], &[u8], &[u8], &[u8]) {
        (&self.blocks, &self.data, &self.block_light, &self.sky_light)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_block_coord_standard_and_bounds() {
        assert_eq!(split_block_coord(0), (0, 0));
        assert_eq!(split_block_coord(15), (0, 15));
        assert_eq!(split_block_coord(16), (1, 0));
        assert_eq!(split_block_coord(17), (1, 1));
        assert_eq!(split_block_coord(-1), (-1, 15));
        assert_eq!(split_block_coord(-15), (-1, 1));
        assert_eq!(split_block_coord(-16), (-1, 0));
        assert_eq!(split_block_coord(-17), (-2, 15));
        assert_eq!(split_block_coord(i32::MIN), (-134_217_728, 0));
        assert_eq!(split_block_coord(i32::MAX), (134_217_727, 15));
    }

    #[test]
    fn test_block_index_computation() {
        assert_eq!(block_index(0, 0, 0), Ok(0));
        assert_eq!(block_index(0, 1, 0), Ok(1));
        assert_eq!(block_index(0, 0, 1), Ok(128));
        assert_eq!(block_index(1, 0, 0), Ok(2048));
        assert_eq!(block_index(15, 127, 15), Ok(32767));

        assert_eq!(block_index(16, 0, 0), Err(ChunkError::InvalidCoordinate));
        assert_eq!(block_index(0, 128, 0), Err(ChunkError::InvalidCoordinate));
        assert_eq!(block_index(0, 0, 16), Err(ChunkError::InvalidCoordinate));
        assert_eq!(
            block_index(255, 255, 255),
            Err(ChunkError::InvalidCoordinate)
        );
    }

    #[test]
    fn test_chunk_new_and_raw_arrays() {
        let chunk = ChunkData::new();
        let (blocks, data, block_light, sky_light) = chunk.raw_arrays();
        assert_eq!(blocks.len(), BLOCKS_LEN);
        assert_eq!(data.len(), NIBBLES_LEN);
        assert_eq!(block_light.len(), NIBBLES_LEN);
        assert_eq!(sky_light.len(), NIBBLES_LEN);
        assert!(blocks.iter().all(|&b| b == 0));
        assert!(data.iter().all(|&b| b == 0));
        assert!(block_light.iter().all(|&b| b == 0));
        assert!(sky_light.iter().all(|&b| b == 0));

        let block = chunk.get(0, 0, 0).unwrap();
        assert_eq!(
            block,
            BlockData {
                id: 0,
                metadata: 0,
                block_light: 0,
                sky_light: 0,
            }
        );
    }

    #[test]
    fn test_from_raw_lengths() {
        let ok = ChunkData::from_raw(
            vec![0; BLOCKS_LEN],
            vec![0; NIBBLES_LEN],
            vec![0; NIBBLES_LEN],
            vec![0; NIBBLES_LEN],
        );
        assert!(ok.is_ok());

        assert_eq!(
            ChunkData::from_raw(
                vec![0; BLOCKS_LEN - 1],
                vec![0; NIBBLES_LEN],
                vec![0; NIBBLES_LEN],
                vec![0; NIBBLES_LEN],
            ),
            Err(ChunkError::InvalidLength)
        );

        assert_eq!(
            ChunkData::from_raw(
                vec![0; BLOCKS_LEN],
                vec![0; NIBBLES_LEN + 1],
                vec![0; NIBBLES_LEN],
                vec![0; NIBBLES_LEN],
            ),
            Err(ChunkError::InvalidLength)
        );
    }

    #[test]
    fn test_nibble_packing_and_neighbor_isolation() {
        let mut chunk = ChunkData::new();

        // (0, 0, 0) -> index 0 (even, low nibble)
        // (0, 1, 0) -> index 1 (odd, high nibble)
        let b0 = BlockData {
            id: 1,
            metadata: 3,
            block_light: 5,
            sky_light: 7,
        };
        chunk.set(0, 0, 0, b0).unwrap();

        // Check b0 is set and neighbor index 1 is still 0
        assert_eq!(chunk.get(0, 0, 0).unwrap(), b0);
        assert_eq!(
            chunk.get(0, 1, 0).unwrap(),
            BlockData {
                id: 0,
                metadata: 0,
                block_light: 0,
                sky_light: 0,
            }
        );

        let (blocks, data, block_light, sky_light) = chunk.raw_arrays();
        assert_eq!(blocks[0], 1);
        assert_eq!(data[0], 0x03);
        assert_eq!(block_light[0], 0x05);
        assert_eq!(sky_light[0], 0x07);

        // Now set b1 at (0, 1, 0)
        let b1 = BlockData {
            id: 42,
            metadata: 12,
            block_light: 9,
            sky_light: 15,
        };
        chunk.set(0, 1, 0, b1).unwrap();

        // Verify both b0 and b1 are correct
        assert_eq!(chunk.get(0, 0, 0).unwrap(), b0);
        assert_eq!(chunk.get(0, 1, 0).unwrap(), b1);

        let (blocks, data, block_light, sky_light) = chunk.raw_arrays();
        assert_eq!(blocks[0], 1);
        assert_eq!(blocks[1], 42);
        // data[0] has low=3, high=12 -> 0xC3 = 195
        assert_eq!(data[0], 0xC3);
        // block_light[0] has low=5, high=9 -> 0x95
        assert_eq!(block_light[0], 0x95);
        // sky_light[0] has low=7, high=15 -> 0xF7
        assert_eq!(sky_light[0], 0xF7);
    }

    #[test]
    fn test_set_atomic_on_error() {
        let mut chunk = ChunkData::new();

        let initial_block = BlockData {
            id: 10,
            metadata: 1,
            block_light: 2,
            sky_light: 3,
        };
        chunk.set(5, 5, 5, initial_block).unwrap();

        // 1. Invalid coordinate: nothing should change
        let bad_coord_block = BlockData {
            id: 99,
            metadata: 9,
            block_light: 9,
            sky_light: 9,
        };
        assert_eq!(
            chunk.set(16, 5, 5, bad_coord_block),
            Err(ChunkError::InvalidCoordinate)
        );
        assert_eq!(chunk.get(5, 5, 5).unwrap(), initial_block);

        // 2. Invalid nibble: blocks array must NOT be updated even though coordinates are valid
        let bad_nibble_block = BlockData {
            id: 99,
            metadata: 16,
            block_light: 2,
            sky_light: 3,
        };
        assert_eq!(
            chunk.set(5, 5, 5, bad_nibble_block),
            Err(ChunkError::InvalidNibble)
        );
        assert_eq!(chunk.get(5, 5, 5).unwrap(), initial_block);

        // 3. Both coordinate and nibble invalid -> coordinate check takes precedence
        let bad_both_block = BlockData {
            id: 99,
            metadata: 16,
            block_light: 20,
            sky_light: 30,
        };
        assert_eq!(
            chunk.set(16, 128, 16, bad_both_block),
            Err(ChunkError::InvalidCoordinate)
        );
    }
}
