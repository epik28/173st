//! Bounded block targeting and creative edits over an already-loaded scene.
//!
//! This module owns no file I/O and does not persist edits. Callers provide the
//! native-confirmed set of block states that use full-cube ray geometry.

use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    chunk::{BlockData, ChunkData, ChunkError, split_block_coord},
    geometry::{Aabb, Vec3},
};

pub const BLOCK_REACH: f64 = 5.0;
pub const MAX_SCENE_CHUNKS: u8 = 5;
pub const MAX_RAY_STEPS: usize = 32;
/// Maximum number of cells inspected by one bounded relight operation.
pub const MAX_RELIGHT_CELLS: usize = 3 * 3 * 128 * 256;

/// Resolves states whose exact native interaction shape has been verified.
/// Returning `false` keeps a non-air state explicit as an unsupported target.
pub trait FullCubeStates {
    fn is_full_cube(&self, id: u8, metadata: u8) -> bool;
}

/// Interaction shape whose exact ray geometry has been verified against the
/// pinned native client. Unknown states must remain `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RayShape {
    FullCube,
    LowerSlab,
    Liquid,
    Unsupported,
}

pub trait RayShapes {
    fn ray_shape(&self, id: u8, metadata: u8) -> RayShape;
}

impl<F> RayShapes for F
where
    F: Fn(u8, u8) -> RayShape,
{
    fn ray_shape(&self, id: u8, metadata: u8) -> RayShape {
        self(id, metadata)
    }
}

impl<F> FullCubeStates for F
where
    F: Fn(u8, u8) -> bool,
{
    fn is_full_cube(&self, id: u8, metadata: u8) -> bool {
        self(id, metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSceneChunk {
    pub x: i32,
    pub z: i32,
    pub data: ChunkData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSceneBounds {
    pub min_chunk_x: i32,
    pub min_chunk_z: i32,
    pub width: u8,
    pub depth: u8,
}

impl BlockSceneBounds {
    pub fn new(
        min_chunk_x: i32,
        min_chunk_z: i32,
        width: u8,
        depth: u8,
    ) -> Result<Self, BlockError> {
        if width == 0 || depth == 0 || width > MAX_SCENE_CHUNKS || depth > MAX_SCENE_CHUNKS {
            return Err(BlockError::InvalidBounds);
        }
        let max_chunk_x = min_chunk_x
            .checked_add(i32::from(width) - 1)
            .ok_or(BlockError::InvalidBounds)?;
        let max_chunk_z = min_chunk_z
            .checked_add(i32::from(depth) - 1)
            .ok_or(BlockError::InvalidBounds)?;
        let min_x = i64::from(min_chunk_x) * 16;
        let min_z = i64::from(min_chunk_z) * 16;
        let max_x = i64::from(max_chunk_x) * 16 + 15;
        let max_z = i64::from(max_chunk_z) * 16 + 15;
        if min_x < i64::from(i32::MIN)
            || min_z < i64::from(i32::MIN)
            || max_x > i64::from(i32::MAX)
            || max_z > i64::from(i32::MAX)
        {
            return Err(BlockError::InvalidBounds);
        }
        Ok(Self {
            min_chunk_x,
            min_chunk_z,
            width,
            depth,
        })
    }

    fn contains(self, x: i32, z: i32) -> bool {
        i64::from(x) >= i64::from(self.min_chunk_x)
            && i64::from(z) >= i64::from(self.min_chunk_z)
            && i64::from(x) < i64::from(self.min_chunk_x) + i64::from(self.width)
            && i64::from(z) < i64::from(self.min_chunk_z) + i64::from(self.depth)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    InvalidBounds,
    ChunkOutsideScene { x: i32, z: i32 },
    DuplicateChunk { x: i32, z: i32 },
    InvalidRay,
    RayStepLimit,
    InvalidMetadata(u8),
    InvalidBlockId,
    UnsupportedPlacement { id: u8, metadata: u8 },
    HitNotEditable,
    StaleHit,
    TargetOccupied,
    TargetUnloaded { x: i32, y: i32, z: i32 },
    RevisionExhausted,
    Chunk(ChunkError),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => write!(f, "invalid block scene bounds"),
            Self::ChunkOutsideScene { x, z } => write!(f, "chunk ({x}, {z}) is outside scene"),
            Self::DuplicateChunk { x, z } => write!(f, "duplicate chunk ({x}, {z})"),
            Self::InvalidRay => write!(
                f,
                "ray must have finite origin and nonzero finite direction"
            ),
            Self::RayStepLimit => write!(f, "ray exceeds the voxel traversal cell limit"),
            Self::InvalidMetadata(value) => {
                write!(f, "metadata {value} exceeds nibble range 0..=15")
            }
            Self::InvalidBlockId => write!(f, "air cannot be placed"),
            Self::UnsupportedPlacement { id, metadata } => {
                write!(f, "block state {id}:{metadata} is not a verified full cube")
            }
            Self::HitNotEditable => write!(f, "hit is not an editable full-cube block"),
            Self::StaleHit => write!(f, "block changed since it was targeted"),
            Self::TargetOccupied => write!(f, "placement target is occupied"),
            Self::TargetUnloaded { x, y, z } => {
                write!(f, "placement target ({x}, {y}, {z}) is not loaded")
            }
            Self::RevisionExhausted => write!(f, "block scene revision counter is exhausted"),
            Self::Chunk(error) => write!(f, "chunk access: {error}"),
        }
    }
}

impl Error for BlockError {}

impl From<ChunkError> for BlockError {
    fn from(value: ChunkError) -> Self {
        Self::Chunk(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockHitKind {
    FullCube,
    LowerSlab,
    Liquid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockHit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Beta side index: down, up, north, south, west, east = 0..5.
    pub face: u8,
    pub position: Vec3,
    pub kind: BlockHitKind,
    pub id: u8,
    pub metadata: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub id: u8,
    pub metadata: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RayResult {
    Hit(BlockHit),
    Unsupported(UnsupportedBlock),
    Unloaded { x: i32, y: i32, z: i32 },
    Miss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockEdit {
    pub changed: (i32, i32, i32),
    /// Chunks whose mesh may need rebuilding, in deterministic coordinate order.
    pub dirty_chunks: Vec<(i32, i32)>,
    /// Incremented only after a successful mutation.
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelightResult {
    pub changed: (i32, i32, i32),
    pub light_updates: Vec<((i32, i32, i32), u8, u8)>,
    pub dirty_chunks: Vec<(i32, i32)>,
    pub unloaded_neighbors: Vec<(i32, i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelightError {
    InvalidQueueLimit,
    QueueLimitExceeded { limit: usize },
    InvalidCoordinate,
    InvalidMaterial,
    Scene(BlockError),
}
impl fmt::Display for RelightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RelightError {}
impl From<BlockError> for RelightError {
    fn from(e: BlockError) -> Self {
        Self::Scene(e)
    }
}

/// Owned bounded scene, containing only chunks which have actually been loaded.
#[derive(Debug, Clone)]
pub struct BlockScene {
    bounds: BlockSceneBounds,
    chunks: BTreeMap<(i32, i32), ChunkData>,
    revision: u64,
}

impl BlockScene {
    pub fn new(bounds: BlockSceneBounds, chunks: Vec<BlockSceneChunk>) -> Result<Self, BlockError> {
        let bounds = BlockSceneBounds::new(
            bounds.min_chunk_x,
            bounds.min_chunk_z,
            bounds.width,
            bounds.depth,
        )?;
        if chunks.len() > usize::from(bounds.width) * usize::from(bounds.depth) {
            return Err(BlockError::InvalidBounds);
        }
        let mut loaded = BTreeMap::new();
        for chunk in chunks {
            if !bounds.contains(chunk.x, chunk.z) {
                return Err(BlockError::ChunkOutsideScene {
                    x: chunk.x,
                    z: chunk.z,
                });
            }
            if loaded.insert((chunk.x, chunk.z), chunk.data).is_some() {
                return Err(BlockError::DuplicateChunk {
                    x: chunk.x,
                    z: chunk.z,
                });
            }
        }
        Ok(Self {
            bounds,
            chunks: loaded,
            revision: 0,
        })
    }

    #[must_use]
    pub fn bounds(&self) -> BlockSceneBounds {
        self.bounds
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Recomputes the native-observed six-cardinal sky-light update in the
    /// already loaded scene. The operation is pure with respect to disk and
    /// reports unloaded boundary cells explicitly.
    pub fn relight<F>(
        &mut self,
        changed: (i32, i32, i32),
        max_visited_cells: usize,
        material: F,
    ) -> Result<RelightResult, RelightError>
    where
        F: Fn(u8, u8) -> (u8, u8),
    {
        if !(1..=MAX_RELIGHT_CELLS).contains(&max_visited_cells) {
            return Err(RelightError::InvalidQueueLimit);
        }
        if !(0..128).contains(&changed.1) {
            return Err(RelightError::InvalidCoordinate);
        }
        let mut next = self.clone();
        let mut updates = Vec::new();
        let mut unloaded = Vec::new();
        let mut visited = 0usize;
        let keys: Vec<_> = self.chunks.keys().copied().collect();
        for (cx, cz) in keys {
            for lx in 0..16u8 {
                for y in 0..128u8 {
                    for lz in 0..16u8 {
                        let x = cx * 16 + i32::from(lx);
                        let z = cz * 16 + i32::from(lz);
                        visited += 1;
                        if visited > max_visited_cells {
                            return Err(RelightError::QueueLimitExceeded {
                                limit: max_visited_cells,
                            });
                        }
                        let block = self
                            .block(x, i32::from(y), z)
                            .map_err(RelightError::from)?
                            .ok_or(BlockError::TargetUnloaded {
                                x,
                                y: i32::from(y),
                                z,
                            })
                            .map_err(RelightError::from)?;
                        let (opacity, emission) = material(block.id, block.metadata);
                        if opacity > 15 || emission > 15 {
                            return Err(RelightError::InvalidMaterial);
                        }
                        let new_sky = if opacity != 0 {
                            0
                        } else if y == 0 {
                            0
                        } else if y == 127 {
                            15
                        } else {
                            let mut blocked_above = false;
                            for sy in (u16::from(y) + 1)..128 {
                                if self
                                    .block(x, i32::from(sy), z)
                                    .map_err(RelightError::from)?
                                    .is_some_and(|b| material(b.id, b.metadata).0 != 0)
                                {
                                    blocked_above = true;
                                    break;
                                }
                            }
                            if !blocked_above {
                                15
                            } else {
                                let mut best = 0u8;
                                let mut missing = false;
                                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                                    if let Some(n) = self
                                        .block(x + dx, i32::from(y), z + dz)
                                        .map_err(RelightError::from)?
                                    {
                                        best = best.max(n.sky_light.saturating_sub(1));
                                    } else {
                                        unloaded.push((x + dx, i32::from(y), z + dz));
                                        missing = true;
                                    }
                                }
                                if missing { 0 } else { best }
                            }
                        };
                        if new_sky != block.sky_light {
                            updates.push(((x, i32::from(y), z), block.block_light, new_sky));
                            let mut v = block;
                            v.sky_light = new_sky;
                            next.chunks
                                .get_mut(&(cx, cz))
                                .unwrap()
                                .set(lx, y, lz, v)
                                .map_err(|e| RelightError::Scene(e.into()))?;
                        }
                    }
                }
            }
        }
        unloaded.sort_unstable();
        unloaded.dedup();
        let mut dirty = vec![(changed.0 >> 4, changed.2 >> 4)];
        dirty.extend(updates.iter().map(|((x, _, z), _, _)| (x >> 4, z >> 4)));
        dirty.sort_unstable();
        dirty.dedup();
        *self = next;
        updates.sort_by_key(|(p, _, _)| *p);
        Ok(RelightResult {
            changed,
            light_updates: updates,
            dirty_chunks: dirty,
            unloaded_neighbors: unloaded,
        })
    }

    pub fn block(&self, x: i32, y: i32, z: i32) -> Result<Option<BlockData>, BlockError> {
        if !(0..128).contains(&y) {
            return Ok(None);
        }
        let (cx, lx) = split_block_coord(x);
        let (cz, lz) = split_block_coord(z);
        let Some(chunk) = self.chunks.get(&(cx, cz)) else {
            return Ok(None);
        };
        Ok(Some(chunk.get(lx, y as u8, lz)?))
    }

    /// Traces a normalized direction for the native controller's five-block reach.
    /// Shape support is supplied by the caller from the committed native mapping.
    pub fn raycast<S: FullCubeStates>(
        &self,
        origin: Vec3,
        direction: Vec3,
        supported: &S,
    ) -> Result<RayResult, BlockError> {
        if !finite(origin) || !finite(direction) || direction.length() == 0.0 {
            return Err(BlockError::InvalidRay);
        }
        let direction = direction.normalize();
        if !finite(direction) || direction.length() == 0.0 {
            return Err(BlockError::InvalidRay);
        }
        let end = origin.add_xyz(
            direction.x * BLOCK_REACH,
            direction.y * BLOCK_REACH,
            direction.z * BLOCK_REACH,
        );
        self.raycast_segment(origin, end, supported)
    }

    /// Traces the exact native start/end segment. A cell entered exactly at the
    /// endpoint is excluded, matching the client's open ray endpoint behavior.
    pub fn raycast_segment<S: FullCubeStates>(
        &self,
        origin: Vec3,
        end: Vec3,
        supported: &S,
    ) -> Result<RayResult, BlockError> {
        self.raycast_segment_with_shapes(origin, end, false, &|id, metadata| {
            if supported.is_full_cube(id, metadata) {
                RayShape::FullCube
            } else {
                RayShape::Unsupported
            }
        })
    }

    /// Traces a native segment with the C01-confirmed block shapes and the
    /// native stop-on-liquid option. Only those exact shapes should be supplied.
    pub fn raycast_segment_with_shapes<S: RayShapes>(
        &self,
        origin: Vec3,
        end: Vec3,
        stop_on_liquid: bool,
        shapes: &S,
    ) -> Result<RayResult, BlockError> {
        if !finite(origin) || !finite(end) {
            return Err(BlockError::InvalidRay);
        }
        let delta = origin.vector_to(end);
        if delta.x == 0.0 && delta.y == 0.0 && delta.z == 0.0 {
            return Err(BlockError::InvalidRay);
        }
        let required_cells = 1.0
            + axis_crossings(origin.x, end.x)
            + axis_crossings(origin.y, end.y)
            + axis_crossings(origin.z, end.z);
        if !required_cells.is_finite() || required_cells > MAX_RAY_STEPS as f64 {
            return Err(BlockError::RayStepLimit);
        }
        let mut cell = [
            floor_i32(origin.x)?,
            floor_i32(origin.y)?,
            floor_i32(origin.z)?,
        ];
        let step = [sign(delta.x), sign(delta.y), sign(delta.z)];
        let t_delta = [delta_t(delta.x), delta_t(delta.y), delta_t(delta.z)];
        let mut t_max = [
            first_t(origin.x, cell[0], step[0], delta.x),
            first_t(origin.y, cell[1], step[1], delta.y),
            first_t(origin.z, cell[2], step[2], delta.z),
        ];

        for _ in 0..MAX_RAY_STEPS {
            let [x, y, z] = cell;
            if (0..128).contains(&y) {
                let (cx, _) = split_block_coord(x);
                let (cz, _) = split_block_coord(z);
                if self.bounds.contains(cx, cz) {
                    let Some(block) = self.block(x, y, z)? else {
                        return Ok(RayResult::Unloaded { x, y, z });
                    };
                    if block.id != 0 {
                        let shape = shapes.ray_shape(block.id, block.metadata);
                        if shape == RayShape::Liquid && !stop_on_liquid {
                            // With stopOnLiquid=false, liquids do not participate
                            // in native block targeting.
                            let axis = min_axis(t_max);
                            if !t_max[axis].is_finite() || t_max[axis] >= 1.0 {
                                break;
                            }
                            t_max[axis] += t_delta[axis];
                            cell[axis] = cell[axis]
                                .checked_add(step[axis])
                                .ok_or(BlockError::InvalidRay)?;
                            continue;
                        }
                        let bounds = match shape {
                            RayShape::LowerSlab => Aabb::new(
                                [f64::from(x), f64::from(y), f64::from(z)],
                                [f64::from(x) + 1.0, f64::from(y) + 0.5, f64::from(z) + 1.0],
                            ),
                            _ => Aabb::new(
                                [f64::from(x), f64::from(y), f64::from(z)],
                                [f64::from(x) + 1.0, f64::from(y) + 1.0, f64::from(z) + 1.0],
                            ),
                        };
                        let hit = bounds.ray_intersect(origin, end);
                        if let Some(hit) = hit {
                            if shape == RayShape::Unsupported {
                                return Ok(RayResult::Unsupported(UnsupportedBlock {
                                    x,
                                    y,
                                    z,
                                    id: block.id,
                                    metadata: block.metadata,
                                }));
                            }
                            let result = BlockHit {
                                x,
                                y,
                                z,
                                face: hit.face,
                                position: hit.position,
                                kind: match shape {
                                    RayShape::FullCube => BlockHitKind::FullCube,
                                    RayShape::LowerSlab => BlockHitKind::LowerSlab,
                                    RayShape::Liquid => BlockHitKind::Liquid,
                                    RayShape::Unsupported => unreachable!("returned above"),
                                },
                                id: block.id,
                                metadata: block.metadata,
                            };
                            return Ok(RayResult::Hit(result));
                        }
                    }
                } else if cell_intersects_xz_bounds(self.bounds, x, z) {
                    return Ok(RayResult::Unloaded { x, y, z });
                }
            }

            let axis = min_axis(t_max);
            if !t_max[axis].is_finite() || t_max[axis] >= 1.0 {
                break;
            }
            t_max[axis] += t_delta[axis];
            cell[axis] = cell[axis]
                .checked_add(step[axis])
                .ok_or(BlockError::InvalidRay)?;
        }
        Ok(RayResult::Miss)
    }

    pub fn break_block<S: FullCubeStates>(
        &mut self,
        hit: BlockHit,
        supported: &S,
    ) -> Result<BlockEdit, BlockError> {
        if hit.kind != BlockHitKind::FullCube {
            return Err(BlockError::HitNotEditable);
        }
        let current = self
            .block(hit.x, hit.y, hit.z)?
            .ok_or(BlockError::StaleHit)?;
        if current.id != hit.id || current.metadata != hit.metadata || current.id == 0 {
            return Err(BlockError::StaleHit);
        }
        if !supported.is_full_cube(current.id, current.metadata) {
            return Err(BlockError::HitNotEditable);
        }
        let mut air = current;
        air.id = 0;
        air.metadata = 0;
        self.set_block(hit.x, hit.y, hit.z, air)
    }

    pub fn place_block<S: FullCubeStates>(
        &mut self,
        hit: BlockHit,
        id: u8,
        metadata: u8,
        supported: &S,
    ) -> Result<BlockEdit, BlockError> {
        if hit.kind != BlockHitKind::FullCube {
            return Err(BlockError::HitNotEditable);
        }
        if metadata > 15 {
            return Err(BlockError::InvalidMetadata(metadata));
        }
        if id == 0 {
            return Err(BlockError::InvalidBlockId);
        }
        if metadata != 0 || !supported.is_full_cube(id, metadata) {
            return Err(BlockError::UnsupportedPlacement { id, metadata });
        }
        let current = self
            .block(hit.x, hit.y, hit.z)?
            .ok_or(BlockError::StaleHit)?;
        if current.id != hit.id || current.metadata != hit.metadata || current.id == 0 {
            return Err(BlockError::StaleHit);
        }
        if !supported.is_full_cube(current.id, current.metadata) {
            return Err(BlockError::HitNotEditable);
        }
        let (dx, dy, dz) = face_offset(hit.face).ok_or(BlockError::HitNotEditable)?;
        let x = hit.x.checked_add(dx).ok_or(BlockError::HitNotEditable)?;
        let y = hit.y.checked_add(dy).ok_or(BlockError::HitNotEditable)?;
        let z = hit.z.checked_add(dz).ok_or(BlockError::HitNotEditable)?;
        let target = self
            .block(x, y, z)?
            .ok_or(BlockError::TargetUnloaded { x, y, z })?;
        if target.id != 0 {
            return Err(BlockError::TargetOccupied);
        }
        let mut placed = target;
        placed.id = id;
        placed.metadata = metadata;
        self.set_block(x, y, z, placed)
    }

    fn set_block(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        value: BlockData,
    ) -> Result<BlockEdit, BlockError> {
        let (cx, lx) = split_block_coord(x);
        let (cz, lz) = split_block_coord(z);
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(BlockError::RevisionExhausted)?;
        let chunk = self
            .chunks
            .get_mut(&(cx, cz))
            .ok_or(BlockError::TargetUnloaded { x, y, z })?;
        chunk.set(
            lx,
            u8::try_from(y).map_err(|_| BlockError::HitNotEditable)?,
            lz,
            value,
        )?;
        self.revision = next_revision;
        let mut dirty = vec![(cx, cz)];
        // Fluid corner heights sample diagonal chunks too. Rebuild all chunks
        // touching the edited chunk corner, not only cardinal edge neighbors.
        for dx in chunk_neighbor_offsets(lx) {
            for dz in chunk_neighbor_offsets(lz) {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let key = (cx + dx, cz + dz);
                if self.chunks.contains_key(&key) && !dirty.contains(&key) {
                    dirty.push(key);
                }
            }
        }
        dirty.sort_unstable();
        Ok(BlockEdit {
            changed: (x, y, z),
            dirty_chunks: dirty,
            revision: self.revision,
        })
    }
}

fn finite(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

fn floor_i32(value: f64) -> Result<i32, BlockError> {
    let value = value.floor();
    if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        Err(BlockError::InvalidRay)
    } else {
        Ok(value as i32)
    }
}

fn sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn delta_t(direction: f64) -> f64 {
    if direction == 0.0 {
        f64::INFINITY
    } else {
        direction.abs().recip()
    }
}

/// Number of voxel-boundary crossings before the open segment endpoint.
fn axis_crossings(start: f64, end: f64) -> f64 {
    if end > start {
        let crossings = end.floor() - start.floor();
        if end.fract() == 0.0 {
            (crossings - 1.0).max(0.0)
        } else {
            crossings
        }
    } else if end < start {
        start.floor() - end.floor()
    } else {
        0.0
    }
}

fn first_t(origin: f64, cell: i32, step: i32, direction: f64) -> f64 {
    if step == 0 {
        f64::INFINITY
    } else {
        let plane = if step > 0 {
            f64::from(cell) + 1.0
        } else {
            f64::from(cell)
        };
        (plane - origin) / direction
    }
}

fn min_axis(values: [f64; 3]) -> usize {
    if values[0] <= values[1] && values[0] <= values[2] {
        0
    } else if values[1] <= values[2] {
        1
    } else {
        2
    }
}

fn face_offset(face: u8) -> Option<(i32, i32, i32)> {
    match face {
        0 => Some((0, -1, 0)),
        1 => Some((0, 1, 0)),
        2 => Some((0, 0, -1)),
        3 => Some((0, 0, 1)),
        4 => Some((-1, 0, 0)),
        5 => Some((1, 0, 0)),
        _ => None,
    }
}

fn chunk_neighbor_offsets(local: u8) -> impl Iterator<Item = i32> {
    let at_low_edge = local == 0;
    let at_high_edge = local == 15;
    [-1, 0, 1].into_iter().filter(move |offset| match offset {
        -1 => at_low_edge,
        1 => at_high_edge,
        _ => true,
    })
}

fn cell_intersects_xz_bounds(bounds: BlockSceneBounds, x: i32, z: i32) -> bool {
    let min_x = i64::from(bounds.min_chunk_x) * 16;
    let min_z = i64::from(bounds.min_chunk_z) * 16;
    let max_x = min_x + i64::from(bounds.width) * 16;
    let max_z = min_z + i64::from(bounds.depth) * 16;
    i64::from(x) >= min_x && i64::from(x) < max_x && i64::from(z) >= min_z && i64::from(z) < max_z
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBES: [bool; 256] = {
        let mut ids = [false; 256];
        ids[1] = true;
        ids[2] = true;
        ids
    };

    fn scene(bounds: BlockSceneBounds, chunks: Vec<BlockSceneChunk>) -> BlockScene {
        BlockScene::new(bounds, chunks).unwrap()
    }

    fn chunk_with(blocks: &[(u8, u8, u8, u8, u8)]) -> ChunkData {
        let mut data = ChunkData::new();
        for &(x, y, z, id, meta) in blocks {
            data.set(
                x,
                y,
                z,
                BlockData {
                    id,
                    metadata: meta,
                    block_light: 4,
                    sky_light: 7,
                },
            )
            .unwrap();
        }
        data
    }

    fn ray(start: Vec3, direction: Vec3, scene: &BlockScene) -> RayResult {
        scene
            .raycast(start, direction, &|id, _meta| CUBES[id as usize])
            .unwrap()
    }

    #[test]
    fn ray_hits_all_six_faces_and_reports_the_exact_full_cube_cell() {
        let cases = [
            (Vec3::new(0.5, 12.0, 0.5), Vec3::new(0.0, -1.0, 0.0), 1),
            (Vec3::new(0.5, 8.0, 0.5), Vec3::new(0.0, 1.0, 0.0), 0),
            (Vec3::new(0.5, 10.5, -2.0), Vec3::new(0.0, 0.0, 1.0), 2),
            (Vec3::new(0.5, 10.5, 3.0), Vec3::new(0.0, 0.0, -1.0), 3),
            (Vec3::new(-2.0, 10.5, 0.5), Vec3::new(1.0, 0.0, 0.0), 4),
            (Vec3::new(3.0, 10.5, 0.5), Vec3::new(-1.0, 0.0, 0.0), 5),
        ];
        for (origin, direction, face) in cases {
            let data = chunk_with(&[(0, 10, 0, 1, 0)]);
            let world = scene(
                BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
                vec![BlockSceneChunk { x: 0, z: 0, data }],
            );
            let RayResult::Hit(hit) = ray(origin, direction, &world) else {
                panic!("expected full cube hit")
            };
            assert_eq!(
                (hit.x, hit.y, hit.z, hit.face, hit.id, hit.metadata),
                (0, 10, 0, face, 1, 0)
            );
        }
    }

    #[test]
    fn native_c01_segments_match_faces_ties_open_endpoints_and_unsupported_slab() {
        let cubes = |id, _metadata| id == 1;
        let world_for = |x: i32, y: i32, z: i32, id: u8| {
            let (cx, lx) = split_block_coord(x);
            let (cz, lz) = split_block_coord(z);
            let data = chunk_with(&[(lx, y as u8, lz, id, 0)]);
            scene(
                BlockSceneBounds::new(cx, cz, 1, 1).unwrap(),
                vec![BlockSceneChunk { x: cx, z: cz, data }],
            )
        };
        let faces = [
            (
                Vec3::new(-2.0, 0.5, 0.5),
                Vec3::new(2.0, 0.5, 0.5),
                4,
                [0.0, 0.5, 0.5],
            ),
            (
                Vec3::new(2.0, 0.5, 0.5),
                Vec3::new(-2.0, 0.5, 0.5),
                5,
                [1.0, 0.5, 0.5],
            ),
            (
                Vec3::new(0.5, -2.0, 0.5),
                Vec3::new(0.5, 2.0, 0.5),
                0,
                [0.5, 0.0, 0.5],
            ),
            (
                Vec3::new(0.5, 2.0, 0.5),
                Vec3::new(0.5, -2.0, 0.5),
                1,
                [0.5, 1.0, 0.5],
            ),
            (
                Vec3::new(0.5, 0.5, -2.0),
                Vec3::new(0.5, 0.5, 2.0),
                2,
                [0.5, 0.5, 0.0],
            ),
            (
                Vec3::new(0.5, 0.5, 2.0),
                Vec3::new(0.5, 0.5, -2.0),
                3,
                [0.5, 0.5, 1.0],
            ),
        ];
        let world = world_for(0, 0, 0, 1);
        for (start, end, face, point) in faces {
            let RayResult::Hit(hit) = world.raycast_segment(start, end, &cubes).unwrap() else {
                panic!("expected native full-cube hit")
            };
            assert_eq!((hit.x, hit.y, hit.z, hit.face), (0, 0, 0, face));
            assert_eq!(
                [
                    hit.position.x.to_bits(),
                    hit.position.y.to_bits(),
                    hit.position.z.to_bits()
                ],
                point.map(f64::to_bits)
            );
        }

        assert_eq!(
            world.raycast_segment(Vec3::new(-2.0, 2.0, 0.5), Vec3::new(2.0, 2.0, 0.5), &cubes),
            Ok(RayResult::Miss)
        );

        let tie = world_for(0, 0, 0, 1);
        let RayResult::Hit(hit) = tie
            .raycast_segment(
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(2.0, 2.0, 2.0),
                &cubes,
            )
            .unwrap()
        else {
            panic!("expected native corner hit")
        };
        assert_eq!((hit.x, hit.y, hit.z, hit.face), (0, 0, 0, 4));
        assert_eq!(
            [
                hit.position.x.to_bits(),
                hit.position.y.to_bits(),
                hit.position.z.to_bits()
            ],
            [0_u64; 3]
        );

        let endpoint_world = world_for(0, 0, 0, 1);
        for endpoint in [-f64::EPSILON, 0.0] {
            assert_eq!(
                endpoint_world.raycast_segment(
                    Vec3::new(-2.0, 0.5, 0.5),
                    Vec3::new(endpoint, 0.5, 0.5),
                    &cubes
                ),
                Ok(RayResult::Miss)
            );
        }

        let negative = world_for(-1, 0, 0, 1);
        let RayResult::Hit(hit) = negative
            .raycast_segment(Vec3::new(-2.0, 0.5, 0.5), Vec3::new(2.0, 0.5, 0.5), &cubes)
            .unwrap()
        else {
            panic!("expected negative-coordinate native hit")
        };
        assert_eq!((hit.x, hit.y, hit.z, hit.face), (-1, 0, 0, 4));
        assert_eq!(hit.position.x.to_bits(), (-1.0_f64).to_bits());

        let shapes = |id, metadata| match (id, metadata) {
            (1, 0) => RayShape::FullCube,
            (44, 0) => RayShape::LowerSlab,
            (8, 0) => RayShape::Liquid,
            _ => RayShape::Unsupported,
        };
        let slab = world_for(0, 0, 0, 44);
        let RayResult::Hit(hit) = slab
            .raycast_segment_with_shapes(
                Vec3::new(-2.0, 0.25, 0.5),
                Vec3::new(2.0, 0.25, 0.5),
                false,
                &shapes,
            )
            .unwrap()
        else {
            panic!("expected native lower-slab hit")
        };
        assert_eq!(
            (hit.x, hit.y, hit.z, hit.face, hit.kind),
            (0, 0, 0, 4, BlockHitKind::LowerSlab)
        );
        assert_eq!(
            [
                hit.position.x.to_bits(),
                hit.position.y.to_bits(),
                hit.position.z.to_bits()
            ],
            [0.0_f64.to_bits(), 0.25_f64.to_bits(), 0.5_f64.to_bits()]
        );
        let mut slab_edits = slab.clone();
        assert_eq!(
            slab_edits.break_block(hit, &|_, _| true),
            Err(BlockError::HitNotEditable)
        );
        assert_eq!(slab_edits.revision(), 0);
        assert_eq!(
            slab.raycast_segment(
                Vec3::new(-2.0, 0.25, 0.5),
                Vec3::new(2.0, 0.25, 0.5),
                &cubes,
            ),
            Ok(RayResult::Unsupported(UnsupportedBlock {
                x: 0,
                y: 0,
                z: 0,
                id: 44,
                metadata: 0
            }))
        );

        let liquid = world_for(0, 0, 0, 8);
        assert_eq!(
            liquid.raycast_segment_with_shapes(
                Vec3::new(-2.0, 0.5, 0.5),
                Vec3::new(2.0, 0.5, 0.5),
                false,
                &shapes
            ),
            Ok(RayResult::Miss)
        );
        let RayResult::Hit(hit) = liquid
            .raycast_segment_with_shapes(
                Vec3::new(-2.0, 0.5, 0.5),
                Vec3::new(2.0, 0.5, 0.5),
                true,
                &shapes,
            )
            .unwrap()
        else {
            panic!("expected native stop-on-liquid hit")
        };
        assert_eq!(
            (hit.x, hit.y, hit.z, hit.face, hit.kind),
            (0, 0, 0, 4, BlockHitKind::Liquid)
        );
        assert_eq!(
            [
                hit.position.x.to_bits(),
                hit.position.y.to_bits(),
                hit.position.z.to_bits()
            ],
            [0.0_f64.to_bits(), 0.5_f64.to_bits(), 0.5_f64.to_bits()]
        );
        let mut liquid_edits = liquid.clone();
        assert_eq!(
            liquid_edits.place_block(hit, 1, 0, &|_, _| true),
            Err(BlockError::HitNotEditable)
        );
        assert_eq!(liquid_edits.revision(), 0);

        let range = world_for(5, 0, 0, 1);
        for endpoint in [4.999, 5.0] {
            assert_eq!(
                range.raycast_segment(
                    Vec3::new(0.0, 0.5, 0.5),
                    Vec3::new(endpoint, 0.5, 0.5),
                    &cubes
                ),
                Ok(RayResult::Miss)
            );
        }
        let Ok(RayResult::Hit(hit)) =
            range.raycast_segment(Vec3::new(0.0, 0.5, 0.5), Vec3::new(5.001, 0.5, 0.5), &cubes)
        else {
            panic!("expected hit beyond the exact five-block endpoint")
        };
        assert_eq!((hit.x, hit.y, hit.z, hit.face), (5, 0, 0, 4));
        assert_eq!(
            [
                hit.position.x.to_bits(),
                hit.position.y.to_bits(),
                hit.position.z.to_bits()
            ],
            [
                0x4014_0000_0000_0000,
                0x3fe0_0000_0000_0000,
                0x3fe0_0000_0000_0000
            ]
        );
    }

    #[test]
    fn segment_rejects_a_voxel_path_that_exceeds_the_cell_visit_budget() {
        let data = chunk_with(&[(11, 11, 11, 1, 0)]);
        let world = scene(
            BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk { x: 0, z: 0, data }],
        );
        assert_eq!(
            world.raycast_segment(
                Vec3::new(0.999, 0.999, 0.999),
                Vec3::new(11.5, 11.5, 11.5),
                &|id, _| id == 1
            ),
            Err(BlockError::RayStepLimit)
        );
    }

    #[test]
    fn ray_supports_negative_world_coordinates_and_rejects_unknown_shapes_explicitly() {
        let data = chunk_with(&[(15, 10, 0, 1, 0), (14, 10, 0, 3, 0)]);
        let world = scene(
            BlockSceneBounds::new(-1, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk { x: -1, z: 0, data }],
        );
        assert!(matches!(
            ray(Vec3::new(2.0, 10.5, 0.5), Vec3::new(-1.0, 0.0, 0.0), &world),
            RayResult::Hit(BlockHit { x: -1, .. })
        ));
        assert!(matches!(
            ray(
                Vec3::new(-1.5, 10.5, 0.5),
                Vec3::new(-1.0, 0.0, 0.0),
                &world
            ),
            RayResult::Unsupported(UnsupportedBlock { x: -2, .. })
        ));
    }

    #[test]
    fn ray_enforces_five_block_reach_ties_misses_and_unloaded_cells() {
        let data = chunk_with(&[(5, 10, 0, 1, 0)]);
        let world = scene(
            BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk { x: 0, z: 0, data }],
        );
        assert_eq!(
            ray(Vec3::new(0.0, 10.5, 0.5), Vec3::new(1.0, 0.0, 0.0), &world),
            RayResult::Miss
        );
        assert!(matches!(
            world.raycast_segment(
                Vec3::new(0.0, 10.5, 0.5),
                Vec3::new(5.001, 10.5, 0.5),
                &|id, _| CUBES[id as usize]
            ),
            Ok(RayResult::Hit(BlockHit { x: 5, face: 4, .. }))
        ));
        let beyond = chunk_with(&[(6, 10, 0, 1, 0)]);
        let world = scene(
            BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk {
                x: 0,
                z: 0,
                data: beyond,
            }],
        );
        assert_eq!(
            ray(Vec3::new(0.0, 10.5, 0.5), Vec3::new(1.0, 0.0, 0.0), &world),
            RayResult::Miss
        );
        assert_eq!(
            ray(Vec3::new(0.0, 10.5, 0.5), Vec3::new(0.0, 1.0, 0.0), &world),
            RayResult::Miss
        );
        assert!(matches!(
            ray(Vec3::new(15.5, 10.5, 0.5), Vec3::new(1.0, 0.0, 0.0), &world),
            RayResult::Miss
        ));
        let partial = scene(
            BlockSceneBounds::new(0, 0, 2, 1).unwrap(),
            vec![BlockSceneChunk {
                x: 0,
                z: 0,
                data: ChunkData::new(),
            }],
        );
        assert!(matches!(
            ray(
                Vec3::new(15.5, 10.5, 0.5),
                Vec3::new(1.0, 0.0, 0.0),
                &partial
            ),
            RayResult::Unloaded { x: 16, .. }
        ));
        assert!(matches!(
            ray(Vec3::new(0.5, 10.5, 0.5), Vec3::new(1.0, 1.0, 0.0), &world),
            RayResult::Miss | RayResult::Hit(_)
        ));
    }

    #[test]
    fn edits_are_atomic_validated_and_report_chunks_to_rebuild() {
        let data = chunk_with(&[(15, 10, 8, 1, 0)]);
        let neighbor = chunk_with(&[(0, 10, 8, 3, 0)]);
        let mut world = scene(
            BlockSceneBounds::new(0, 0, 2, 1).unwrap(),
            vec![
                BlockSceneChunk { x: 0, z: 0, data },
                BlockSceneChunk {
                    x: 1,
                    z: 0,
                    data: neighbor,
                },
            ],
        );
        let hit = BlockHit {
            x: 15,
            y: 10,
            z: 8,
            face: 5,
            position: Vec3::new(16.0, 10.5, 8.5),
            kind: BlockHitKind::FullCube,
            id: 1,
            metadata: 0,
        };
        assert_eq!(
            world.place_block(hit, 2, 1, &|id, _| CUBES[id as usize]),
            Err(BlockError::UnsupportedPlacement { id: 2, metadata: 1 })
        );
        assert_eq!(world.block(16, 10, 8).unwrap().unwrap().id, 3);
        assert_eq!(world.revision(), 0);
        assert_eq!(
            world.place_block(hit, 2, 0, &|id, _| CUBES[id as usize]),
            Err(BlockError::TargetOccupied)
        );
        assert_eq!(world.revision(), 0);
        let edit = world.break_block(hit, &|id, _| CUBES[id as usize]).unwrap();
        assert_eq!(edit.revision, 1);
        assert_eq!(edit.changed, (15, 10, 8));
        assert_eq!(edit.dirty_chunks, vec![(0, 0), (1, 0)]);
        let broken = world.block(15, 10, 8).unwrap().unwrap();
        assert_eq!(
            (
                broken.id,
                broken.metadata,
                broken.block_light,
                broken.sky_light
            ),
            (0, 0, 4, 7)
        );
        let other_data = chunk_with(&[(1, 10, 8, 1, 0)]);
        let mut place_world = scene(
            BlockSceneBounds::new(0, 0, 2, 1).unwrap(),
            vec![
                BlockSceneChunk {
                    x: 0,
                    z: 0,
                    data: ChunkData::new(),
                },
                BlockSceneChunk {
                    x: 1,
                    z: 0,
                    data: other_data,
                },
            ],
        );
        let support_hit = BlockHit {
            x: 17,
            y: 10,
            z: 8,
            face: 4,
            position: Vec3::new(17.0, 10.5, 8.5),
            kind: BlockHitKind::FullCube,
            id: 1,
            metadata: 0,
        };
        let edit = place_world
            .place_block(support_hit, 2, 0, &|id, _| CUBES[id as usize])
            .unwrap();
        assert_eq!(edit.revision, 1);
        assert_eq!(place_world.block(16, 10, 8).unwrap().unwrap().id, 2);
        assert_eq!(edit.dirty_chunks, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn corner_edits_mark_diagonal_chunk_dirty_for_neighbor_sampling() {
        let bounds = BlockSceneBounds::new(0, 0, 2, 2).unwrap();
        let mut world = scene(
            bounds,
            vec![
                BlockSceneChunk {
                    x: 0,
                    z: 0,
                    data: chunk_with(&[(15, 10, 15, 1, 0)]),
                },
                BlockSceneChunk {
                    x: 1,
                    z: 0,
                    data: ChunkData::new(),
                },
                BlockSceneChunk {
                    x: 0,
                    z: 1,
                    data: ChunkData::new(),
                },
                BlockSceneChunk {
                    x: 1,
                    z: 1,
                    data: ChunkData::new(),
                },
            ],
        );
        let hit = BlockHit {
            x: 15,
            y: 10,
            z: 15,
            face: 1,
            position: Vec3::new(15.5, 11.0, 15.5),
            kind: BlockHitKind::FullCube,
            id: 1,
            metadata: 0,
        };
        let edit = world.break_block(hit, &|_, _| true).unwrap();
        assert_eq!(edit.dirty_chunks, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    }

    #[test]
    fn three_sequential_breaks_keep_each_in_memory_edit_and_advance_revision() {
        let mut world = scene(
            BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![BlockSceneChunk {
                x: 0,
                z: 0,
                data: chunk_with(&[(2, 10, 2, 1, 0), (3, 10, 2, 1, 0), (4, 10, 2, 1, 0)]),
            }],
        );
        for (revision, x) in [(1, 2), (2, 3), (3, 4)] {
            let hit = BlockHit {
                x,
                y: 10,
                z: 2,
                face: 1,
                position: Vec3::new(f64::from(x) + 0.5, 11.0, 2.5),
                kind: BlockHitKind::FullCube,
                id: 1,
                metadata: 0,
            };
            let edit = world.break_block(hit, &|_, _| true).unwrap();
            assert_eq!(edit.revision, revision);
            assert_eq!(world.block(x, 10, 2).unwrap().unwrap().id, 0);
        }
        assert_eq!(world.revision(), 3);
    }

    #[test]
    fn placement_rejects_unloaded_and_stale_hits_without_partial_mutation() {
        let data = chunk_with(&[(15, 10, 8, 1, 0)]);
        let mut world = scene(
            BlockSceneBounds::new(0, 0, 2, 1).unwrap(),
            vec![BlockSceneChunk { x: 0, z: 0, data }],
        );
        let hit = BlockHit {
            x: 15,
            y: 10,
            z: 8,
            face: 5,
            position: Vec3::new(16.0, 10.5, 8.5),
            kind: BlockHitKind::FullCube,
            id: 1,
            metadata: 0,
        };
        assert!(matches!(
            world.place_block(hit, 2, 0, &|id, _| CUBES[id as usize]),
            Err(BlockError::TargetUnloaded { x: 16, .. })
        ));
        assert_eq!(world.revision(), 0);
        let stale = BlockHit { id: 2, ..hit };
        assert_eq!(
            world.break_block(stale, &|id, _| CUBES[id as usize]),
            Err(BlockError::StaleHit)
        );
        assert_eq!(world.revision(), 0);
    }

    #[test]
    fn construction_and_rays_reject_invalid_bounds_and_nonfinite_vectors() {
        assert_eq!(
            BlockSceneBounds::new(0, 0, 6, 1),
            Err(BlockError::InvalidBounds)
        );
        let world = scene(BlockSceneBounds::new(0, 0, 1, 1).unwrap(), vec![]);
        assert_eq!(
            world.raycast(
                Vec3::new(f64::NAN, 1.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
                &|_, _| true
            ),
            Err(BlockError::InvalidRay)
        );
        assert_eq!(
            world.raycast(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                &|_, _| true
            ),
            Err(BlockError::InvalidRay)
        );
    }

    #[test]
    fn relight_queue_limit_is_atomic_and_reports_sorted_dirty_state() {
        let bounds = BlockSceneBounds::new(0, 0, 1, 1).unwrap();
        let mut world = scene(
            bounds,
            vec![BlockSceneChunk {
                x: 0,
                z: 0,
                data: ChunkData::new(),
            }],
        );
        let before = world.clone();
        assert_eq!(
            world.relight((0, 64, 0), 1, |id, _| (if id == 0 { 0 } else { 15 }, 0)),
            Err(RelightError::QueueLimitExceeded { limit: 1 })
        );
        assert_eq!(world.revision(), before.revision());
        assert_eq!(world.block(0, 64, 0), before.block(0, 64, 0));
        assert!(matches!(
            world.relight((0, 64, 0), 0, |_, _| (0, 0)),
            Err(RelightError::InvalidQueueLimit)
        ));
    }
}
