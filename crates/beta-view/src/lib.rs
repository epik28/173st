#![forbid(unsafe_code)]

//! Bounded CPU scene, camera, and standard-cube meshing for the Beta 1.7.3 viewer.
//!
//! This crate deliberately does not load files or create a window/GPU device. Callers
//! provide already-validated chunks and mappings produced by the native view oracle.
//! Mesh vertices retain the adjacent block's sky/block light nibbles for each full-cube
//! face and native directional face shade as separate values. This follows the sample
//! position used by the native flat renderer but does not reproduce `uu.d` packed
//! brightness conversion or ambient-occlusion corner sampling.

use std::{collections::BTreeMap, error::Error, fmt};

use beta_core::{
    chunk::{ChunkData, ChunkError},
    geometry::Aabb,
};
use glam::{Mat4, Vec3, Vec4};

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
        let world_min_x = self.bounds.min_chunk_x * 16;
        let world_min_z = self.bounds.min_chunk_z * 16;
        let world_max_x = world_min_x + i32::from(self.bounds.width) * 16;
        let world_max_z = world_min_z + i32::from(self.bounds.depth) * 16;
        let x0 = (area.min[0].floor() as i32).max(world_min_x);
        let x1 = (area.max[0].ceil() as i32).min(world_max_x);
        let y0 = (area.min[1].floor() as i32).clamp(0, WORLD_HEIGHT as i32);
        let y1 = (area.max[1].ceil() as i32).clamp(0, WORLD_HEIGHT as i32);
        let z0 = (area.min[2].floor() as i32).max(world_min_z);
        let z1 = (area.max[2].ceil() as i32).min(world_max_z);
        if x1.saturating_sub(x0) > 32 || y1.saturating_sub(y0) > 32 || z1.saturating_sub(z0) > 32 {
            return Err(ViewError::InvalidColliderQuery);
        }
        let mut result = Vec::new();
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    let block = self.block(x, y, z)?;
                    if block.id == 0 {
                        continue;
                    }
                    let above = self.block(x, y + 1, z)?.id;
                    if self.is_supported_cube(block.id, block.metadata, above) {
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

    /// Finds a nearby column with a confirmed solid floor and two clear blocks.
    pub fn safe_spawn_near(&self, spawn: [i32; 3]) -> Result<[f64; 3], ViewError> {
        let min_x = self.bounds.min_chunk_x * 16;
        let min_z = self.bounds.min_chunk_z * 16;
        let max_x = min_x + i32::from(self.bounds.width) * 16 - 1;
        let max_z = min_z + i32::from(self.bounds.depth) * 16 - 1;
        let center_x = spawn[0].clamp(min_x, max_x);
        let center_z = spawn[2].clamp(min_z, max_z);
        for radius in 0_i32..=16 {
            for dx in -radius..=radius {
                let dz_abs = radius - dx.abs();
                for dz in if dz_abs == 0 {
                    vec![0]
                } else {
                    vec![-dz_abs, dz_abs]
                } {
                    let x = center_x + dx;
                    let z = center_z + dz;
                    if x < min_x || x > max_x || z < min_z || z > max_z {
                        continue;
                    }
                    let start_y = spawn[1].clamp(1, WORLD_HEIGHT as i32 - 3);
                    for y in (1..=start_y)
                        .rev()
                        .chain(start_y + 1..WORLD_HEIGHT as i32 - 2)
                    {
                        let floor = self.block(x, y - 1, z)?;
                        if floor.id == 0 {
                            continue;
                        }
                        let above = self.block(x, y, z)?;
                        let head = self.block(x, y + 1, z)?;
                        if above.id == 0 && head.id == 0 {
                            let floor_above = above.id;
                            if self.is_supported_cube(floor.id, floor.metadata, floor_above) {
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
        let y = body.min[1].floor() as i32 - 1;
        let block = self.block(x, y, z)?;
        let above = self.block(x, y + 1, z)?.id;
        if block.id == 0 || !self.is_supported_cube(block.id, block.metadata, above) {
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

    /// Deterministic face mesh in world block coordinates. Unsupported states are omitted
    /// and counted once per occupied block. Adjacent supported full cubes share no face.
    pub fn mesh(&self, limits: MeshLimits) -> Result<Mesh, ViewError> {
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
                            let wx = x0 + i32::from(chunk_x) * 16 + i32::from(local_x);
                            let wz = z0 + i32::from(chunk_z) * 16 + i32::from(local_z);
                            let above_id = self.block(wx, i32::from(y) + 1, wz)?.id;
                            let Some(mapping) =
                                self.mapping_for(block.id, block.metadata, above_id)
                            else {
                                mesh.unsupported_blocks += 1;
                                continue;
                            };
                            if !mapping.is_renderable_cube() {
                                mesh.unsupported_blocks += 1;
                                continue;
                            }
                            for face in 0..6_u8 {
                                let (dx, dy, dz) = face_delta(face)?;
                                let neighbor = self.block(wx + dx, i32::from(y) + dy, wz + dz)?;
                                let neighbor_above =
                                    self.block(wx + dx, i32::from(y) + dy + 1, wz + dz)?.id;
                                if self.is_supported_cube(
                                    neighbor.id,
                                    neighbor.metadata,
                                    neighbor_above,
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
                                append_face(
                                    &mut mesh,
                                    [wx, i32::from(y), wz],
                                    face,
                                    mapping.tiles[usize::from(face)],
                                    neighbor.block_light,
                                    neighbor.sky_light,
                                )?;
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
    /// Native renderer's standard-cube directional shade for this face.
    pub shade: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub visible_faces: usize,
    pub unsupported_blocks: usize,
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

fn append_face(
    mesh: &mut Mesh,
    origin: [i32; 3],
    face: u8,
    tile: u16,
    block_light: u8,
    sky_light: u8,
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
        1..=4 => [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
        5 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        _ => return Err(ViewError::InvalidFace(face)),
    };
    for (position, uv) in positions.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            position,
            uv,
            block_light,
            sky_light,
            shade,
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
            [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
            [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
            [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
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
    fn adjacent_confirmed_cubes_cull_shared_face_and_missing_neighbors_are_air() {
        let mesh = one_block_scene(true).mesh(MeshLimits::default()).unwrap();
        assert_eq!(mesh.visible_faces, 10);
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
        assert_eq!(mesh.unsupported_blocks, 1);
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
