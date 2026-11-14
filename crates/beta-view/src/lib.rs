#![forbid(unsafe_code)]

//! Bounded CPU scene, camera, and standard-cube meshing for the Beta 1.7.3 viewer.
//!
//! This crate deliberately does not load files or create a window/GPU device. Callers
//! provide already-validated chunks and mappings produced by the native view oracle.
//! Mesh vertices retain the adjacent block's sky/block light nibbles for each full-cube
//! face and native directional face shade as separate values. This follows the sample
//! position used by the native flat renderer. The viewer applies saved-time scalar
//! brightness to these saved light samples; ambient-occlusion corner sampling remains
//! outside this flat-rendering scope.

use std::{collections::BTreeMap, error::Error, fmt};

use beta_core::{
    chunk::{ChunkData, ChunkError},
    geometry::Aabb,
    motion::{MAX_WATER_MOTION_CELLS, WaterMotionCell, WaterMotionError, WaterMotionPos},
};
use glam::{Mat4, Vec3, Vec4};

pub mod fluid;
mod plant;
pub use plant::tall_grass_biome_coords;
mod farmland;
mod ladder;
mod slab;
mod snow;
mod torch;

pub const CHUNK_WIDTH: usize = 16;
pub const WORLD_HEIGHT: usize = 128;
pub const MAX_SCENE_CHUNKS: usize = 25;
pub const MAX_SCENE_BLOCKS: usize = MAX_SCENE_CHUNKS * CHUNK_WIDTH * CHUNK_WIDTH * WORLD_HEIGHT;
pub const MAX_MESH_VERTICES: usize = 5_000_000;
pub const MAX_MESH_INDICES: usize = 7_500_000;
pub const ATLAS_SIZE: f32 = 256.0;
pub const TILE_SIZE: f32 = 16.0;
pub const TILE_INSET: f32 = 0.01;

/// A tile lookup is keyed by the optional block ID immediately above the state.
/// `None` means that the original method is context-independent; `Some(id)` is an
/// exact world-aware mapping case from the native corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockStateKey {
    pub id: u8,
    pub metadata: u8,
    pub above_id: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMapping {
    pub key: BlockStateKey,
    pub render_type: u8,
    pub opaque: bool,
    pub full_cube: bool,
    pub alpha_opaque_faces: [bool; 6],
    pub tiles: [u16; 6],
    /// The native adapter's supported flag includes render type, bounds, and texture.
    pub supported: bool,
}

impl BlockMapping {
    #[must_use]
    pub fn is_renderable_cube(&self) -> bool {
        self.supported
            && self.render_type == 0
            && self.opaque
            && self.full_cube
            && self.alpha_opaque_faces.into_iter().all(|opaque| opaque)
            && self.tiles.into_iter().all(|tile| tile < 256)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneChunk {
    pub x: i32,
    pub z: i32,
    pub data: ChunkData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneBounds {
    pub min_chunk_x: i32,
    pub min_chunk_z: i32,
    pub width: u8,
    pub depth: u8,
}

impl SceneBounds {
    pub fn new(
        min_chunk_x: i32,
        min_chunk_z: i32,
        width: u8,
        depth: u8,
    ) -> Result<Self, ViewError> {
        if width == 0 || depth == 0 || width > 5 || depth > 5 {
            return Err(ViewError::InvalidSceneBounds);
        }
        min_chunk_x
            .checked_add(i32::from(width) - 1)
            .ok_or(ViewError::InvalidSceneBounds)?;
        min_chunk_z
            .checked_add(i32::from(depth) - 1)
            .ok_or(ViewError::InvalidSceneBounds)?;
        let max_x = (i64::from(min_chunk_x) + i64::from(width) - 1) * 16 + 15;
        let max_z = (i64::from(min_chunk_z) + i64::from(depth) - 1) * 16 + 15;
        if i64::from(min_chunk_x) * 16 < i64::from(i32::MIN)
            || i64::from(min_chunk_z) * 16 < i64::from(i32::MIN)
            || max_x > i64::from(i32::MAX)
            || max_z > i64::from(i32::MAX)
        {
            return Err(ViewError::InvalidSceneBounds);
        }
        Ok(Self {
            min_chunk_x,
            min_chunk_z,
            width,
            depth,
        })
    }

    #[must_use]
    pub fn chunk_count(self) -> usize {
        usize::from(self.width) * usize::from(self.depth)
    }

    fn contains(self, x: i32, z: i32) -> bool {
        let x = i64::from(x);
        let z = i64::from(z);
        x >= i64::from(self.min_chunk_x)
            && z >= i64::from(self.min_chunk_z)
            && x < i64::from(self.min_chunk_x) + i64::from(self.width)
            && z < i64::from(self.min_chunk_z) + i64::from(self.depth)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    InvalidSceneBounds,
    InvalidColliderQuery,
    TooManyWaterCells(usize),
    WaterMotion(WaterMotionError),
    NoSafeSpawn,
    TooManyChunks(usize),
    ChunkOutsideScene { x: i32, z: i32 },
    DuplicateChunk { x: i32, z: i32 },
    DuplicateMapping(BlockStateKey),
    InvalidMetadata(u8),
    InvalidTile(u16),
    InvalidFace(u8),
    MeshLimitExceeded { vertices: usize, indices: usize },
    IndexOverflow,
    InvalidCamera,
    UnsupportedFluid(u8),
    Chunk(ChunkError),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSceneBounds => write!(
                f,
                "scene bounds must be a nonempty rectangle no larger than 5x5 chunks"
            ),
            Self::InvalidColliderQuery => write!(
                f,
                "collider query must be finite and at most 32 blocks per axis"
            ),
            Self::TooManyWaterCells(count) => write!(
                f,
                "water query found {count} cells; limit is {MAX_WATER_MOTION_CELLS}"
            ),
            Self::WaterMotion(error) => write!(f, "water travel: {error:?}"),
            Self::NoSafeSpawn => write!(
                f,
                "scene contains no confirmed full-cube floor with two clear blocks"
            ),
            Self::TooManyChunks(count) => {
                write!(f, "scene has {count} chunks; limit is {MAX_SCENE_CHUNKS}")
            }
            Self::ChunkOutsideScene { x, z } => {
                write!(f, "chunk ({x}, {z}) is outside scene bounds")
            }
            Self::DuplicateChunk { x, z } => write!(f, "duplicate chunk ({x}, {z})"),
            Self::DuplicateMapping(key) => write!(f, "duplicate native mapping for {:?}", key),
            Self::InvalidMetadata(value) => {
                write!(f, "metadata {value} exceeds Beta nibble range 0..=15")
            }
            Self::InvalidTile(value) => write!(f, "atlas tile {value} exceeds 0..=255"),
            Self::InvalidFace(value) => write!(f, "face {value} exceeds 0..=5"),
            Self::MeshLimitExceeded { vertices, indices } => write!(
                f,
                "mesh limit exceeded ({vertices} vertices, {indices} indices)"
            ),
            Self::IndexOverflow => write!(f, "mesh vertex index exceeds u32"),
            Self::InvalidCamera => write!(f, "camera parameters are invalid"),
            Self::UnsupportedFluid(id) => write!(f, "block {id} is not a supported fluid"),
            Self::Chunk(error) => write!(f, "chunk access: {error}"),
        }
    }
}
impl Error for ViewError {}
impl From<ChunkError> for ViewError {
    fn from(value: ChunkError) -> Self {
        Self::Chunk(value)
    }
}

/// Fixed, immutable rectangle of at most 25 already-loaded chunks. Missing slots are air.
#[derive(Debug, Clone)]
pub struct Scene {
    bounds: SceneBounds,
    chunks: BTreeMap<(i32, i32), ChunkData>,
    mappings: BTreeMap<BlockStateKey, BlockMapping>,
    fancy_leaves: bool,
}

impl Scene {
    pub fn new(
        bounds: SceneBounds,
        chunks: Vec<SceneChunk>,
        mappings: Vec<BlockMapping>,
    ) -> Result<Self, ViewError> {
        let bounds = SceneBounds::new(
            bounds.min_chunk_x,
            bounds.min_chunk_z,
            bounds.width,
            bounds.depth,
        )?;
        if chunks.len() > MAX_SCENE_CHUNKS {
            return Err(ViewError::TooManyChunks(chunks.len()));
        }
        let mut chunk_map = BTreeMap::new();
        for chunk in chunks {
            if !bounds.contains(chunk.x, chunk.z) {
                return Err(ViewError::ChunkOutsideScene {
                    x: chunk.x,
                    z: chunk.z,
                });
            }
            if chunk_map.insert((chunk.x, chunk.z), chunk.data).is_some() {
                return Err(ViewError::DuplicateChunk {
                    x: chunk.x,
                    z: chunk.z,
                });
            }
        }
        let mut mapping_map = BTreeMap::new();
        for mapping in mappings {
            if mapping.key.metadata > 15 {
                return Err(ViewError::InvalidMetadata(mapping.key.metadata));
            }
            if let Some(tile) = mapping.tiles.iter().find(|tile| **tile >= 256) {
                return Err(ViewError::InvalidTile(*tile));
            }
            if mapping_map.insert(mapping.key, mapping.clone()).is_some() {
                return Err(ViewError::DuplicateMapping(mapping.key));
            }
        }
        Ok(Self {
            bounds,
            chunks: chunk_map,
            mappings: mapping_map,
            fancy_leaves: false,
        })
    }

    #[must_use]
    pub fn bounds(&self) -> SceneBounds {
        self.bounds
    }
    #[must_use]
    pub fn loaded_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// Replaces a loaded chunk in place so a block edit need not clone the full scene.
    pub fn replace_chunk(&mut self, chunk: SceneChunk) -> Result<(), ViewError> {
        if !self.bounds.contains(chunk.x, chunk.z) {
            return Err(ViewError::ChunkOutsideScene {
                x: chunk.x,
                z: chunk.z,
            });
        }
        let Some(data) = self.chunks.get_mut(&(chunk.x, chunk.z)) else {
            return Err(ViewError::ChunkOutsideScene {
                x: chunk.x,
                z: chunk.z,
            });
        };
        *data = chunk.data;
        Ok(())
    }

    /// Builds a separate, bounded transparent mesh for water blocks in the scene.
    /// The ordinary cube mesh remains unchanged and continues to own the opaque pass.
    pub fn fluid_mesh(&self, limits: MeshLimits) -> Result<fluid::Mesh, ViewError> {
        self.fluid_mesh_selected(limits, None)
    }

    /// Builds water geometry only for selected chunks while sampling neighbors
    /// from the complete scene.
    pub fn fluid_mesh_chunks(
        &self,
        limits: MeshLimits,
        chunks: &[(i32, i32)],
    ) -> Result<fluid::Mesh, ViewError> {
        self.fluid_mesh_selected(limits, Some(chunks))
    }

    fn fluid_mesh_selected(
        &self,
        limits: MeshLimits,
        selected_chunks: Option<&[(i32, i32)]>,
    ) -> Result<fluid::Mesh, ViewError> {
        let mut mesh = fluid::Mesh {
            vertices: Vec::new(),
        };
        let vertex_limit = limits.max_vertices.min(MAX_MESH_VERTICES);
        let x0 = self
            .bounds
            .min_chunk_x
            .checked_mul(16)
            .ok_or(ViewError::InvalidSceneBounds)?;
        let z0 = self
            .bounds
            .min_chunk_z
            .checked_mul(16)
            .ok_or(ViewError::InvalidSceneBounds)?;
        for chunk_z in 0..self.bounds.depth {
            for chunk_x in 0..self.bounds.width {
                let cx = self.bounds.min_chunk_x + i32::from(chunk_x);
                let cz = self.bounds.min_chunk_z + i32::from(chunk_z);
                if selected_chunks.is_some_and(|selected| !selected.contains(&(cx, cz))) {
                    continue;
                }
                let Some(chunk) = self.chunks.get(&(cx, cz)) else {
                    continue;
                };
                for local_x in 0..16_u8 {
                    for local_z in 0..16_u8 {
                        for y in 0..128_u8 {
                            let block = chunk.get(local_x, y, local_z)?;
                            if block.id != 8 && block.id != 9 {
                                continue;
                            }
                            let x = x0 + i32::from(chunk_x) * 16 + i32::from(local_x);
                            let z = z0 + i32::from(chunk_z) * 16 + i32::from(local_z);
                            let cell = |dx: i32, dy: i32, dz: i32| -> Result<Option<fluid::FluidCell>, ViewError> {
                                let b = self.block(x + dx, i32::from(y) + dy, z + dz)?;
                                Ok((b.id != 0).then_some(fluid::FluidCell { id: b.id, metadata: b.metadata }))
                            };
                            let neighborhood = fluid::FluidNeighborhood {
                                center: fluid::FluidCell {
                                    id: block.id,
                                    metadata: block.metadata,
                                },
                                below: cell(0, -1, 0)?,
                                above: cell(0, 1, 0)?,
                                east: cell(1, 0, 0)?,
                                west: cell(-1, 0, 0)?,
                                north: cell(0, 0, -1)?,
                                south: cell(0, 0, 1)?,
                                north_east: cell(1, 0, -1)?,
                                north_west: cell(-1, 0, -1)?,
                                south_east: cell(1, 0, 1)?,
                                south_west: cell(-1, 0, 1)?,
                            };
                            let water = fluid::mesh(x, i32::from(y), z, neighborhood)?;
                            if mesh.vertices.len().saturating_add(water.vertices.len())
                                > vertex_limit
                            {
                                return Err(ViewError::MeshLimitExceeded {
                                    vertices: mesh.vertices.len() + water.vertices.len(),
                                    indices: mesh.vertices.len() + water.vertices.len(),
                                });
                            }
                            mesh.vertices.extend(water.vertices);
                        }
                    }
                }
            }
        }
        Ok(mesh)
    }

    /// Enables native fancy leaf cube rendering and full-cube leaf collision.
    pub fn enable_fancy_leaves(&mut self) {
        self.fancy_leaves = true;
    }

    /// Returns confirmed opaque full-cube obstacles overlapping a bounded local
    /// query. Missing chunks and unsupported block shapes are treated as air.
    pub fn colliders_in(&self, area: Aabb) -> Result<Vec<Aabb>, ViewError> {
        if (0..3).any(|axis| {
            !area.min[axis].is_finite()
                || !area.max[axis].is_finite()
                || area.min[axis] > area.max[axis]
        }) {
            return Err(ViewError::InvalidColliderQuery);
        }
        let world_min_x = i64::from(self.bounds.min_chunk_x) * 16;
        let world_min_z = i64::from(self.bounds.min_chunk_z) * 16;
        let world_max_x = world_min_x + i64::from(self.bounds.width) * 16;
        let world_max_z = world_min_z + i64::from(self.bounds.depth) * 16;
        let x0 = (area.min[0].floor() as i64).max(world_min_x);
        let x1 = (area.max[0].ceil() as i64).min(world_max_x);
        let y0 = (area.min[1].floor() as i32).clamp(0, WORLD_HEIGHT as i32);
        let y1 = (area.max[1].ceil() as i32).clamp(0, WORLD_HEIGHT as i32);
        let z0 = (area.min[2].floor() as i64).max(world_min_z);
        let z1 = (area.max[2].ceil() as i64).min(world_max_z);
        if x1.saturating_sub(x0) > 32 || y1.saturating_sub(y0) > 32 || z1.saturating_sub(z0) > 32 {
            return Err(ViewError::InvalidColliderQuery);
        }
        let mut result = Vec::new();
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    let block = self.block(x as i32, y, z as i32)?;
                    if block.id == 0 {
                        continue;
                    }
                    let above = self.block(x as i32, y + 1, z as i32)?.id;
                    if self.is_collision_cube(block.id, block.metadata, above) {
                        result.push(Aabb::new(
                            [x as f64, y as f64, z as f64],
                            [(x + 1) as f64, (y + 1) as f64, (z + 1) as f64],
                        ));
                    }
                }
            }
        }
        Ok(result)
    }

    /// Returns only native water IDs in a small loaded-scene query region.
    /// Missing chunks and blocks outside build height are treated as air, as in
    /// the existing bounded collision query.
    pub fn water_cells_in(
        &self,
        area: Aabb,
    ) -> Result<BTreeMap<WaterMotionPos, WaterMotionCell>, ViewError> {
        if (0..3).any(|axis| {
            !area.min[axis].is_finite()
                || !area.max[axis].is_finite()
                || area.min[axis] > area.max[axis]
        }) {
            return Err(ViewError::InvalidColliderQuery);
        }
        let world_min_x = i64::from(self.bounds.min_chunk_x) * 16;
        let world_min_z = i64::from(self.bounds.min_chunk_z) * 16;
        let world_max_x = world_min_x + i64::from(self.bounds.width) * 16;
        let world_max_z = world_min_z + i64::from(self.bounds.depth) * 16;
        let x0 = (area.min[0].floor() as i64).max(world_min_x);
        let x1 = (area.max[0].ceil() as i64).min(world_max_x);
        let y0 = (area.min[1].floor() as i32).clamp(0, WORLD_HEIGHT as i32);
        let y1 = (area.max[1].ceil() as i32).clamp(0, WORLD_HEIGHT as i32);
        let z0 = (area.min[2].floor() as i64).max(world_min_z);
        let z1 = (area.max[2].ceil() as i64).min(world_max_z);
        if x1.saturating_sub(x0) > 32 || y1.saturating_sub(y0) > 32 || z1.saturating_sub(z0) > 32 {
            return Err(ViewError::InvalidColliderQuery);
        }
        let mut cells = BTreeMap::new();
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    let block = self.block(x as i32, y, z as i32)?;
                    if matches!(block.id, 8 | 9) {
                        cells.insert(
                            (x as i32, y, z as i32),
                            WaterMotionCell {
                                id: block.id,
                                metadata: block.metadata,
                            },
                        );
                        if cells.len() > MAX_WATER_MOTION_CELLS {
                            return Err(ViewError::TooManyWaterCells(cells.len()));
                        }
                    }
                }
            }
        }
        Ok(cells)
    }

    /// Finds a nearby column with a confirmed solid floor and two clear blocks.
    pub fn safe_spawn_near(&self, spawn: [i32; 3]) -> Result<[f64; 3], ViewError> {
        let min_x = i64::from(self.bounds.min_chunk_x) * 16;
        let min_z = i64::from(self.bounds.min_chunk_z) * 16;
        let max_x = min_x + i64::from(self.bounds.width) * 16 - 1;
        let max_z = min_z + i64::from(self.bounds.depth) * 16 - 1;
        let center_x = i64::from(spawn[0]).clamp(min_x, max_x);
        let center_z = i64::from(spawn[2]).clamp(min_z, max_z);
        for radius in 0_i32..=16 {
            for dx in -radius..=radius {
                let dz_abs = radius - dx.abs();
                for dz in if dz_abs == 0 {
                    vec![0]
                } else {
                    vec![-dz_abs, dz_abs]
                } {
                    let x = center_x + i64::from(dx);
                    let z = center_z + i64::from(dz);
                    if x < min_x || x > max_x || z < min_z || z > max_z {
                        continue;
                    }
                    let start_y = spawn[1].clamp(1, WORLD_HEIGHT as i32 - 3);
                    for y in (1..=start_y)
                        .rev()
                        .chain(start_y + 1..WORLD_HEIGHT as i32 - 2)
                    {
                        let floor = self.block(x as i32, y - 1, z as i32)?;
                        if floor.id == 0 {
                            continue;
                        }
                        let above = self.block(x as i32, y, z as i32)?;
                        let head = self.block(x as i32, y + 1, z as i32)?;
                        if above.id == 0 && head.id == 0 {
                            let floor_above = above.id;
                            if self.is_collision_cube(floor.id, floor.metadata, floor_above) {
                                return Ok([x as f64 + 0.5, y as f64, z as f64 + 0.5]);
                            }
                        }
                    }
                }
            }
        }
        Err(ViewError::NoSafeSpawn)
    }

    pub fn surface_below(&self, body: Aabb) -> Result<beta_core::motion::Surface, ViewError> {
        let x = ((body.min[0] + body.max[0]) * 0.5).floor() as i32;
        let z = ((body.min[2] + body.max[2]) * 0.5).floor() as i32;
        let Some(y) = (body.min[1].floor() as i64).checked_sub(1) else {
            return Ok(beta_core::motion::Surface::Air);
        };
        if !(0..WORLD_HEIGHT as i64).contains(&y) {
            return Ok(beta_core::motion::Surface::Air);
        }
        let y = y as i32;
        let block = self.block(x, y, z)?;
        let above = self.block(x, y + 1, z)?.id;
        if block.id == 0 || !self.is_collision_cube(block.id, block.metadata, above) {
            Ok(beta_core::motion::Surface::Air)
        } else if block.id == 79 {
            Ok(beta_core::motion::Surface::Ice)
        } else {
            Ok(beta_core::motion::Surface::Stone)
        }
    }

    /// Keeps the player body inside the loaded rectangular scene and build height.
    /// This viewer safety bound is separate from native block collision.
    pub fn constrain_motion_state(&self, state: &mut beta_core::motion::MotionState) {
        let min_x = f64::from(self.bounds.min_chunk_x * 16);
        let min_z = f64::from(self.bounds.min_chunk_z * 16);
        let max_x = min_x + f64::from(self.bounds.width) * 16.0;
        let max_z = min_z + f64::from(self.bounds.depth) * 16.0;
        let half_x = (state.bbox.max[0] - state.bbox.min[0]) * 0.5;
        let half_z = (state.bbox.max[2] - state.bbox.min[2]) * 0.5;
        let mut hit_horizontal = false;
        for (axis, lower, upper, half) in [(0, min_x, max_x, half_x), (2, min_z, max_z, half_z)] {
            let lower = lower + half;
            let upper = upper - half;
            let center = (state.bbox.min[axis] + state.bbox.max[axis]) * 0.5;
            let clamped = center.clamp(lower, upper);
            if clamped != center {
                let delta = clamped - center;
                state.bbox.min[axis] += delta;
                state.bbox.max[axis] += delta;
                state.position[axis] += delta;
                state.velocity[axis] = 0.0;
                hit_horizontal = true;
            }
        }
        if hit_horizontal {
            state.collided_horizontally = true;
            state.collided = true;
        }
        let lower_y = 0.0;
        let upper_y = WORLD_HEIGHT as f64 - (state.bbox.max[1] - state.bbox.min[1]);
        let clamped_y = state.bbox.min[1].clamp(lower_y, upper_y);
        if clamped_y != state.bbox.min[1] {
            let delta = clamped_y - state.bbox.min[1];
            state.bbox.min[1] += delta;
            state.bbox.max[1] += delta;
            state.position[1] += delta;
            state.velocity[1] = 0.0;
            state.collided_vertically = true;
            state.collided = true;
            state.on_ground = delta > 0.0;
        }
    }

    /// Returns a block from the bounded loaded scene; absent chunks and y outside
    /// the Beta build range are treated as air, consistent with rendering queries.
    pub fn block_at(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<beta_core::chunk::BlockData, ViewError> {
        self.block(x, y, z)
    }

    fn block(&self, x: i32, y: i32, z: i32) -> Result<beta_core::chunk::BlockData, ViewError> {
        if !(0..WORLD_HEIGHT as i32).contains(&y) {
            return Ok(Default::default());
        }
        let chunk_x = x >> 4;
        let chunk_z = z >> 4;
        let Some(data) = self.chunks.get(&(chunk_x, chunk_z)) else {
            return Ok(Default::default());
        };
        Ok(data.get((x & 15) as u8, y as u8, (z & 15) as u8)?)
    }

    fn mapping_for(&self, id: u8, metadata: u8, above_id: u8) -> Option<&BlockMapping> {
        self.mappings
            .get(&BlockStateKey {
                id,
                metadata,
                above_id: Some(above_id),
            })
            .or_else(|| {
                self.mappings.get(&BlockStateKey {
                    id,
                    metadata,
                    above_id: None,
                })
            })
    }

    fn is_supported_cube(&self, id: u8, metadata: u8, above_id: u8) -> bool {
        self.mapping_for(id, metadata, above_id)
            .is_some_and(BlockMapping::is_renderable_cube)
    }

    fn is_collision_cube(&self, id: u8, metadata: u8, above_id: u8) -> bool {
        (self.fancy_leaves && id == 18) || self.is_supported_cube(id, metadata, above_id)
    }

    /// Deterministic face mesh in world block coordinates. Unsupported states are omitted
    /// and counted once per occupied block. Adjacent supported full cubes share no face.
    pub fn mesh(&self, limits: MeshLimits) -> Result<Mesh, ViewError> {
        self.mesh_impl(limits, None, None, None)
    }

    /// Builds the extended render mesh using exact palette colors supplied by the caller.
    /// The callback receives block ID, metadata, and signed world X/Z; this crate does not
    /// compute climate, load palettes, or perform I/O.
    pub fn mesh_with_tint<F>(&self, limits: MeshLimits, mut tint: F) -> Result<Mesh, ViewError>
    where
        F: FnMut(u8, u8, i32, i32) -> u32,
    {
        self.mesh_impl(limits, Some(&mut tint), None, None)
    }

    /// Builds a tinted render mesh and carries a source-block emission minimum on every
    /// face vertex, including grass overlays. The legacy mesh helpers retain minimum 0.
    pub fn mesh_with_tint_and_emission<F, G>(
        &self,
        limits: MeshLimits,
        mut tint: F,
        mut emission_minimum: G,
    ) -> Result<Mesh, ViewError>
    where
        F: FnMut(u8, u8, i32, i32) -> u32,
        G: FnMut(u8) -> u8,
    {
        self.mesh_impl(limits, Some(&mut tint), Some(&mut emission_minimum), None)
    }

    /// Builds only selected chunk meshes while retaining the complete scene for
    /// neighbor visibility and fluid corner queries.
    pub fn mesh_chunks_with_tint_and_emission<F, G>(
        &self,
        limits: MeshLimits,
        chunks: &[(i32, i32)],
        mut tint: F,
        mut emission_minimum: G,
    ) -> Result<Mesh, ViewError>
    where
        F: FnMut(u8, u8, i32, i32) -> u32,
        G: FnMut(u8) -> u8,
    {
        self.mesh_impl(
            limits,
            Some(&mut tint),
            Some(&mut emission_minimum),
            Some(chunks),
        )
    }

    fn mesh_impl(
        &self,
        limits: MeshLimits,
        mut tint: Option<&mut dyn FnMut(u8, u8, i32, i32) -> u32>,
        mut emission_minimum: Option<&mut dyn FnMut(u8) -> u8>,
        only_chunks: Option<&[(i32, i32)]>,
    ) -> Result<Mesh, ViewError> {
        let mut mesh = Mesh::default();
        let vertex_limit = limits.max_vertices.min(MAX_MESH_VERTICES);
        let index_limit = limits.max_indices.min(MAX_MESH_INDICES);
        let x0 = self
            .bounds
            .min_chunk_x
            .checked_mul(16)
            .ok_or(ViewError::InvalidSceneBounds)?;
        let z0 = self
            .bounds
            .min_chunk_z
            .checked_mul(16)
            .ok_or(ViewError::InvalidSceneBounds)?;
        for chunk_z in 0..self.bounds.depth {
            for chunk_x in 0..self.bounds.width {
                let cx = self.bounds.min_chunk_x + i32::from(chunk_x);
                let cz = self.bounds.min_chunk_z + i32::from(chunk_z);
                if only_chunks.is_some_and(|selected| !selected.contains(&(cx, cz))) {
                    continue;
                }
                let Some(chunk) = self.chunks.get(&(cx, cz)) else {
                    continue;
                };
                for local_x in 0..16_u8 {
                    for local_z in 0..16_u8 {
                        for y in 0..128_u8 {
                            let block = chunk.get(local_x, y, local_z)?;
                            if block.id == 0 {
                                continue;
                            }
                            if block.id == 8 || block.id == 9 {
                                continue;
                            }
                            let wx = x0 + i32::from(chunk_x) * 16 + i32::from(local_x);
                            let wz = z0 + i32::from(chunk_z) * 16 + i32::from(local_z);
                            if block.id == 65 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                ladder::append(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    block,
                                    source_minimum,
                                    limits,
                                )?;
                                continue;
                            }
                            if block.id == 20 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                for face in 0..6_u8 {
                                    let (dx, dy, dz) = face_delta(face)?;
                                    let neighbor =
                                        self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                    let above =
                                        self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                    if !glass_face_visible(
                                        neighbor.id,
                                        self.is_supported_cube(
                                            neighbor.id,
                                            neighbor.metadata,
                                            above,
                                        ),
                                    ) {
                                        continue;
                                    }
                                    if mesh.vertices.len().saturating_add(4) > vertex_limit
                                        || mesh.indices.len().saturating_add(6) > index_limit
                                    {
                                        return Err(ViewError::MeshLimitExceeded {
                                            vertices: mesh.vertices.len() + 4,
                                            indices: mesh.indices.len() + 6,
                                        });
                                    }
                                    append_face_tinted(
                                        &mut mesh,
                                        [wx, i32::from(y), wz],
                                        face,
                                        49,
                                        neighbor.block_light,
                                        neighbor.sky_light,
                                        [1.0; 3],
                                        source_minimum,
                                        false,
                                    )?;
                                }
                                continue;
                            }
                            if block.id == 60 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                for face in 0..6_u8 {
                                    let (dx, dy, dz) = face_delta(face)?;
                                    let neighbor =
                                        self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                    let neighbor_above =
                                        self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                    if !farmland::is_visible(
                                        face,
                                        self.is_supported_cube(
                                            neighbor.id,
                                            neighbor.metadata,
                                            neighbor_above,
                                        ),
                                    ) {
                                        continue;
                                    }
                                    farmland::append_face(
                                        &mut mesh,
                                        [wx, i32::from(y), wz],
                                        block,
                                        neighbor,
                                        face,
                                        source_minimum,
                                        limits,
                                    )?;
                                }
                                continue;
                            }
                            if block.id == 78 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                for face in 0..6_u8 {
                                    let (dx, dy, dz) = face_delta(face)?;
                                    let neighbor =
                                        self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                    let neighbor_above =
                                        self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                    if !snow::is_visible(
                                        face,
                                        self.is_supported_cube(
                                            neighbor.id,
                                            neighbor.metadata,
                                            neighbor_above,
                                        ),
                                    ) {
                                        continue;
                                    }
                                    snow::append_face(
                                        &mut mesh,
                                        [wx, i32::from(y), wz],
                                        block,
                                        neighbor,
                                        face,
                                        source_minimum,
                                        limits,
                                    )?;
                                }
                                continue;
                            }
                            if block.id == 44 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                for face in 0..6_u8 {
                                    let (dx, dy, dz) = face_delta(face)?;
                                    let neighbor =
                                        self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                    let neighbor_above =
                                        self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                    if !slab::is_visible(
                                        face,
                                        neighbor.id,
                                        self.is_supported_cube(
                                            neighbor.id,
                                            neighbor.metadata,
                                            neighbor_above,
                                        ),
                                    ) {
                                        continue;
                                    }
                                    slab::append_face(
                                        &mut mesh,
                                        [wx, i32::from(y), wz],
                                        block,
                                        neighbor,
                                        face,
                                        source_minimum,
                                        limits,
                                    )?;
                                }
                                continue;
                            }
                            if block.id == 31 {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                let color = if block.metadata == 0 {
                                    [1.0; 3]
                                } else if let Some(sample) = tint.as_deref_mut() {
                                    let (tx, tz) = tall_grass_biome_coords(wx, i32::from(y), wz);
                                    color_tint(sample(block.id, block.metadata, tx, tz))
                                } else {
                                    [1.0; 3]
                                };
                                plant::append_grass(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    block,
                                    source_minimum,
                                    color,
                                    limits,
                                )?;
                                continue;
                            }
                            if matches!(block.id, 6 | 32 | 37..=40 | 83) {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                plant::append(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    block,
                                    source_minimum,
                                    limits,
                                )?;
                                continue;
                            }
                            if matches!(block.id, 50 | 75 | 76) {
                                let source_minimum = emission_minimum
                                    .as_deref_mut()
                                    .map_or(0, |minimum| minimum(block.id));
                                torch::append(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    block,
                                    source_minimum,
                                    limits,
                                )?;
                                continue;
                            }
                            let above_id = self.block(wx, i32::from(y) + 1, wz)?.id;
                            let fancy_leaf = self.fancy_leaves && block.id == 18;
                            let mapping = self.mapping_for(block.id, block.metadata, above_id);
                            if !fancy_leaf && !mapping.is_some_and(BlockMapping::is_renderable_cube)
                            {
                                mesh.unsupported_blocks += 1;
                                continue;
                            }
                            let block_tint = if let Some(sample) = tint.as_deref_mut() {
                                if fancy_leaf || block.id == 2 {
                                    Some(color_tint(sample(block.id, block.metadata, wx, wz)))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            let source_minimum = emission_minimum
                                .as_deref_mut()
                                .map_or(0, |minimum| minimum(block.id));
                            for face in 0..6_u8 {
                                let (dx, dy, dz) = face_delta(face)?;
                                let neighbor = self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                let neighbor_above =
                                    self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                if !(self.fancy_leaves && neighbor.id == 18)
                                    && self.is_supported_cube(
                                        neighbor.id,
                                        neighbor.metadata,
                                        neighbor_above,
                                    )
                                {
                                    continue;
                                }
                                let tile = if fancy_leaf {
                                    if block.metadata & 3 == 1 { 132 } else { 52 }
                                } else {
                                    mapping.expect("renderable mapping checked above").tiles
                                        [usize::from(face)]
                                };
                                let top_grass = block.id == 2 && face == 1;
                                let overlay_grass = tint.is_some()
                                    && block.id == 2
                                    && (2..=5).contains(&face)
                                    && tile == 3;
                                let face_tint = if fancy_leaf || top_grass {
                                    block_tint.unwrap_or([1.0; 3])
                                } else {
                                    [1.0; 3]
                                };
                                if mesh.vertices.len().saturating_add(4) > vertex_limit
                                    || mesh.indices.len().saturating_add(6) > index_limit
                                {
                                    return Err(ViewError::MeshLimitExceeded {
                                        vertices: mesh.vertices.len() + 4,
                                        indices: mesh.indices.len() + 6,
                                    });
                                }
                                append_face_tinted(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    face,
                                    tile,
                                    neighbor.block_light,
                                    neighbor.sky_light,
                                    face_tint,
                                    source_minimum,
                                    false,
                                )?;
                                if overlay_grass {
                                    if mesh.vertices.len().saturating_add(4) > vertex_limit
                                        || mesh.indices.len().saturating_add(6) > index_limit
                                    {
                                        return Err(ViewError::MeshLimitExceeded {
                                            vertices: mesh.vertices.len() + 4,
                                            indices: mesh.indices.len() + 6,
                                        });
                                    }
                                    append_face_tinted(
                                        &mut mesh,
                                        [wx, i32::from(y), wz],
                                        face,
                                        38,
                                        neighbor.block_light,
                                        neighbor.sky_light,
                                        block_tint.unwrap_or([1.0; 3]),
                                        source_minimum,
                                        true,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(mesh)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshLimits {
    /// Caller may lower this cap; the implementation also enforces `MAX_MESH_VERTICES`.
    pub max_vertices: usize,
    /// Caller may lower this cap; the implementation also enforces `MAX_MESH_INDICES`.
    pub max_indices: usize,
}
impl Default for MeshLimits {
    fn default() -> Self {
        Self {
            max_vertices: 5_000_000,
            max_indices: 7_500_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub block_light: u8,
    pub sky_light: u8,
    /// Source block's minimum saved-light level; zero for legacy mesh helpers.
    pub minimum_light: u8,
    /// Native renderer's standard-cube directional shade for this face.
    pub shade: f32,
    /// Encoded RGB multiplier from the exact native palette ARGB/RGB result.
    pub tint: [f32; 3],
    /// Grass side overlays multiply shade, brightness, then tint in native call order.
    pub tint_after_light: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub visible_faces: usize,
    pub unsupported_blocks: usize,
}

fn glass_face_visible(neighbor_id: u8, neighbor_opaque: bool) -> bool {
    neighbor_id != 20 && !neighbor_opaque
}

pub const fn face_delta(face: u8) -> Result<(i32, i32, i32), ViewError> {
    match face {
        0 => Ok((0, -1, 0)),
        1 => Ok((0, 1, 0)),
        2 => Ok((0, 0, -1)),
        3 => Ok((0, 0, 1)),
        4 => Ok((-1, 0, 0)),
        5 => Ok((1, 0, 0)),
        _ => Err(ViewError::InvalidFace(face)),
    }
}

pub const fn face_shade(face: u8) -> Result<f32, ViewError> {
    match face {
        0 => Ok(0.5),
        1 => Ok(1.0),
        2 | 3 => Ok(0.8),
        4 | 5 => Ok(0.6),
        _ => Err(ViewError::InvalidFace(face)),
    }
}

#[cfg(test)]
fn append_face(
    mesh: &mut Mesh,
    origin: [i32; 3],
    face: u8,
    tile: u16,
    block_light: u8,
    sky_light: u8,
) -> Result<(), ViewError> {
    append_face_tinted(
        mesh,
        origin,
        face,
        tile,
        block_light,
        sky_light,
        [1.0; 3],
        0,
        false,
    )
}

fn color_tint(argb: u32) -> [f32; 3] {
    [
        ((argb >> 16) & 0xff) as f32 / 255.0,
        ((argb >> 8) & 0xff) as f32 / 255.0,
        (argb & 0xff) as f32 / 255.0,
    ]
}

#[allow(clippy::too_many_arguments)]
fn append_face_tinted(
    mesh: &mut Mesh,
    origin: [i32; 3],
    face: u8,
    tile: u16,
    block_light: u8,
    sky_light: u8,
    tint: [f32; 3],
    minimum_light: u8,
    tint_after_light: bool,
) -> Result<(), ViewError> {
    let tile = usize::from(tile);
    let tx = (tile % 16) as f32 * TILE_SIZE;
    let ty = (tile / 16) as f32 * TILE_SIZE;
    let u0 = tx / ATLAS_SIZE;
    let v0 = ty / ATLAS_SIZE;
    let u1 = (tx + TILE_SIZE - TILE_INSET) / ATLAS_SIZE;
    let v1 = (ty + TILE_SIZE - TILE_INSET) / ATLAS_SIZE;
    let x = origin[0] as f32;
    let y = origin[1] as f32;
    let z = origin[2] as f32;
    // Vertices are counter-clockwise when viewed from outside the block.
    let positions = match face {
        0 => [
            [x, y, z + 1.0],
            [x, y, z],
            [x + 1.0, y, z],
            [x + 1.0, y, z + 1.0],
        ],
        1 => [
            [x + 1.0, y + 1.0, z + 1.0],
            [x + 1.0, y + 1.0, z],
            [x, y + 1.0, z],
            [x, y + 1.0, z + 1.0],
        ],
        2 => [
            [x, y + 1.0, z],
            [x + 1.0, y + 1.0, z],
            [x + 1.0, y, z],
            [x, y, z],
        ],
        3 => [
            [x, y + 1.0, z + 1.0],
            [x, y, z + 1.0],
            [x + 1.0, y, z + 1.0],
            [x + 1.0, y + 1.0, z + 1.0],
        ],
        4 => [
            [x, y + 1.0, z + 1.0],
            [x, y + 1.0, z],
            [x, y, z],
            [x, y, z + 1.0],
        ],
        5 => [
            [x + 1.0, y, z + 1.0],
            [x + 1.0, y, z],
            [x + 1.0, y + 1.0, z],
            [x + 1.0, y + 1.0, z + 1.0],
        ],
        _ => return Err(ViewError::InvalidFace(face)),
    };
    let base = u32::try_from(mesh.vertices.len()).map_err(|_| ViewError::IndexOverflow)?;
    let shade = face_shade(face)?;
    let uvs = match face {
        0 => [[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
        1 => [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        2 | 4 => [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
        3 => [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
        5 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        _ => return Err(ViewError::InvalidFace(face)),
    };
    for (position, uv) in positions.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            position,
            uv,
            block_light,
            sky_light,
            minimum_light,
            shade,
            tint,
            tint_after_light,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.visible_faces += 1;
    Ok(())
}

/// Free-fly perspective camera. Yaw zero looks toward +Z; positive pitch looks upward.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
}
impl Camera {
    pub fn new(
        position: Vec3,
        yaw: f32,
        pitch: f32,
        fov_y_degrees: f32,
        near: f32,
        far: f32,
    ) -> Result<Self, ViewError> {
        let camera = Self {
            position,
            yaw,
            pitch,
            fov_y_degrees,
            near,
            far,
        };
        camera.validate()?;
        Ok(camera)
    }
    pub fn beta_start(spawn: [i32; 3]) -> Self {
        Self {
            position: Vec3::new(
                spawn[0] as f32 + 12.0,
                spawn[1] as f32 + 18.0,
                spawn[2] as f32 - 22.0,
            ),
            yaw: 0.0,
            pitch: -0.35,
            fov_y_degrees: 70.0,
            near: 0.05,
            far: 128.0,
        }
    }
    pub fn validate(self) -> Result<(), ViewError> {
        if !self.position.is_finite()
            || !self.yaw.is_finite()
            || !self.pitch.is_finite()
            || !self.fov_y_degrees.is_finite()
            || !(1.0..179.0).contains(&self.fov_y_degrees)
            || !self.near.is_finite()
            || !self.far.is_finite()
            || self.near <= 0.0
            || self.far <= self.near
        {
            return Err(ViewError::InvalidCamera);
        }
        Ok(())
    }
    #[must_use]
    pub fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp).normalize()
    }
    #[must_use]
    pub fn right(self) -> Vec3 {
        Vec3::new(-self.yaw.cos(), 0.0, self.yaw.sin())
    }
    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) -> Result<(), ViewError> {
        self.yaw += yaw_delta;
        self.pitch = (self.pitch + pitch_delta).clamp(-1.553343, 1.553343);
        self.validate()
    }
    pub fn move_local(
        &mut self,
        forward: f32,
        strafe: f32,
        vertical: f32,
        speed: f32,
        dt: f32,
    ) -> Result<(), ViewError> {
        if ![forward, strafe, vertical, speed, dt]
            .into_iter()
            .all(f32::is_finite)
            || speed < 0.0
            || dt < 0.0
        {
            return Err(ViewError::InvalidCamera);
        }
        let horizontal = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let right = self.right();
        self.position += (horizontal * forward + right * strafe + Vec3::Y * vertical) * speed * dt;
        self.validate()
    }
    pub fn view_projection(self, aspect: f32) -> Result<Mat4, ViewError> {
        self.validate()?;
        if !aspect.is_finite() || aspect <= 0.0 {
            return Err(ViewError::InvalidCamera);
        }
        let forward = self.forward();
        let right = self.right();
        let up = right.cross(forward).normalize();
        let position = self.position;
        let view = Mat4::from_cols(
            Vec4::new(right.x, up.x, -forward.x, 0.0),
            Vec4::new(right.y, up.y, -forward.y, 0.0),
            Vec4::new(right.z, up.z, -forward.z, 0.0),
            Vec4::new(
                -right.dot(position),
                -up.dot(position),
                forward.dot(position),
                1.0,
            ),
        );
        let f = (self.fov_y_degrees.to_radians() * 0.5).tan().recip();
        let range = self.near - self.far;
        // Right-handed zero-to-one depth range, as used by wgpu.
        let projection = Mat4::from_cols(
            Vec4::new(f / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, self.far / range, -1.0),
            Vec4::new(0.0, 0.0, self.near * self.far / range, 0.0),
        );
        Ok(projection * view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mapping(id: u8) -> BlockMapping {
        BlockMapping {
            key: BlockStateKey {
                id,
                metadata: 0,
                above_id: None,
            },
            render_type: 0,
            opaque: true,
            full_cube: true,
            alpha_opaque_faces: [true; 6],
            tiles: [1; 6],
            supported: true,
        }
    }
    fn one_block_scene(neighbor: bool) -> Scene {
        let mut data = ChunkData::new();
        data.set(
            0,
            1,
            0,
            beta_core::chunk::BlockData {
                id: 1,
                metadata: 0,
                block_light: 3,
                sky_light: 14,
            },
        )
        .unwrap();
        if neighbor {
            data.set(
                1,
                1,
                0,
                beta_core::chunk::BlockData {
                    id: 1,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let bounds = SceneBounds::new(0, 0, 1, 1).unwrap();
        Scene::new(
            bounds,
            vec![SceneChunk { x: 0, z: 0, data }],
            vec![mapping(1)],
        )
        .unwrap()
    }

    fn blocks_scene(blocks: &[(u8, u8, u8, u8, u8)], mappings: Vec<BlockMapping>) -> Scene {
        let mut data = ChunkData::new();
        for &(x, y, z, id, metadata) in blocks {
            data.set(
                x,
                y,
                z,
                beta_core::chunk::BlockData {
                    id,
                    metadata,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        Scene::new(
            SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![SceneChunk { x: 0, z: 0, data }],
            mappings,
        )
        .unwrap()
    }

    #[test]
    fn water_is_routed_to_bounded_transparent_mesh_only() {
        let scene = blocks_scene(&[(3, 10, 4, 9, 0)], vec![]);
        let opaque = scene.mesh(MeshLimits::default()).unwrap();
        let fluid = scene.fluid_mesh(MeshLimits::default()).unwrap();
        assert_eq!(opaque.unsupported_blocks, 0);
        assert!(opaque.vertices.is_empty());
        assert_eq!(fluid.vertices.len(), 30);
        assert_eq!(fluid.vertices[0].position[0], 3.0);
        assert_eq!(fluid.vertices[0].position[2], 4.0);
        assert!((fluid.vertices[0].position[1] - 10.698_413).abs() < 0.000_001);
        assert!(matches!(
            scene.fluid_mesh(MeshLimits {
                max_vertices: 29,
                max_indices: 0
            }),
            Err(ViewError::MeshLimitExceeded { .. })
        ));
    }

    #[test]
    fn bounded_water_cell_query_returns_ids_and_metadata_in_scene_coordinates() {
        let scene = blocks_scene(
            &[(3, 10, 4, 9, 6), (4, 10, 4, 8, 2), (5, 10, 4, 1, 0)],
            vec![],
        );
        let cells = scene
            .water_cells_in(Aabb::new([2.0, 9.0, 3.0], [6.0, 12.0, 5.0]))
            .unwrap();
        assert_eq!(
            cells,
            BTreeMap::from([
                ((3, 10, 4), WaterMotionCell { id: 9, metadata: 6 }),
                ((4, 10, 4), WaterMotionCell { id: 8, metadata: 2 }),
            ])
        );
        assert!(matches!(
            scene.water_cells_in(Aabb::new([0.0, 0.0, 0.0], [1.0, 33.0, 1.0])),
            Err(ViewError::InvalidColliderQuery)
        ));
    }

    #[test]
    fn chunk_mesh_face_uses_light_from_adjacent_chunk() {
        let mut left = ChunkData::new();
        left.set(
            15,
            64,
            4,
            beta_core::chunk::BlockData {
                id: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let mut right = ChunkData::new();
        right
            .set(
                0,
                64,
                4,
                beta_core::chunk::BlockData {
                    sky_light: 14,
                    ..Default::default()
                },
            )
            .unwrap();
        let scene = Scene::new(
            SceneBounds::new(0, 0, 2, 1).unwrap(),
            vec![
                SceneChunk {
                    x: 0,
                    z: 0,
                    data: left,
                },
                SceneChunk {
                    x: 1,
                    z: 0,
                    data: right,
                },
            ],
            vec![mapping(1)],
        )
        .unwrap();
        let mesh = scene
            .mesh_chunks_with_tint_and_emission(
                MeshLimits::default(),
                &[(0, 0)],
                |_, _, _, _| 0xffffff,
                |_| 0,
            )
            .unwrap();
        let east_face = &mesh.vertices[20..24];
        assert!(
            east_face
                .iter()
                .all(|v| v.sky_light == 14 && v.position[0] == 16.0)
        );
    }

    #[test]
    fn mesh_emits_six_outward_faces_with_native_shade_uv_and_light() {
        let mut data = ChunkData::new();
        data.set(
            0,
            1,
            0,
            beta_core::chunk::BlockData {
                id: 1,
                metadata: 0,
                block_light: 3,
                sky_light: 14,
            },
        )
        .unwrap();
        for (coord, block_light, sky_light) in [
            ((0, 0, 0), 4, 10),
            ((0, 2, 0), 5, 11),
            ((0, 1, 1), 6, 12),
            ((1, 1, 0), 7, 13),
        ] {
            data.set(
                coord.0,
                coord.1,
                coord.2,
                beta_core::chunk::BlockData {
                    block_light,
                    sky_light,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let scene = Scene::new(
            SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![SceneChunk { x: 0, z: 0, data }],
            vec![mapping(1)],
        )
        .unwrap();
        let mesh = scene.mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.visible_faces, 6);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        let positions = [
            [
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
            [
                [1.0, 2.0, 1.0],
                [1.0, 2.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.0, 2.0, 1.0],
            ],
            [
                [0.0, 2.0, 0.0],
                [1.0, 2.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            [
                [0.0, 2.0, 1.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 2.0, 1.0],
            ],
            [
                [0.0, 2.0, 1.0],
                [0.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
            ],
            [
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [1.0, 2.0, 0.0],
                [1.0, 2.0, 1.0],
            ],
        ];
        let u0 = 16.0 / 256.0;
        let u1 = (16.0 + 15.99) / 256.0;
        let v0 = 0.0;
        let v1 = 15.99 / 256.0;
        let uvs = [
            [[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
            [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
            [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
            [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
            [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
            [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        ];
        let lights = [(4, 10), (5, 11), (0, 0), (6, 12), (0, 0), (7, 13)];
        for face in 0..6 {
            let vertices = &mesh.vertices[usize::from(face) * 4..usize::from(face) * 4 + 4];
            for corner in 0..4 {
                assert_eq!(
                    vertices[corner].position,
                    positions[usize::from(face)][corner]
                );
                assert_eq!(vertices[corner].uv, uvs[usize::from(face)][corner]);
                assert_eq!(vertices[corner].block_light, lights[usize::from(face)].0);
                assert_eq!(vertices[corner].sky_light, lights[usize::from(face)].1);
                assert_eq!(vertices[corner].shade, face_shade(face).unwrap());
            }
            let a = Vec3::from_array(vertices[0].position);
            let b = Vec3::from_array(vertices[1].position);
            let c = Vec3::from_array(vertices[2].position);
            let (dx, dy, dz) = face_delta(face).unwrap();
            let normal = (b - a).cross(c - a);
            assert!(
                normal.dot(Vec3::new(dx as f32, dy as f32, dz as f32)) > 0.0,
                "face {face} winding"
            );
        }
    }

    #[test]
    fn selected_chunk_mesh_keeps_neighbor_occlusion_and_chunk_replace_updates_data() {
        let mut west = ChunkData::new();
        west.set(
            15,
            10,
            4,
            beta_core::chunk::BlockData {
                id: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let mut east = ChunkData::new();
        east.set(
            0,
            10,
            4,
            beta_core::chunk::BlockData {
                id: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let bounds = SceneBounds::new(0, 0, 2, 1).unwrap();
        let mut scene = Scene::new(
            bounds,
            vec![
                SceneChunk {
                    x: 0,
                    z: 0,
                    data: west,
                },
                SceneChunk {
                    x: 1,
                    z: 0,
                    data: east.clone(),
                },
            ],
            vec![mapping(1)],
        )
        .unwrap();
        let full = scene.mesh(MeshLimits::default()).unwrap();
        let selected = scene
            .mesh_chunks_with_tint_and_emission(
                MeshLimits::default(),
                &[(0, 0)],
                |_, _, _, _| 0x00ff_ffff,
                beta_core::light::emission_level,
            )
            .unwrap();
        assert_eq!(full.visible_faces, 10);
        assert_eq!(selected.visible_faces, 5);
        assert_eq!(selected.vertices.len(), 20);

        east.set(0, 10, 4, beta_core::chunk::BlockData::default())
            .unwrap();
        scene
            .replace_chunk(SceneChunk {
                x: 1,
                z: 0,
                data: east,
            })
            .unwrap();
        let updated = scene
            .mesh_chunks_with_tint_and_emission(
                MeshLimits::default(),
                &[(0, 0)],
                |_, _, _, _| 0x00ff_ffff,
                beta_core::light::emission_level,
            )
            .unwrap();
        assert_eq!(updated.visible_faces, 6);
        assert_eq!(updated.vertices.len(), 24);
    }

    #[test]
    fn cube_face_uv_associations_match_native_tessellator_vertices() {
        let mut native_uv = HashMap::new();
        for row in include_str!("../../../tests/fixtures/uv/native.tsv").lines() {
            if row.is_empty() || row.starts_with('#') {
                continue;
            }
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 7, "native row {row}");
            let position = [
                u32::from_str_radix(fields[2], 16).unwrap(),
                u32::from_str_radix(fields[3], 16).unwrap(),
                u32::from_str_radix(fields[4], 16).unwrap(),
            ];
            let uv = [
                u32::from_str_radix(fields[5], 16).unwrap(),
                u32::from_str_radix(fields[6], 16).unwrap(),
            ];
            if let Some(previous) = native_uv.insert((fields[0].to_owned(), position), uv) {
                assert_eq!(previous, uv, "native triangle duplicates differ for {row}");
            }
        }
        assert_eq!(native_uv.len(), 84 * 4);

        let mut case_count = 0;
        let mut checked_vertices = 0;
        let mut mismatches = Vec::new();
        for row in include_str!("../../../tests/fixtures/uv/cases.tsv").lines() {
            if row.is_empty() || row.starts_with('#') {
                continue;
            }
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 6, "case row {row}");
            let face = fields[1].parse::<u8>().unwrap();
            let tile = fields[2].parse::<u16>().unwrap();
            let origin = [
                fields[3].parse::<i32>().unwrap(),
                fields[4].parse::<i32>().unwrap(),
                fields[5].parse::<i32>().unwrap(),
            ];
            let mut mesh = Mesh::default();
            append_face(&mut mesh, origin, face, tile, 0, 0).unwrap();
            assert_eq!(mesh.vertices.len(), 4);
            for vertex in mesh.vertices {
                let position = vertex.position.map(f32::to_bits);
                let actual_uv = vertex.uv.map(f32::to_bits);
                let expected_uv = native_uv
                    .get(&(fields[0].to_owned(), position))
                    .unwrap_or_else(|| panic!("missing native corner {} {position:?}", fields[0]));
                if &actual_uv != expected_uv {
                    mismatches.push(format!(
                        "{} face {face} position {position:08x?}: native {expected_uv:08x?}, Rust {actual_uv:08x?}",
                        fields[0]
                    ));
                }
                checked_vertices += 1;
            }
            case_count += 1;
        }
        assert_eq!(case_count, 84);
        assert_eq!(checked_vertices, 336);
        assert!(
            mismatches.is_empty(),
            "native UV mismatches:\n{}",
            mismatches.join("\n")
        );
    }

    #[test]
    fn adjacent_confirmed_cubes_cull_shared_face_and_missing_neighbors_are_air() {
        let mesh = one_block_scene(true).mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.visible_faces, 10);
    }

    #[test]
    fn glass_mesh_matches_native_vertices_and_visibility() {
        let scene = blocks_scene(&[(0, 1, 0, 20, 0)], vec![]);
        let mesh = scene.mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.unsupported_blocks, 0);
        assert_eq!(mesh.visible_faces, 6);
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/glass_render/native.tsv")
            .lines()
            .filter(|l| l.starts_with("vertex\t20\t"))
        {
            let row: Vec<_> = line.split('\t').collect();
            let index: usize = row[3].parse().unwrap();
            let bits: Vec<_> = row[4..9]
                .iter()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            let vertex = mesh.vertices[mesh.indices[index] as usize];
            assert_eq!(
                [
                    vertex.position[0].to_bits(),
                    (vertex.position[1] - 1.0).to_bits(),
                    vertex.position[2].to_bits()
                ],
                [bits[0], bits[1], bits[2]]
            ); // native reference is at y=0
            assert_eq!(vertex.uv.map(f32::to_bits), [bits[3], bits[4]]);
            count += 1;
        }
        assert_eq!(count, 16 * 36);
        for line in include_str!("../../../tests/fixtures/glass_render/visibility.tsv").lines() {
            let row: Vec<_> = line.split('\t').collect();
            let neighbor: u8 = row[0].parse().unwrap();
            assert_eq!(
                glass_face_visible(neighbor, neighbor == 1),
                row[2] == "1",
                "neighbor={neighbor} face={}",
                row[1]
            );
        }
    }

    #[test]
    fn tall_grass_scene_uses_native_biome_sample_and_white_shrub() {
        let scene = blocks_scene(&[(0, 1, 0, 31, 0), (1, 1, 0, 31, 1)], vec![]);
        let mut sampled = Vec::new();
        let mesh = scene
            .mesh_with_tint(MeshLimits::default(), |id, meta, x, z| {
                sampled.push((id, meta, x, z));
                0x0076923d
            })
            .unwrap();
        let (x, z) = tall_grass_biome_coords(1, 1, 0);
        assert_eq!(sampled, vec![(31, 1, x, z)]);
        assert_eq!(mesh.unsupported_blocks, 0);
        assert_eq!(mesh.vertices.len(), 48);
        assert!(mesh.vertices[..24].iter().all(|v| v.tint == [1.0; 3]));
        assert!(
            mesh.vertices[24..]
                .iter()
                .all(|v| v.tint == color_tint(0x0076923d))
        );
    }

    #[test]
    fn fancy_leaf_tiles_tint_and_occlusion_match_native_fixture() {
        let leaf = blocks_scene(&[(0, 1, 0, 18, 1)], vec![]);
        assert_eq!(
            leaf.mesh(MeshLimits::default()).unwrap().unsupported_blocks,
            1
        );

        let mut leaf = leaf;
        leaf.enable_fancy_leaves();
        let color = 0x0012_3456;
        let mesh = leaf
            .mesh_with_tint(MeshLimits::default(), |id, meta, x, z| {
                assert_eq!((id, meta, x, z), (18, 1, 0, 0));
                color
            })
            .unwrap();
        assert_eq!(mesh.visible_faces, 6);
        assert!(mesh.vertices.iter().all(|v| v.tint == color_tint(color)));
        let tile132_u0 = (132 % 16) as f32 * TILE_SIZE / ATLAS_SIZE;
        assert_eq!(mesh.vertices[0].uv[0], tile132_u0);

        let mut two_leaves = blocks_scene(&[(0, 1, 0, 18, 1), (1, 1, 0, 18, 0)], vec![]);
        two_leaves.enable_fancy_leaves();
        let mesh = two_leaves.mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.visible_faces, 12, "fancy leaves retain shared faces");

        let mut leaf_and_stone =
            blocks_scene(&[(0, 1, 0, 18, 1), (1, 1, 0, 1, 0)], vec![mapping(1)]);
        leaf_and_stone.enable_fancy_leaves();
        assert_eq!(
            leaf_and_stone
                .mesh(MeshLimits::default())
                .unwrap()
                .visible_faces,
            11,
            "opaque stone faces are retained against nonopaque fancy leaves"
        );

        let stale_opaque_leaf_mapping = BlockMapping {
            key: BlockStateKey {
                id: 18,
                metadata: 0,
                above_id: None,
            },
            ..mapping(18)
        };
        let mut two_leaves = blocks_scene(
            &[(0, 1, 0, 18, 0), (1, 1, 0, 18, 0)],
            vec![stale_opaque_leaf_mapping],
        );
        two_leaves.enable_fancy_leaves();
        assert_eq!(
            two_leaves
                .mesh(MeshLimits::default())
                .unwrap()
                .visible_faces,
            12,
            "fancy mode must override a stale opaque mapping for leaves"
        );
    }

    #[test]
    fn fancy_leaf_tile_selection_matches_native_metadata_fixture() {
        for metadata in 0_u8..=15 {
            let scene = blocks_scene(&[(0, 1, 0, 18, metadata)], vec![]);
            let mut scene = scene;
            scene.enable_fancy_leaves();
            let mesh = scene.mesh(MeshLimits::default()).unwrap();
            assert_eq!(mesh.visible_faces, 6);
            for face in 0_u8..6 {
                let row = include_str!("../../../tests/fixtures/leaf/native.tsv")
                    .lines()
                    .find(|line| {
                        let fields: Vec<_> = line.split('\t').collect();
                        fields.len() == 6
                            && fields[0] == "leaf"
                            && fields[1] == "1"
                            && fields[2].parse::<u8>().ok() == Some(metadata)
                            && fields[3].parse::<u8>().ok() == Some(face)
                            && fields[4] == "0"
                    })
                    .expect("native fixture has every fancy metadata/face case");
                let native_tile = row.split('\t').nth(5).unwrap().parse::<u16>().unwrap();
                let u0 = (native_tile % 16) as f32 * TILE_SIZE / ATLAS_SIZE;
                let v0 = (native_tile / 16) as f32 * TILE_SIZE / ATLAS_SIZE;
                let u1 = u0 + (TILE_SIZE - TILE_INSET) / ATLAS_SIZE;
                let v1 = v0 + (TILE_SIZE - TILE_INSET) / ATLAS_SIZE;
                let vertices = &mesh.vertices[usize::from(face) * 4..usize::from(face) * 4 + 4];
                assert!(
                    vertices.iter().all(|vertex| {
                        vertex.uv[0] >= u0 - 1.0e-6
                            && vertex.uv[0] <= u1 + 1.0e-6
                            && vertex.uv[1] >= v0 - 1.0e-6
                            && vertex.uv[1] <= v1 + 1.0e-6
                    }),
                    "metadata {metadata} face {face}: expected native tile {native_tile}"
                );
            }
        }
    }

    #[test]
    fn grass_palette_tints_top_and_native_side_overlays_only() {
        let mut grass_mapping = mapping(2);
        grass_mapping.tiles = [2, 0, 3, 3, 3, 3];
        let scene = blocks_scene(&[(0, 1, 0, 2, 0)], vec![grass_mapping]);
        let legacy_mesh = scene.mesh(MeshLimits::default()).unwrap();
        assert_eq!(legacy_mesh.visible_faces, 6);
        assert!(legacy_mesh.vertices.iter().all(|v| v.tint == [1.0; 3]));
        assert!(legacy_mesh.vertices.iter().all(|v| !v.tint_after_light));
        let color = 0x00a1_b2c3;
        let mesh = scene
            .mesh_with_tint(MeshLimits::default(), |id, meta, x, z| {
                assert_eq!((id, meta, x, z), (2, 0, 0, 0));
                color
            })
            .unwrap();
        assert_eq!(mesh.visible_faces, 10);
        assert_eq!(mesh.vertices.len(), 40);
        assert!(
            mesh.vertices[0..8]
                .iter()
                .all(|vertex| !vertex.tint_after_light)
        );
        for side in 2_usize..6 {
            let base = side * 8 - 8;
            assert!(
                mesh.vertices[base..base + 4]
                    .iter()
                    .all(|vertex| !vertex.tint_after_light)
            );
            assert!(
                mesh.vertices[base + 4..base + 8]
                    .iter()
                    .all(|vertex| vertex.tint_after_light)
            );
        }
        assert_eq!(
            &mesh.vertices[0..4]
                .iter()
                .map(|v| v.tint)
                .collect::<Vec<_>>(),
            &vec![[1.0; 3]; 4]
        );
        assert!(
            mesh.vertices[4..8]
                .iter()
                .all(|v| v.tint == color_tint(color))
        );
        for side in 2_u8..6 {
            let base = (usize::from(side) + usize::from(side - 2)) * 4;
            assert!(
                mesh.vertices[base..base + 4]
                    .iter()
                    .all(|v| v.tint == [1.0; 3])
            );
            let overlay = base + 4;
            assert!(
                mesh.vertices[overlay..overlay + 4]
                    .iter()
                    .all(|v| v.tint == color_tint(color))
            );
            assert_eq!(
                mesh.vertices[base..base + 4]
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>(),
                mesh.vertices[overlay..overlay + 4]
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>()
            );
            let overlay_u0 = (38 % 16) as f32 * TILE_SIZE / ATLAS_SIZE;
            assert!(
                mesh.vertices[overlay..overlay + 4]
                    .iter()
                    .all(|v| v.uv[0] >= overlay_u0)
            );
        }
        assert!(
            scene
                .mesh_with_tint(
                    MeshLimits {
                        max_vertices: 39,
                        max_indices: 100
                    },
                    |_, _, _, _| color,
                )
                .is_err()
        );
    }

    #[test]
    fn emission_minimum_is_source_block_value_on_base_and_overlay_vertices() {
        let mut grass_mapping = mapping(2);
        grass_mapping.tiles = [2, 0, 3, 3, 3, 3];
        let scene = blocks_scene(&[(0, 1, 0, 2, 0)], vec![grass_mapping]);
        let legacy = scene.mesh(MeshLimits::default()).unwrap();
        assert_eq!(legacy.visible_faces, 6);
        assert!(
            legacy
                .vertices
                .iter()
                .all(|vertex| vertex.minimum_light == 0)
        );

        let tinted = scene
            .mesh_with_tint(MeshLimits::default(), |_, _, _, _| 0x00aabbcc)
            .unwrap();
        assert!(
            tinted
                .vertices
                .iter()
                .all(|vertex| vertex.minimum_light == 0)
        );

        let mut seen = Vec::new();
        let emitted = scene
            .mesh_with_tint_and_emission(
                MeshLimits::default(),
                |_, _, _, _| 0x00aabbcc,
                |id| {
                    seen.push(id);
                    7
                },
            )
            .unwrap();
        assert_eq!(emitted.visible_faces, 10);
        assert_eq!(seen, vec![2]);
        assert!(
            emitted
                .vertices
                .iter()
                .all(|vertex| vertex.minimum_light == 7)
        );
        assert!(
            emitted
                .vertices
                .iter()
                .any(|vertex| vertex.uv[0] >= (38 % 16) as f32 * TILE_SIZE / ATLAS_SIZE)
        );
    }

    #[test]
    fn fancy_leaf_collision_remains_full_cube() {
        let mut scene = blocks_scene(&[(0, 1, 0, 18, 0)], vec![]);
        scene.enable_fancy_leaves();
        let unit = Aabb::new([0.0, 1.0, 0.0], [1.0, 2.0, 1.0]);
        assert_eq!(scene.colliders_in(unit).unwrap(), vec![unit]);
        assert_eq!(scene.safe_spawn_near([0, 2, 0]).unwrap(), [0.5, 2.0, 0.5]);
        assert_eq!(
            scene
                .surface_below(Aabb::new([0.2, 2.0, 0.2], [0.8, 3.8, 0.8]))
                .unwrap(),
            beta_core::motion::Surface::Stone
        );
    }

    #[test]
    fn collision_query_is_local_and_safe_spawn_requires_confirmed_floor() {
        let scene = one_block_scene(false);
        assert_eq!(
            scene
                .colliders_in(Aabb::new([0.0, 1.0, 0.0], [1.0, 2.0, 1.0]))
                .unwrap(),
            vec![Aabb::new([0.0, 1.0, 0.0], [1.0, 2.0, 1.0])]
        );
        assert_eq!(scene.safe_spawn_near([0, 2, 0]).unwrap(), [0.5, 2.0, 0.5]);
        assert!(
            scene
                .colliders_in(Aabb::new([0.0, 0.0, 0.0], [1.0, 33.0, 1.0]))
                .is_err()
        );
        assert!(
            Scene::new(SceneBounds::new(0, 0, 1, 1).unwrap(), vec![], vec![])
                .unwrap()
                .safe_spawn_near([0, 64, 0])
                .is_err()
        );
    }

    #[test]
    fn valid_upper_i32_scene_bounds_do_not_overflow_exclusive_edges() {
        let bounds = SceneBounds::new(134_217_727, 0, 1, 1).unwrap();
        let scene = Scene::new(bounds, vec![], vec![]).unwrap();
        let max = f64::from(i32::MAX);
        assert!(
            scene
                .colliders_in(Aabb::new([max, 0.0, 0.0], [max + 1.0, 1.0, 1.0]))
                .unwrap()
                .is_empty()
        );
        assert!(scene.safe_spawn_near([i32::MAX, 64, 0]).is_err());
        assert_eq!(
            scene
                .surface_below(Aabb::new(
                    [max, f64::from(i32::MIN), 0.0],
                    [max + 1.0, f64::from(i32::MIN) + 1.0, 1.0]
                ))
                .unwrap(),
            beta_core::motion::Surface::Air
        );
        assert_eq!(
            scene
                .surface_below(Aabb::new(
                    [0.0, -f64::MAX, 0.0],
                    [1.0, -f64::MAX + 1.0, 1.0],
                ))
                .unwrap(),
            beta_core::motion::Surface::Air
        );
    }

    #[test]
    fn world_aware_mapping_uses_exact_above_block_context() {
        let mut data = ChunkData::new();
        data.set(
            0,
            1,
            0,
            beta_core::chunk::BlockData {
                id: 2,
                metadata: 0,
                ..Default::default()
            },
        )
        .unwrap();
        data.set(
            0,
            2,
            0,
            beta_core::chunk::BlockData {
                id: 8,
                metadata: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let mut grass = mapping(2);
        grass.key.above_id = Some(8);
        grass.tiles = [2, 0, 68, 68, 68, 68];
        let scene = Scene::new(
            SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![SceneChunk { x: 0, z: 0, data }],
            vec![grass.clone()],
        )
        .unwrap();
        let mesh = scene.mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.visible_faces, 6);
        assert_eq!(mesh.unsupported_blocks, 0);
        assert_eq!(
            mesh.vertices[8].uv[0],
            ((68 % 16) as f32 * 16.0 + TILE_SIZE - TILE_INSET) / ATLAS_SIZE
        );
    }

    #[test]
    fn unsupported_states_are_reported_and_mesh_limits_are_hard() {
        let mut data = ChunkData::new();
        data.set(
            0,
            0,
            0,
            beta_core::chunk::BlockData {
                id: 7,
                metadata: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let scene = Scene::new(
            SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![SceneChunk { x: 0, z: 0, data }],
            vec![],
        )
        .unwrap();
        assert_eq!(
            scene
                .mesh(MeshLimits::default())
                .unwrap()
                .unsupported_blocks,
            1
        );
        assert!(matches!(
            one_block_scene(false).mesh(MeshLimits {
                max_vertices: 3,
                max_indices: 3
            }),
            Err(ViewError::MeshLimitExceeded { .. })
        ));
    }

    #[test]
    fn scene_bounds_duplicate_and_camera_validation() {
        assert!(SceneBounds::new(0, 0, 6, 1).is_err());
        assert!(SceneBounds::new(i32::MAX, 0, 2, 1).is_err());
        assert!(SceneBounds::new(i32::MAX / 16, 0, 2, 1).is_err());
        let bounds = SceneBounds::new(0, 0, 1, 1).unwrap();
        assert!(matches!(
            Scene::new(
                bounds,
                vec![
                    SceneChunk {
                        x: 0,
                        z: 0,
                        data: ChunkData::new()
                    },
                    SceneChunk {
                        x: 0,
                        z: 0,
                        data: ChunkData::new()
                    },
                ],
                vec![]
            ),
            Err(ViewError::DuplicateChunk { x: 0, z: 0 })
        ));
        assert!(matches!(
            Scene::new(
                bounds,
                vec![SceneChunk {
                    x: 1,
                    z: 0,
                    data: ChunkData::new()
                }],
                vec![]
            ),
            Err(ViewError::ChunkOutsideScene { x: 1, z: 0 })
        ));
        assert!(ChunkData::from_raw(vec![], vec![], vec![], vec![]).is_err());
        assert!(matches!(
            Camera::new(Vec3::ZERO, 0.0, 0.0, 70.0, 0.05, 128.0)
                .unwrap()
                .view_projection(0.0),
            Err(ViewError::InvalidCamera)
        ));
        let mut camera = Camera::beta_start([-9, 64, -5]);
        assert!((camera.forward().length() - 1.0).abs() < f32::EPSILON);
        assert_eq!(camera.right(), Vec3::new(-1.0, 0.0, 0.0));
        let before_strafe = camera.position;
        camera.move_local(0.0, 1.0, 0.0, 1.0, 1.0).unwrap();
        assert!(camera.position.x < before_strafe.x);
        camera.pitch = std::f32::consts::FRAC_PI_2;
        assert!(camera.right().is_finite());
        assert!(camera.view_projection(16.0 / 9.0).unwrap().is_finite());
        assert!(camera.view_projection(16.0 / 9.0).unwrap().is_finite());
        camera.rotate(0.1, 20.0).unwrap();
        assert!(camera.pitch < 1.56 && camera.pitch > 1.55);
        camera.move_local(1.0, 0.0, 0.0, 4.0, 0.5).unwrap();
        assert!(camera.position.is_finite());
    }
}
