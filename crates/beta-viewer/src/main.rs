#![forbid(unsafe_code)]

//! Read-only winit/wgpu rendering shell with fixed-step walking and debug flight.
//! The scene and CPU mesh are prepared once and held in static GPU buffers.

mod hotbar_input;
mod hud;
mod inventory_screen;
mod saved_player;

use beta_core::{
    biome::{Climate, ClimateSampler, foliage_color, grass_color},
    block::{
        BlockEdit, BlockHit, BlockScene, BlockSceneBounds, BlockSceneChunk, RayResult, RayShape,
    },
    chunk::{BlockData, split_block_coord},
    items::Inventory,
    light::{LightDimension, sample_brightness, sky_subtraction},
    motion::MotionState,
};
use beta_view::{BlockMapping, Camera, Mesh, Scene, SceneChunk, Vertex, fluid::Mesh as FluidMesh};
use beta_viewer::controller::{
    Controls, MotionController, MouseDeltaAccumulator, apply_native_mouse_delta,
};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    sync::Arc,
    time::Instant,
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use beta_viewer::loader;

const TERRAIN_HASH: &str = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf";
const HUD_NOTICE: &str = "HUD: saved hotbar items and saved health (no survival state)";
fn main() {
    if let Err(e) = run() {
        eprintln!("beta-viewer: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let terrain_path = terrain_path()?;
    let bytes = fs::read(&terrain_path)?;
    use sha2::Digest;
    let hash = format!("{:x}", sha2::Sha256::digest(&bytes));
    if hash != TERRAIN_HASH {
        return Err(format!(
            "terrain PNG hash mismatch at {}: {hash}",
            terrain_path.display()
        )
        .into());
    }
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
    if image.dimensions() != (256, 256) {
        return Err("terrain.png must be 256x256".into());
    }
    let client_jar = client_jar_path()?;
    let (gui_atlas, icons_atlas) = hud::load_atlases(&client_jar)?;
    let (inventory_atlas, items_atlas) = hud::load_inventory_atlases(&client_jar)?;
    let font_atlas = hud::load_font_atlas(&client_jar)?;
    let (input, source_pins) = copy_local_world()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let scene_path = root.join("tests/fixtures/view/scene.tsv");
    let (mut scene, metadata, scene_chunks, mappings) =
        loader::load_scene_for_preview_with_edit_data(
            &input,
            &terrain_path,
            &scene_path,
            &source_pins,
        )?;
    let palette_dir = terrain_path
        .parent()
        .ok_or("terrain atlas path has no parent for color palettes")?;
    let color_maps = beta_viewer::colors::load_color_maps(
        &palette_dir.join("grasscolor.png"),
        &palette_dir.join("foliagecolor.png"),
    )?;
    scene.enable_fancy_leaves();
    let (chunk_meshes, chunk_fluid_meshes) =
        build_chunk_mesh_cache(&scene, &color_maps, metadata.seed)?;
    let (visible_faces, unsupported_blocks) = chunk_mesh_stats(&chunk_meshes);
    let fluid_mesh = join_fluid_cache(&chunk_fluid_meshes);
    let light_dimension = match metadata.dimension {
        0 => LightDimension::Overworld,
        -1 => LightDimension::Nether,
        dimension => return Err(format!("unsupported saved light dimension {dimension}").into()),
    };
    let sky_subtraction = sky_subtraction(metadata.time, 1.0, light_dimension)?;
    let scene_bounds = scene.bounds();
    let interaction = BlockScene::new(
        BlockSceneBounds::new(
            scene_bounds.min_chunk_x,
            scene_bounds.min_chunk_z,
            scene_bounds.width,
            scene_bounds.depth,
        )?,
        scene_chunks
            .iter()
            .map(|chunk| BlockSceneChunk {
                x: chunk.x,
                z: chunk.z,
                data: chunk.data.clone(),
            })
            .collect(),
    )?;
    let spawn = metadata.spawn;
    let safe_spawn = scene.safe_spawn_near(spawn)?;
    let saved_inventory = saved_player::inventory_from_player(metadata.player.as_ref());
    let saved_health = saved_player::health_from_player(metadata.player.as_ref());
    if saved_inventory.unsupported_stacks > 0 {
        eprintln!(
            "beta-viewer: skipped {} unsupported saved inventory stack(s)",
            saved_inventory.unsupported_stacks
        );
    }
    let loaded_chunks = scene.loaded_chunks();
    eprintln!(
        "viewer scene: chunks={loaded_chunks}, opaque_vertices={}, water_vertices={}",
        chunk_mesh_vertex_count(&chunk_meshes),
        fluid_mesh.vertices.len()
    );
    let event = EventLoop::new()?;
    let mut app = App {
        visible_faces,
        unsupported_blocks,
        fluid_mesh,
        chunk_meshes,
        chunk_fluid_meshes,
        light_dimension,
        sky_subtraction,
        loaded_chunks,
        scene_chunks,
        mappings,
        block_scene: interaction,
        color_maps,
        biome_seed: metadata.seed,
        aimed_hit: None,
        target_status: "no target".to_owned(),
        atlas: image,
        gui_atlas,
        icons_atlas,
        inventory_atlas,
        items_atlas,
        font_atlas,
        window: None,
        gpu: None,
        camera: Camera::new(
            glam::Vec3::new(
                safe_spawn[0] as f32,
                safe_spawn[1] as f32 + 1.62,
                safe_spawn[2] as f32,
            ),
            0.0,
            0.0,
            70.0,
            0.05,
            128.0,
        )?,
        scene,
        controller: MotionController::new(MotionState::new(safe_spawn)),
        safe_spawn,
        controls: Controls::default(),
        inventory: saved_inventory.inventory,
        saved_health,
        wheel_input: hotbar_input::WheelAccumulator::default(),
        native_pitch_degrees: 0.0,
        mouse_delta: MouseDeltaAccumulator::default(),
        last: Instant::now(),
        captured: false,
        focused: true,
        inventory_screen: inventory_screen::InventoryScreen::default(),
        inventory_resume_capture: false,
        cursor_position: None,
    };
    event.run_app(&mut app)?;
    Ok(())
}
fn terrain_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let text = fs::read_to_string("config/local.toml")?;
    let mut in_paths = false;
    let mut reference = None;
    let mut is_directory = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_paths = l == "[paths]";
            continue;
        }
        if in_paths
            && let Some((k, v)) = l.split_once('=')
            && (k.trim() == "terrain_png" || k.trim() == "minecraft_reference")
        {
            reference = Some(v.trim().trim_matches('"').to_owned());
            if k.trim() == "terrain_png" {
                is_directory = false;
                break;
            }
            is_directory = true;
        }
    }
    let value =
        reference.ok_or("config requires paths.terrain_png or paths.minecraft_reference")?;
    let path = std::path::PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(path)
    };
    Ok(if is_directory {
        path.join("terrain.png")
    } else {
        path
    })
}

fn client_jar_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let text = fs::read_to_string("config/local.toml")?;
    let mut in_paths = false;
    let mut client_jar = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_paths = line == "[paths]";
            continue;
        }
        if in_paths
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "client_jar"
        {
            client_jar = Some(value.trim().trim_matches('"').to_owned());
            break;
        }
    }
    let value = client_jar.ok_or("config requires paths.client_jar for pinned GUI atlases")?;
    let path = std::path::PathBuf::from(value);
    let root = if path.is_absolute() {
        path
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(path)
    };
    Ok(root)
}

fn chunk_mesh_stats(cache: &ChunkMeshCache) -> (usize, usize) {
    cache.values().fold((0, 0), |(faces, unsupported), mesh| {
        (
            faces + mesh.visible_faces,
            unsupported + mesh.unsupported_blocks,
        )
    })
}

fn chunk_mesh_vertex_count(cache: &ChunkMeshCache) -> usize {
    cache.values().map(|mesh| mesh.vertices.len()).sum()
}

fn join_fluid_cache(cache: &BTreeMap<(i32, i32), FluidMesh>) -> FluidMesh {
    FluidMesh {
        vertices: ordered_chunk_meshes(cache)
            .into_iter()
            .flat_map(|(_, chunk_mesh)| chunk_mesh.vertices.iter().copied())
            .collect(),
    }
}

fn ordered_chunk_meshes<T>(cache: &BTreeMap<(i32, i32), T>) -> Vec<(&(i32, i32), &T)> {
    let mut chunks: Vec<_> = cache.iter().collect();
    chunks.sort_by_key(|((x, z), _)| (*z, *x));
    chunks
}

type ChunkMeshCache = BTreeMap<(i32, i32), Mesh>;
type ChunkFluidMeshCache = BTreeMap<(i32, i32), FluidMesh>;

fn build_chunk_mesh_cache(
    scene: &Scene,
    color_maps: &beta_viewer::colors::ColorMaps,
    biome_seed: i64,
) -> Result<(ChunkMeshCache, ChunkFluidMeshCache), Box<dyn std::error::Error>> {
    let mut opaque = BTreeMap::new();
    let mut fluids = BTreeMap::new();
    // `Scene` exposes its bounded rectangle and its selected mesh methods skip
    // unloaded positions, so cache only non-empty geometry.
    let bounds = scene.bounds();
    for z in bounds.min_chunk_z..bounds.min_chunk_z + i32::from(bounds.depth) {
        for x in bounds.min_chunk_x..bounds.min_chunk_x + i32::from(bounds.width) {
            let selected = [(x, z)];
            let mut climate_sampler = ClimateSampler::new(biome_seed);
            let mut climate_cache: HashMap<(i32, i32), Climate> = HashMap::new();
            let mesh = scene.mesh_chunks_with_tint_and_emission(
                beta_view::MeshLimits::default(),
                &selected,
                |id, metadata, block_x, block_z| {
                    let climate = *climate_cache
                        .entry((block_x, block_z))
                        .or_insert_with(|| climate_sampler.sample(block_x, block_z));
                    match id {
                        2 => grass_color(&color_maps.grass, climate).expect("valid climate"),
                        18 => foliage_color(&color_maps.foliage, climate, metadata)
                            .expect("valid climate"),
                        _ => 0x00ff_ffff,
                    }
                },
                beta_core::light::emission_level,
            )?;
            let fluid = scene.fluid_mesh_chunks(beta_view::MeshLimits::default(), &selected)?;
            if !mesh.vertices.is_empty() || mesh.unsupported_blocks != 0 {
                opaque.insert((x, z), mesh);
            }
            if !fluid.vertices.is_empty() {
                fluids.insert((x, z), fluid);
            }
        }
    }
    Ok((opaque, fluids))
}

fn copy_local_world()
-> Result<(std::path::PathBuf, Vec<loader::PreviewSourcePin>), Box<dyn std::error::Error>> {
    const SOURCE_NAMES: [&str; 5] = [
        "level.dat",
        "r.-1.-1.mcr",
        "r.-1.0.mcr",
        "r.0.-1.mcr",
        "r.0.0.mcr",
    ];
    let config = fs::read_to_string("config/local.toml")?;
    let mut in_paths = false;
    let mut prism = None;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_paths = line == "[paths]";
            continue;
        }
        if in_paths
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "prism_instance"
        {
            prism = Some(value.trim().trim_matches('"').to_owned());
            break;
        }
    }
    let prism = prism.ok_or("config requires paths.prism_instance")?;
    let source_world = std::path::PathBuf::from(prism).join("minecraft/saves/New World");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let manifest_path = local_world_manifest_path(&root)?;
    let manifest_meta = fs::symlink_metadata(&manifest_path)?;
    if manifest_meta.file_type().is_symlink() || !manifest_meta.is_file() {
        return Err("local world pin manifest must be a regular non-symlink file".into());
    }
    let manifest = fs::read_to_string(&manifest_path).map_err(|_| {
        format!(
            "local world pin manifest is missing: {}",
            manifest_path.display()
        )
    })?;
    let mut pins_by_name = HashMap::new();
    for (line_number, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split_ascii_whitespace().collect();
        if fields.len() != 3 {
            return Err(format!("malformed local world pin at line {}", line_number + 1).into());
        }
        let name = if fields[0] == "level.dat" {
            "level.dat"
        } else if let Some(region_name) = fields[0].strip_prefix("region/") {
            region_name
        } else if fields[0] == "level.dat_old" {
            continue;
        } else {
            return Err(format!("unexpected local world pin {}", fields[0]).into());
        };
        let size = fields[1].parse::<usize>()?;
        let max_size = if name == "level.dat" {
            16 * 1024 * 1024
        } else {
            64 * 1024 * 1024
        };
        if size == 0 || size > max_size {
            return Err(format!("local world pin size is outside limit for {name}").into());
        }
        if fields[2].len() != 64
            || !fields[2]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("invalid SHA-256 in local world pin for {name}").into());
        }
        let pin = loader::PreviewSourcePin {
            name: name.to_owned(),
            size,
            sha256: fields[2].to_owned(),
        };
        if pins_by_name.insert(name.to_owned(), pin).is_some() {
            return Err(format!("duplicate local world pin for {name}").into());
        }
    }
    if pins_by_name.len() != SOURCE_NAMES.len()
        || SOURCE_NAMES
            .iter()
            .any(|name| !pins_by_name.contains_key(*name))
    {
        return Err("local world pin manifest must contain exactly the five viewer inputs".into());
    }
    let source_pins: Vec<_> = SOURCE_NAMES
        .iter()
        .map(|name| {
            pins_by_name
                .get(*name)
                .expect("validated pin exists")
                .clone()
        })
        .collect();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let run = root
        .join("artifacts")
        .join("viewer")
        .join(format!("run-{}-{now}", std::process::id()));
    let input = run.join("input");
    fs::create_dir_all(input.join("region"))?;
    use sha2::Digest;
    let mut pin_manifest = String::new();
    for pin in &source_pins {
        let name = pin.name.as_str();
        let parent = if name == "level.dat" {
            source_world.clone()
        } else {
            source_world.join("region")
        };
        let source = parent.join(name);
        let metadata = fs::symlink_metadata(&source)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() as usize != pin.size
        {
            return Err(format!("unsafe or unexpected local world source {name}").into());
        }
        let data = fs::read(&source)?;
        let hash = format!("{:x}", sha2::Sha256::digest(&data));
        if hash != pin.sha256 {
            return Err(format!("pinned world source hash mismatch: {name}").into());
        }
        pin_manifest.push_str(&format!("{}\t{}\t{}\n", pin.name, pin.size, pin.sha256));
        let destination = if name == "level.dat" {
            input.join(name)
        } else {
            input.join("region").join(name)
        };
        fs::write(destination, data)?;
    }
    // Loader protocol requires all five top-level basenames. Keep region files at
    // the top level as the checked native input-copy format does.
    for name in SOURCE_NAMES.iter().filter(|name| **name != "level.dat") {
        let nested = input.join("region").join(name);
        fs::rename(&nested, input.join(name))?;
    }
    fs::remove_dir(input.join("region"))?;
    fs::write(
        input.join(".world-input-copy"),
        "WORLD-LOAD-001 copied inputs\n",
    )?;
    fs::write(
        input.join(".region-input-copy"),
        "REGION-001 copied inputs\n",
    )?;
    fs::write(input.join(".preview-source-hashes.tsv"), pin_manifest)?;
    Ok((input, source_pins))
}

/// Keep the user-owned world pin outside individual Git worktrees so `main`,
/// `codex`, and `agy` can all launch the same local preview configuration.
fn local_world_manifest_path(
    repository_root: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let repository_root = repository_root.canonicalize()?;
    let workspace_root = repository_root
        .parent()
        .ok_or("repository worktree has no shared workspace parent")?;
    Ok(workspace_root
        .join("artifacts")
        .join("user-world-postplay.sha256"))
}

/// Builds tile-isolated mip levels so minification does not alias and neighboring
/// terrain atlas tiles cannot bleed into one another at lower levels.
fn build_atlas_mips(base: &image::RgbaImage) -> Vec<image::RgbaImage> {
    debug_assert_eq!(base.dimensions(), (256, 256));
    let mut levels = vec![base.clone()];
    for level in 1..=4 {
        let previous = levels.last().expect("base atlas level exists");
        let tile_size = 16_u32 >> level;
        let source_tile_size = tile_size * 2;
        let dimension = 256_u32 >> level;
        let mut next = image::RgbaImage::new(dimension, dimension);
        for tile_y in 0..16 {
            for tile_x in 0..16 {
                for y in 0..tile_size {
                    for x in 0..tile_size {
                        let sx = tile_x * source_tile_size + x * 2;
                        let sy = tile_y * source_tile_size + y * 2;
                        let samples = [
                            previous.get_pixel(sx, sy),
                            previous.get_pixel(sx + 1, sy),
                            previous.get_pixel(sx, sy + 1),
                            previous.get_pixel(sx + 1, sy + 1),
                        ];
                        let mut rgba = [0_u8; 4];
                        for (channel, value) in rgba.iter_mut().enumerate() {
                            let total: u16 = samples
                                .iter()
                                .map(|pixel| u16::from(pixel.0[channel]))
                                .sum();
                            *value = ((total + 2) / 4) as u8;
                        }
                        next.put_pixel(
                            tile_x * tile_size + x,
                            tile_y * tile_size + y,
                            image::Rgba(rgba),
                        );
                    }
                }
            }
        }
        levels.push(next);
    }
    levels
}

/// Alpha-blended water must be submitted back-to-front. Geometry emits each
/// quad as two consecutive triangles, so sort whole six-vertex quads and keep
/// their native vertex order intact.
fn sort_fluid_quad_indices(
    vertices: &[beta_view::fluid::Vertex],
    camera: [f32; 3],
) -> Result<Vec<u32>, &'static str> {
    if !vertices.len().is_multiple_of(6) {
        return Err("fluid mesh vertex count is not a whole number of quads");
    }
    let quad_count = vertices.len() / 6;
    if vertices.len() > u32::MAX as usize {
        return Err("fluid mesh exceeds u32 index range");
    }
    let mut order: Vec<_> = vertices
        .chunks_exact(6)
        .enumerate()
        .map(|(index, quad)| {
            let center = std::array::from_fn::<_, 3, _>(|axis| {
                quad.iter().map(|vertex| vertex.position[axis]).sum::<f32>() / 6.0
            });
            let distance = (0..3)
                .map(|axis| (center[axis] - camera[axis]).powi(2))
                .sum::<f32>();
            (distance, index)
        })
        .collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut sorted = Vec::with_capacity(vertices.len());
    for (_, index) in order {
        let base = u32::try_from(index.checked_mul(6).ok_or("fluid index overflow")?)
            .map_err(|_| "fluid mesh exceeds u32 index range")?;
        sorted.extend(base..base + 6);
    }
    debug_assert_eq!(sorted.len(), quad_count * 6);
    Ok(sorted)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FluidFrameUploadBytes {
    vertex: usize,
    indices: usize,
}

fn fluid_frame_upload_bytes(index_count: usize) -> Option<FluidFrameUploadBytes> {
    Some(FluidFrameUploadBytes {
        vertex: 0,
        indices: index_count.checked_mul(std::mem::size_of::<u32>())?,
    })
}

fn fluid_gpu_layout(vertex_count: usize) -> Result<(usize, usize, u32), &'static str> {
    if !vertex_count.is_multiple_of(6) {
        return Err("fluid mesh vertex count is not a whole number of quads");
    }
    let vertex_bytes = vertex_count
        .checked_mul(std::mem::size_of::<[f32; 8]>())
        .ok_or("fluid vertex buffer size overflow")?;
    let index_bytes = vertex_count
        .checked_mul(std::mem::size_of::<u32>())
        .ok_or("fluid index buffer size overflow")?;
    let vertex_count_u32 =
        u32::try_from(vertex_count).map_err(|_| "fluid mesh exceeds u32 draw range")?;
    if vertex_bytes > 128 * 1024 * 1024 || index_bytes > 128 * 1024 * 1024 {
        return Err("fluid GPU buffers exceed viewer buffer limits");
    }
    Ok((vertex_bytes, index_bytes, vertex_count_u32))
}

fn prepare_fluid_gpu_buffers(
    device: &wgpu::Device,
    vertices: &[beta_view::fluid::Vertex],
) -> Result<(wgpu::Buffer, wgpu::Buffer, u32), Box<dyn std::error::Error>> {
    let (vertex_bytes, index_bytes, vertex_count) = fluid_gpu_layout(vertices.len())?;
    let packed: Vec<[f32; 8]> = vertices
        .iter()
        .map(|v| {
            [
                v.position[0],
                v.position[1],
                v.position[2],
                v.uv[0],
                v.uv[1],
                ((v.color >> 16) & 255) as f32 / 255.0,
                ((v.color >> 8) & 255) as f32 / 255.0,
                (v.color & 255) as f32 / 255.0,
            ]
        })
        .collect();
    debug_assert_eq!(packed.len() * std::mem::size_of::<[f32; 8]>(), vertex_bytes);
    let vertex_contents = if packed.is_empty() {
        bytemuck::cast_slice(&[[0.0_f32; 8]])
    } else {
        bytemuck::cast_slice(&packed)
    };
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("static transparent water vertices"),
        contents: vertex_contents,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sorted transparent water indices"),
        size: index_bytes.max(4) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    Ok((vertex_buffer, index_buffer, vertex_count))
}

struct App {
    visible_faces: usize,
    unsupported_blocks: usize,
    fluid_mesh: FluidMesh,
    chunk_meshes: ChunkMeshCache,
    chunk_fluid_meshes: ChunkFluidMeshCache,
    light_dimension: LightDimension,
    sky_subtraction: u8,
    scene: Scene,
    loaded_chunks: usize,
    scene_chunks: Vec<SceneChunk>,
    mappings: Vec<BlockMapping>,
    block_scene: BlockScene,
    color_maps: beta_viewer::colors::ColorMaps,
    biome_seed: i64,
    aimed_hit: Option<BlockHit>,
    target_status: String,
    atlas: image::RgbaImage,
    gui_atlas: image::RgbaImage,
    icons_atlas: image::RgbaImage,
    inventory_atlas: image::RgbaImage,
    items_atlas: image::RgbaImage,
    font_atlas: image::RgbaImage,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    camera: Camera,
    controller: MotionController,
    safe_spawn: [f64; 3],
    controls: Controls,
    inventory: Inventory,
    saved_health: i16,
    wheel_input: hotbar_input::WheelAccumulator,
    native_pitch_degrees: f32,
    mouse_delta: MouseDeltaAccumulator,
    last: Instant,
    captured: bool,
    focused: bool,
    inventory_screen: inventory_screen::InventoryScreen,
    inventory_resume_capture: bool,
    cursor_position: Option<(f32, f32)>,
}
struct Gpu {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    fluid_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    hud_shadow_pipeline: wgpu::RenderPipeline,
    chunk_meshes: BTreeMap<(i32, i32), GpuChunkMesh>,
    fluid_vertex: wgpu::Buffer,
    fluid_vertex_count: u32,
    fluid_index: wgpu::Buffer,
    fluid_index_count: u32,
    depth: wgpu::TextureView,
    bind: wgpu::BindGroup,
    gui_bind: wgpu::BindGroup,
    icons_bind: wgpu::BindGroup,
    inventory_bind: wgpu::BindGroup,
    items_bind: wgpu::BindGroup,
    hud_gui_vertex: wgpu::Buffer,
    hud_icons_vertex: wgpu::Buffer,
    inventory_vertex: wgpu::Buffer,
    inventory_terrain_icons_vertex: wgpu::Buffer,
    inventory_icons_vertex: wgpu::Buffer,
    cursor_terrain_vertex: wgpu::Buffer,
    cursor_items_vertex: wgpu::Buffer,
    hotbar_terrain_vertex: wgpu::Buffer,
    hotbar_items_vertex: wgpu::Buffer,
    count_shadow_vertex: wgpu::Buffer,
    count_foreground_vertex: wgpu::Buffer,
    font_bind: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
}

struct GpuChunkMesh {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
}

fn preferred_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|format| formats.contains(format))
}

fn saved_vertex_brightness(
    vertex: &Vertex,
    sky_subtraction: u8,
    dimension: LightDimension,
) -> Result<f32, beta_core::light::LightError> {
    sample_brightness(
        vertex.block_light,
        vertex.sky_light,
        sky_subtraction,
        vertex.minimum_light,
        dimension,
    )
}

fn native_packed_vertex_color(vertex: &Vertex, brightness: f32) -> [f32; 3] {
    std::array::from_fn(|channel| {
        let shaded = if vertex.tint_after_light {
            vertex.shade * brightness
        } else {
            vertex.shade * vertex.tint[channel]
        };
        let encoded = if vertex.tint_after_light {
            shaded * vertex.tint[channel]
        } else {
            shaded * brightness
        };
        let packed = (encoded * 255.0).trunc().clamp(0.0, 255.0) as u8;
        f32::from(packed) / 255.0
    })
}

fn pack_opaque_mesh(
    mesh: &Mesh,
    sky_subtraction: u8,
    dimension: LightDimension,
) -> Result<Vec<[f32; 8]>, Box<dyn std::error::Error>> {
    mesh.vertices
        .iter()
        .map(|vertex| {
            let brightness = saved_vertex_brightness(vertex, sky_subtraction, dimension)?;
            let color = native_packed_vertex_color(vertex, brightness);
            Ok([
                vertex.position[0],
                vertex.position[1],
                vertex.position[2],
                vertex.uv[0],
                vertex.uv[1],
                color[0],
                color[1],
                color[2],
            ])
        })
        .collect()
}

fn create_gpu_chunk_mesh(
    device: &wgpu::Device,
    mesh: &Mesh,
    sky_subtraction: u8,
    dimension: LightDimension,
) -> Result<GpuChunkMesh, Box<dyn std::error::Error>> {
    let packed = pack_opaque_mesh(mesh, sky_subtraction, dimension)?;
    let packed_bytes = packed.len() * std::mem::size_of::<[f32; 8]>();
    let index_bytes = mesh.indices.len() * std::mem::size_of::<u32>();
    if packed_bytes > 128 * 1024 * 1024 || index_bytes > 64 * 1024 * 1024 {
        return Err("chunk mesh exceeds viewer GPU buffer limits".into());
    }
    let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chunk opaque mesh vertices"),
        contents: if packed.is_empty() {
            bytemuck::cast_slice(&[[0.0_f32; 8]])
        } else {
            bytemuck::cast_slice(&packed)
        },
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chunk opaque mesh indices"),
        contents: if mesh.indices.is_empty() {
            bytemuck::cast_slice(&[0_u32])
        } else {
            bytemuck::cast_slice(&mesh.indices)
        },
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
    });
    Ok(GpuChunkMesh {
        vertex,
        index,
        index_count: mesh.indices.len() as u32,
    })
}

fn interaction_shape(mappings: &[BlockMapping], id: u8, metadata: u8) -> RayShape {
    if mappings.iter().any(|mapping| {
        mapping.key.id == id && mapping.key.metadata == metadata && mapping.is_renderable_cube()
    }) {
        RayShape::FullCube
    } else {
        match (id, metadata) {
            (44, 0) => RayShape::LowerSlab,
            (8, 0) => RayShape::Liquid,
            _ => RayShape::Unsupported,
        }
    }
}

fn dirty_chunk_union(edit: &[(i32, i32)], relight: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut result = edit.to_vec();
    result.extend_from_slice(relight);
    result.sort_unstable();
    result.dedup();
    result
}

#[cfg(test)]
fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn depth(device: &wgpu::Device, size: PhysicalSize<u32>) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth buffer"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
impl App {
    fn update_aimed_target(&mut self) {
        let direction = self.camera.forward();
        let origin = self.camera.position;
        let start = beta_core::geometry::Vec3::new(
            f64::from(origin.x),
            f64::from(origin.y),
            f64::from(origin.z),
        );
        let end = start.add_xyz(
            f64::from(direction.x) * beta_core::block::BLOCK_REACH,
            f64::from(direction.y) * beta_core::block::BLOCK_REACH,
            f64::from(direction.z) * beta_core::block::BLOCK_REACH,
        );
        let mappings = &self.mappings;
        let shapes = |id, metadata| interaction_shape(mappings, id, metadata);
        match self
            .block_scene
            .raycast_segment_with_shapes(start, end, false, &shapes)
        {
            Ok(RayResult::Hit(hit)) => {
                self.target_status = format!(
                    "target {} at ({},{},{}) face {}",
                    hit.id, hit.x, hit.y, hit.z, hit.face
                );
                self.aimed_hit = Some(hit);
            }
            Ok(RayResult::Unsupported(block)) => {
                self.target_status = format!(
                    "unsupported {} at ({},{},{})",
                    block.id, block.x, block.y, block.z
                );
                self.aimed_hit = None;
            }
            Ok(RayResult::Unloaded { x, y, z }) => {
                self.target_status = format!("unloaded target at ({x},{y},{z})");
                self.aimed_hit = None;
            }
            Ok(RayResult::Miss) => {
                self.target_status = "no target".to_owned();
                self.aimed_hit = None;
            }
            Err(error) => {
                self.target_status = format!("targeting error: {error}");
                self.aimed_hit = None;
            }
        }
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "Beta 1.7.3 sandbox — LMB break / RMB place | {} | {}",
                self.target_status, HUD_NOTICE
            ));
        }
    }

    fn rebuild_dirty_chunks(
        &mut self,
        dirty: &[(i32, i32)],
        next_block_scene: BlockScene,
        next_scene_chunks: Vec<SceneChunk>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut next_scene = self.scene.clone();
        for &(x, z) in dirty {
            if let Some(chunk) = next_scene_chunks
                .iter()
                .find(|chunk| chunk.x == x && chunk.z == z)
            {
                next_scene.replace_chunk(chunk.clone())?;
            }
        }
        let mut next_chunk_meshes = self.chunk_meshes.clone();
        let mut next_chunk_fluid_meshes = self.chunk_fluid_meshes.clone();
        let scene = &next_scene;
        for &(x, z) in dirty {
            let selected = [(x, z)];
            let mut climate_sampler = ClimateSampler::new(self.biome_seed);
            let mut climate_cache: HashMap<(i32, i32), Climate> = HashMap::new();
            let mesh = scene.mesh_chunks_with_tint_and_emission(
                beta_view::MeshLimits::default(),
                &selected,
                |id, metadata, block_x, block_z| {
                    let climate = *climate_cache
                        .entry((block_x, block_z))
                        .or_insert_with(|| climate_sampler.sample(block_x, block_z));
                    match id {
                        2 => grass_color(&self.color_maps.grass, climate)
                            .expect("native climate sampler returns a valid palette climate"),
                        18 => foliage_color(&self.color_maps.foliage, climate, metadata)
                            .expect("native climate sampler returns a valid palette climate"),
                        _ => 0x00ff_ffff,
                    }
                },
                beta_core::light::emission_level,
            )?;
            if mesh.vertices.is_empty() && mesh.unsupported_blocks == 0 {
                next_chunk_meshes.remove(&(x, z));
            } else {
                next_chunk_meshes.insert((x, z), mesh);
            }
            let fluid = scene.fluid_mesh_chunks(beta_view::MeshLimits::default(), &selected)?;
            if fluid.vertices.is_empty() {
                next_chunk_fluid_meshes.remove(&(x, z));
            } else {
                next_chunk_fluid_meshes.insert((x, z), fluid);
            }
        }
        let next_fluid_mesh = join_fluid_cache(&next_chunk_fluid_meshes);
        self.upload_dirty_meshes(dirty, &next_chunk_meshes, &next_fluid_mesh)?;
        self.scene = next_scene;
        self.scene_chunks = next_scene_chunks;
        self.block_scene = next_block_scene;
        self.chunk_meshes = next_chunk_meshes;
        self.chunk_fluid_meshes = next_chunk_fluid_meshes;
        (self.visible_faces, self.unsupported_blocks) = chunk_mesh_stats(&self.chunk_meshes);
        self.fluid_mesh = next_fluid_mesh;
        Ok(())
    }

    fn upload_dirty_meshes(
        &mut self,
        dirty: &[(i32, i32)],
        chunk_meshes: &BTreeMap<(i32, i32), Mesh>,
        fluid_mesh: &FluidMesh,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chunk_updates: Vec<_> = dirty
            .iter()
            .map(|key| (*key, chunk_meshes.get(key).cloned()))
            .collect();
        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        // Prepare all replacement GPU data first. Any fallible mesh/layout
        // validation leaves the currently published GPU state untouched.
        let mut prepared_chunks = Vec::with_capacity(chunk_updates.len());
        for (key, mesh) in chunk_updates {
            let prepared = if let Some(mesh) = mesh.filter(|mesh| !mesh.indices.is_empty()) {
                Some(create_gpu_chunk_mesh(
                    &gpu.device,
                    &mesh,
                    self.sky_subtraction,
                    self.light_dimension,
                )?)
            } else {
                None
            };
            prepared_chunks.push((key, prepared));
        }
        let (fluid_vertex, fluid_index, fluid_vertex_count) =
            prepare_fluid_gpu_buffers(&gpu.device, &fluid_mesh.vertices)?;

        for (key, mesh) in prepared_chunks {
            if let Some(mesh) = mesh {
                gpu.chunk_meshes.insert(key, mesh);
            } else {
                gpu.chunk_meshes.remove(&key);
            }
        }
        // Assign the validated fluid buffers as one coherent set after both
        // replacements have been created successfully.
        gpu.fluid_vertex = fluid_vertex;
        gpu.fluid_index = fluid_index;
        gpu.fluid_vertex_count = fluid_vertex_count;
        gpu.fluid_index_count = 0;
        Ok(())
    }

    fn interact(&mut self, breaking: bool) -> Result<BlockEdit, Box<dyn std::error::Error>> {
        self.update_aimed_target();
        let hit = self.aimed_hit.ok_or("aim at a supported full cube")?;
        let mappings = &self.mappings;
        let supported = |id, metadata| {
            mappings.iter().any(|mapping| {
                mapping.key.id == id
                    && mapping.key.metadata == metadata
                    && mapping.is_renderable_cube()
            })
        };
        let mut next_block_scene = self.block_scene.clone();
        let edit = if breaking {
            next_block_scene.break_block(hit, &supported)?
        } else {
            next_block_scene.place_block(hit, 1, 0, &supported)?
        };
        let (x, y, z) = edit.changed;
        let relight = next_block_scene.relight(
            edit.changed,
            beta_core::block::MAX_RELIGHT_CELLS,
            |id, _metadata| {
                let emission = beta_core::light::emission_level(id);
                (if id == 0 { 0 } else { 255 }, emission)
            },
        )?;
        let dirty_chunks = dirty_chunk_union(&edit.dirty_chunks, &relight.dirty_chunks);
        let (chunk_x, local_x) = split_block_coord(x);
        let (chunk_z, local_z) = split_block_coord(z);
        let value: BlockData = next_block_scene
            .block(x, y, z)?
            .ok_or("edited cell is no longer loaded")?;
        let mut next_scene_chunks = self.scene_chunks.clone();
        let chunk = next_scene_chunks
            .iter_mut()
            .find(|chunk| chunk.x == chunk_x && chunk.z == chunk_z)
            .ok_or("edited chunk is not in the disposable scene")?;
        chunk
            .data
            .set(local_x, y as u8, local_z, value)
            .map_err(|error| format!("chunk update failed: {error}"))?;
        for &((lx_world, ly, lz_world), block_light, sky_light) in &relight.light_updates {
            let (cx, lx) = split_block_coord(lx_world);
            let (cz, lz) = split_block_coord(lz_world);
            let target = next_scene_chunks
                .iter_mut()
                .find(|candidate| candidate.x == cx && candidate.z == cz)
                .ok_or("relit cell is not in the disposable scene")?;
            let mut state = target.data.get(lx, ly as u8, lz)?;
            state.block_light = block_light;
            state.sky_light = sky_light;
            target.data.set(lx, ly as u8, lz, state)?;
        }
        self.rebuild_dirty_chunks(&dirty_chunks, next_block_scene, next_scene_chunks)?;
        self.aimed_hit = None;
        self.target_status = format!(
            "{} block at ({x},{y},{z}); rebuilt {} dirty chunks",
            if breaking { "broke" } else { "placed" },
            dirty_chunks.len()
        );
        Ok(edit)
    }

    fn init(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let w = Arc::new(
            el.create_window(
                Window::default_attributes()
                    .with_title("Beta 1.7.3 — fixed scene")
                    .with_inner_size(PhysicalSize::new(1280, 720)),
            )?,
        );
        w.set_title(&format!(
            "Beta 1.7.3 sandbox — WASD/Space/Shift | F flight, R reset, Enter mouse | LMB break, RMB place | {} | chunks {}, faces {}, water vertices {}, unsupported {}",
            HUD_NOTICE,
            self.loaded_chunks, self.visible_faces, self.fluid_mesh.vertices.len(), self.unsupported_blocks
        ));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(el.owned_display_handle()),
        ));
        let surface = instance.create_surface(w.clone())?;
        let (device, queue, config) = pollster::block_on(async {
            let a = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await?;
            let (d, q) = a
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("beta-viewer"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                })
                .await?;
            let mut c = surface
                .get_default_config(&a, 1280, 720)
                .ok_or("surface has no format")?;
            let capabilities = surface.get_capabilities(&a);
            c.format = preferred_surface_format(&capabilities.formats)
                .ok_or("surface supports no 8-bit RGBA/BGRA unorm or sRGB format")?;
            Ok::<_, Box<dyn std::error::Error>>((d, q, c))
        })?;
        surface.configure(&device, &config);
        let atlas_mips = build_atlas_mips(&self.atlas);
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Beta terrain.png"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: atlas_mips.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, mip) in atlas_mips.iter().enumerate() {
            let (width, height) = mip.dimensions();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mip.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = tex.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 4.0,
            ..Default::default()
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera matrix"),
            contents: bytemuck::cast_slice(
                &self.camera.view_projection(1280.0 / 720.0)?.to_cols_array(),
            ),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas/camera layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas bind"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let output_srgb_constant = [(
            "OUTPUT_SRGB",
            if config.format.is_srgb() { 1.0 } else { 0.0 },
        )];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &output_srgb_constant,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let fluid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transparent water pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_fluid"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &output_srgb_constant,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen-space native HUD pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_hud"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_hud"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &output_srgb_constant,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            // HUD is drawn in the same color pass as the world. Keep an
            // attachment-compatible depth state but bypass world occlusion.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let hud_shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen-space native HUD count shadow pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_hud"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_hud_shadow"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &output_srgb_constant,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let create_hud_view = |label: &'static str, image: &image::RgbaImage| {
            let (width, height) = image.dimensions();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                image.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            texture.create_view(&Default::default())
        };
        let gui_view = create_hud_view("Beta gui/gui.png", &self.gui_atlas);
        let icons_view = create_hud_view("Beta gui/icons.png", &self.icons_atlas);
        let inventory_view = create_hud_view("Beta gui/inventory.png", &self.inventory_atlas);
        let items_view = create_hud_view("Beta gui/items.png", &self.items_atlas);
        let font_view = create_hud_view("Beta font/default.png", &self.font_atlas);
        let make_hud_bind = |label, texture_view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: camera_buffer.as_entire_binding(),
                    },
                ],
            })
        };
        let gui_bind = make_hud_bind("native gui atlas bind", &gui_view);
        let icons_bind = make_hud_bind("native icons atlas bind", &icons_view);
        let inventory_bind = make_hud_bind("native inventory atlas bind", &inventory_view);
        let items_bind = make_hud_bind("native items atlas bind", &items_view);
        let font_bind = make_hud_bind("native font atlas bind", &font_view);
        let hud_gui_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native hotbar screen-space vertices"),
            size: 12 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hud_icons_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native hearts and crosshair screen-space vertices"),
            size: 126 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let inventory_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native inventory panel screen-space vertices"),
            size: 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let inventory_terrain_icons_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native inventory terrain item vertices"),
            size: 36 * 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let inventory_icons_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native inventory item screen-space vertices"),
            size: 36 * 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cursor_terrain_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native inventory cursor terrain vertices"),
            size: 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cursor_items_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native inventory cursor item vertices"),
            size: 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hotbar_terrain_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native hotbar terrain item vertices"),
            size: 9 * 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hotbar_items_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native hotbar item vertices"),
            size: 9 * 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let count_shadow_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native item count shadow vertices"),
            size: 36 * 12 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let count_foreground_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native item count foreground vertices"),
            size: 36 * 12 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut gpu_chunk_meshes = BTreeMap::new();
        for (&key, mesh) in &self.chunk_meshes {
            if !mesh.indices.is_empty() {
                gpu_chunk_meshes.insert(
                    key,
                    create_gpu_chunk_mesh(
                        &device,
                        mesh,
                        self.sky_subtraction,
                        self.light_dimension,
                    )?,
                );
            }
        }
        let (fluid_vertex, fluid_index, fluid_vertex_count) =
            prepare_fluid_gpu_buffers(&device, &self.fluid_mesh.vertices)?;
        let dep = depth(&device, w.inner_size());
        self.gpu = Some(Gpu {
            instance,
            surface,
            device,
            queue,
            config,
            pipeline,
            fluid_pipeline,
            hud_pipeline,
            hud_shadow_pipeline,
            chunk_meshes: gpu_chunk_meshes,
            fluid_vertex,
            fluid_vertex_count,
            fluid_index,
            fluid_index_count: 0,
            depth: dep,
            bind,
            gui_bind,
            icons_bind,
            inventory_bind,
            items_bind,
            hud_gui_vertex,
            hud_icons_vertex,
            inventory_vertex,
            inventory_terrain_icons_vertex,
            inventory_icons_vertex,
            cursor_terrain_vertex,
            cursor_items_vertex,
            hotbar_terrain_vertex,
            hotbar_items_vertex,
            count_shadow_vertex,
            count_foreground_vertex,
            font_bind,
            camera_buffer,
        });
        self.window = Some(w);
        Ok(())
    }
    fn frame(&mut self) {
        let window = self.window.clone();
        self.update_aimed_target();
        let Some(g) = self.gpu.as_mut() else { return };
        let now = Instant::now();
        let dt = now.duration_since(self.last);
        self.last = now;
        if self.focused && !self.inventory_screen.is_open() {
            self.controller.accumulate(dt);
            while self.controller.ticks_due() > 0 {
                if self.controller.debug_flight {
                    self.camera.position =
                        glam::Vec3::from_array(self.controller.flight_position());
                    let _ = self.camera.move_local(
                        u8::from(self.controls.forward) as f32
                            - u8::from(self.controls.backward) as f32,
                        u8::from(self.controls.right) as f32 - u8::from(self.controls.left) as f32,
                        u8::from(self.controls.jump) as f32 - u8::from(self.controls.sneak) as f32,
                        8.0,
                        0.05,
                    );
                    self.controller.flight_tick(self.camera.position.to_array());
                } else {
                    if let Err(error) = self.controller.tick(&self.scene, self.controls) {
                        eprintln!("beta-viewer motion query failed: {error}");
                        self.controller.clear_clock();
                        break;
                    }
                }
            }
            if self.controller.debug_flight {
                let p = self.controller.interpolated_flight_position();
                self.camera.position = glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
            } else {
                let p = self.controller.interpolated_position();
                self.camera.position =
                    glam::Vec3::new(p[0] as f32, p[1] as f32 + 1.62, p[2] as f32);
                self.camera.yaw = -self.controller.state.yaw_degrees.to_radians();
            }
        }
        let fluid_indices = match sort_fluid_quad_indices(
            &self.fluid_mesh.vertices,
            self.camera.position.to_array(),
        ) {
            Ok(indices) => indices,
            Err(error) => {
                eprintln!("beta-viewer fluid sort failed: {error}");
                Vec::new()
            }
        };
        if !fluid_indices.is_empty() {
            if let Some(upload_bytes) = fluid_frame_upload_bytes(fluid_indices.len()) {
                let contents = bytemuck::cast_slice(&fluid_indices);
                debug_assert_eq!(contents.len(), upload_bytes.indices);
                debug_assert_eq!(upload_bytes.vertex, 0);
                g.queue.write_buffer(&g.fluid_index, 0, contents);
            } else {
                eprintln!("beta-viewer fluid index upload size overflow");
            }
        }
        g.fluid_index_count = match u32::try_from(fluid_indices.len()) {
            Ok(count) => count,
            Err(_) => {
                eprintln!("beta-viewer fluid index count exceeds u32 draw range");
                0
            }
        };
        if let Ok(m) = self
            .camera
            .view_projection(g.config.width as f32 / g.config.height as f32)
        {
            g.queue.write_buffer(
                &g.camera_buffer,
                0,
                bytemuck::cast_slice(&m.to_cols_array()),
            )
        }
        let show_hud = !self.inventory_screen.is_open();
        let (hud_gui, hud_icons) = hud::native_vertices_with_selection(
            g.config.width,
            g.config.height,
            hud::gui_scale(g.config.width, g.config.height),
            self.inventory.selected_hotbar(),
            self.saved_health,
        );
        g.queue
            .write_buffer(&g.hud_gui_vertex, 0, bytemuck::cast_slice(&hud_gui));
        g.queue
            .write_buffer(&g.hud_icons_vertex, 0, bytemuck::cast_slice(&hud_icons));
        let inventory_open = self.inventory_screen.is_open();
        let (inventory_terrain_count, inventory_icon_count) = if inventory_open {
            match inventory_screen::native_vertices(
                &self.inventory,
                g.config.width,
                g.config.height,
                hud::gui_scale(g.config.width, g.config.height),
            ) {
                Ok(vertices) => {
                    g.queue.write_buffer(
                        &g.inventory_vertex,
                        0,
                        bytemuck::cast_slice(&vertices.panel),
                    );
                    if !vertices.items.is_empty() {
                        g.queue.write_buffer(
                            &g.inventory_icons_vertex,
                            0,
                            bytemuck::cast_slice(&vertices.items),
                        );
                    }
                    if !vertices.terrain_items.is_empty() {
                        g.queue.write_buffer(
                            &g.inventory_terrain_icons_vertex,
                            0,
                            bytemuck::cast_slice(&vertices.terrain_items),
                        );
                    }
                    (
                        vertices.terrain_items.len() as u32,
                        vertices.items.len() as u32,
                    )
                }
                Err(error) => {
                    eprintln!("beta-viewer inventory render rejected: {error}");
                    (0, 0)
                }
            }
        } else {
            (0, 0)
        };
        let (hotbar_terrain_count, hotbar_items_count) = if show_hud {
            let (terrain, items) = inventory_screen::hotbar_vertices(
                &self.inventory,
                g.config.width,
                g.config.height,
                hud::gui_scale(g.config.width, g.config.height),
            );
            if !terrain.is_empty() {
                g.queue
                    .write_buffer(&g.hotbar_terrain_vertex, 0, bytemuck::cast_slice(&terrain));
            }
            if !items.is_empty() {
                g.queue
                    .write_buffer(&g.hotbar_items_vertex, 0, bytemuck::cast_slice(&items));
            }
            (terrain.len() as u32, items.len() as u32)
        } else {
            (0, 0)
        };
        let (cursor_terrain_count, cursor_items_count, cursor_counts) = if inventory_open {
            if let Some(mouse) = self.cursor_position {
                let layout = inventory_screen::native_layout(
                    g.config.width,
                    g.config.height,
                    hud::gui_scale(g.config.width, g.config.height),
                );
                let (terrain, items, counts) = inventory_screen::cursor_vertices(
                    self.inventory_screen.cursor_stack(),
                    g.config.width,
                    g.config.height,
                    hud::gui_scale(g.config.width, g.config.height),
                    mouse,
                    (layout.panel.x, layout.panel.y),
                );
                if !terrain.is_empty() {
                    g.queue.write_buffer(
                        &g.cursor_terrain_vertex,
                        0,
                        bytemuck::cast_slice(&terrain),
                    );
                }
                if !items.is_empty() {
                    g.queue
                        .write_buffer(&g.cursor_items_vertex, 0, bytemuck::cast_slice(&items));
                }
                (terrain.len() as u32, items.len() as u32, counts)
            } else {
                (
                    0,
                    0,
                    inventory_screen::CountVertices {
                        shadow: Vec::new(),
                        foreground: Vec::new(),
                    },
                )
            }
        } else {
            (
                0,
                0,
                inventory_screen::CountVertices {
                    shadow: Vec::new(),
                    foreground: Vec::new(),
                },
            )
        };
        let count_pass = if inventory_open || show_hud {
            let counts = inventory_screen::count_vertices(
                &self.inventory,
                g.config.width,
                g.config.height,
                hud::gui_scale(g.config.width, g.config.height),
                inventory_open,
            );
            if !counts.shadow.is_empty() {
                g.queue.write_buffer(
                    &g.count_shadow_vertex,
                    0,
                    bytemuck::cast_slice(&counts.shadow),
                );
            }
            if !counts.foreground.is_empty() {
                g.queue.write_buffer(
                    &g.count_foreground_vertex,
                    0,
                    bytemuck::cast_slice(&counts.foreground),
                );
            }
            let mut counts = counts;
            counts.shadow.extend(cursor_counts.shadow);
            counts.foreground.extend(cursor_counts.foreground);
            if !counts.shadow.is_empty() {
                g.queue.write_buffer(
                    &g.count_shadow_vertex,
                    0,
                    bytemuck::cast_slice(&counts.shadow),
                );
            }
            if !counts.foreground.is_empty() {
                g.queue.write_buffer(
                    &g.count_foreground_vertex,
                    0,
                    bytemuck::cast_slice(&counts.foreground),
                );
            }
            (counts.shadow.len() as u32, counts.foreground.len() as u32)
        } else {
            (0, 0)
        };
        match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                let view = frame.texture.create_view(&Default::default());
                let mut e = g.device.create_command_encoder(&Default::default());
                {
                    let mut p = e.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Beta scene"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.46,
                                    g: 0.66,
                                    b: 0.9,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &g.depth,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    p.set_pipeline(&g.pipeline);
                    p.set_bind_group(0, &g.bind, &[]);
                    for chunk in g.chunk_meshes.values() {
                        p.set_vertex_buffer(0, chunk.vertex.slice(..));
                        p.set_index_buffer(chunk.index.slice(..), wgpu::IndexFormat::Uint32);
                        p.draw_indexed(0..chunk.index_count, 0, 0..1);
                    }
                    if g.fluid_vertex_count > 0 {
                        p.set_pipeline(&g.fluid_pipeline);
                        p.set_vertex_buffer(0, g.fluid_vertex.slice(..));
                        p.set_index_buffer(g.fluid_index.slice(..), wgpu::IndexFormat::Uint32);
                        p.draw_indexed(0..g.fluid_index_count, 0, 0..1);
                    }
                    if inventory_open {
                        p.set_pipeline(&g.hud_pipeline);
                        p.set_bind_group(0, &g.inventory_bind, &[]);
                        p.set_vertex_buffer(0, g.inventory_vertex.slice(..));
                        p.draw(0..6, 0..1);
                        if inventory_terrain_count > 0 {
                            p.set_bind_group(0, &g.bind, &[]);
                            p.set_vertex_buffer(0, g.inventory_terrain_icons_vertex.slice(..));
                            p.draw(0..inventory_terrain_count, 0..1);
                        }
                        if inventory_icon_count > 0 {
                            p.set_bind_group(0, &g.items_bind, &[]);
                            p.set_vertex_buffer(0, g.inventory_icons_vertex.slice(..));
                            p.draw(0..inventory_icon_count, 0..1);
                        }
                        if cursor_terrain_count > 0 {
                            p.set_bind_group(0, &g.bind, &[]);
                            p.set_vertex_buffer(0, g.cursor_terrain_vertex.slice(..));
                            p.draw(0..cursor_terrain_count, 0..1);
                        }
                        if cursor_items_count > 0 {
                            p.set_bind_group(0, &g.items_bind, &[]);
                            p.set_vertex_buffer(0, g.cursor_items_vertex.slice(..));
                            p.draw(0..cursor_items_count, 0..1);
                        }
                        if count_pass.0 > 0 {
                            p.set_pipeline(&g.hud_shadow_pipeline);
                            p.set_bind_group(0, &g.font_bind, &[]);
                            p.set_vertex_buffer(0, g.count_shadow_vertex.slice(..));
                            p.draw(0..count_pass.0, 0..1);
                            p.set_pipeline(&g.hud_pipeline);
                            p.set_vertex_buffer(0, g.count_foreground_vertex.slice(..));
                            p.draw(0..count_pass.1, 0..1);
                        }
                    } else if show_hud {
                        p.set_pipeline(&g.hud_pipeline);
                        p.set_bind_group(0, &g.gui_bind, &[]);
                        p.set_vertex_buffer(0, g.hud_gui_vertex.slice(..));
                        p.draw(0..12, 0..1);
                        p.set_bind_group(0, &g.icons_bind, &[]);
                        p.set_vertex_buffer(0, g.hud_icons_vertex.slice(..));
                        p.draw(0..126, 0..1);
                        if hotbar_terrain_count > 0 {
                            p.set_bind_group(0, &g.bind, &[]);
                            p.set_vertex_buffer(0, g.hotbar_terrain_vertex.slice(..));
                            p.draw(0..hotbar_terrain_count, 0..1);
                        }
                        if hotbar_items_count > 0 {
                            p.set_bind_group(0, &g.items_bind, &[]);
                            p.set_vertex_buffer(0, g.hotbar_items_vertex.slice(..));
                            p.draw(0..hotbar_items_count, 0..1);
                        }
                        if count_pass.0 > 0 {
                            p.set_pipeline(&g.hud_shadow_pipeline);
                            p.set_bind_group(0, &g.font_bind, &[]);
                            p.set_vertex_buffer(0, g.count_shadow_vertex.slice(..));
                            p.draw(0..count_pass.0, 0..1);
                            p.set_pipeline(&g.hud_pipeline);
                            p.set_vertex_buffer(0, g.count_foreground_vertex.slice(..));
                            p.draw(0..count_pass.1, 0..1);
                        }
                    }
                }
                g.queue.submit([e.finish()]);
                g.queue.present(frame)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Some(window) = window {
                    match g.instance.create_surface(window) {
                        Ok(surface) => {
                            surface.configure(&g.device, &g.config);
                            g.surface = surface;
                        }
                        Err(error) => {
                            eprintln!("beta-viewer could not recreate lost surface: {error}")
                        }
                    }
                }
            }
            wgpu::CurrentSurfaceTexture::Outdated => g.surface.configure(&g.device, &g.config),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {}
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("beta-viewer surface validation failure")
            }
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, e: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(err) = self.init(e)
        {
            eprintln!("beta-viewer initialization failed: {err}");
            e.exit()
        }
    }
    fn window_event(&mut self, e: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => e.exit(),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.last = Instant::now();
                self.mouse_delta.clear();
                if !focused {
                    let _ = self.inventory_screen.focus_lost(&mut self.inventory);
                    self.controls = Controls::default();
                    self.wheel_input.clear();
                    self.controller.clear_clock();
                    self.controller
                        .sync_flight_position(self.camera.position.to_array());
                    self.captured = false;
                    if let Some(w) = &self.window {
                        w.set_cursor_visible(true);
                        let _ = w.set_cursor_grab(CursorGrabMode::None);
                    }
                }
            }
            WindowEvent::Resized(sz) => {
                if let Some(g) = self.gpu.as_mut()
                    && sz.width > 0
                    && sz.height > 0
                {
                    g.config.width = sz.width;
                    g.config.height = sz.height;
                    g.surface.configure(&g.device, &g.config);
                    g.depth = depth(&g.device, sz)
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x as f32, position.y as f32));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if self.focused && self.inventory_screen.is_open() => {
                let Some((x, y)) = self.cursor_position else {
                    return;
                };
                let Some(g) = self.gpu.as_ref() else {
                    return;
                };
                let Some(slot) = self.inventory_screen.slot_at(
                    g.config.width,
                    g.config.height,
                    hud::gui_scale(g.config.width, g.config.height),
                    x,
                    y,
                ) else {
                    return;
                };
                let right = match button {
                    MouseButton::Left => false,
                    MouseButton::Right => true,
                    _ => return,
                };
                let _ = self
                    .inventory_screen
                    .click(&mut self.inventory, slot, right);
            }
            WindowEvent::MouseInput { state, button, .. }
                if state == ElementState::Pressed
                    && self.focused
                    && self.captured
                    && !self.inventory_screen.is_open() =>
            {
                let breaking = match button {
                    MouseButton::Left => true,
                    MouseButton::Right => false,
                    _ => return,
                };
                if let Err(error) = self.interact(breaking) {
                    self.target_status = format!("action rejected: {error}");
                    if let Some(window) = &self.window {
                        window.set_title(&format!(
                            "Beta 1.7.3 sandbox — LMB break / RMB place | {} | {}",
                            self.target_status, HUD_NOTICE
                        ));
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. }
                if self.focused && !self.inventory_screen.is_open() =>
            {
                let step = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        self.wheel_input.line_event(f64::from(y))
                    }
                    winit::event::MouseScrollDelta::PixelDelta(position) => {
                        self.wheel_input.pixel_event(position.y)
                    }
                };
                if let Some(step) = step {
                    hotbar_input::wheel_step(&mut self.inventory, step);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if !self.focused {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    let transition = self.inventory_screen.key_event(
                        code == KeyCode::KeyE,
                        code == KeyCode::Escape,
                        pressed,
                        event.repeat,
                        self.focused,
                    );
                    if matches!(transition, inventory_screen::Transition::Opened) {
                        self.controls = Controls::default();
                        self.inventory_resume_capture = self.captured;
                        self.captured = false;
                        self.mouse_delta.clear();
                        if let Some(w) = &self.window {
                            w.set_cursor_visible(true);
                            let _ = w.set_cursor_grab(CursorGrabMode::None);
                        }
                        return;
                    }
                    if matches!(transition, inventory_screen::Transition::Closed) {
                        let cursor_returned =
                            self.inventory_screen.return_cursor(&mut self.inventory);
                        self.controls = Controls::default();
                        self.mouse_delta.clear();
                        if cursor_returned
                            && self.inventory_resume_capture
                            && let Some(w) = &self.window
                            && w.set_cursor_grab(CursorGrabMode::Locked).is_ok()
                        {
                            w.set_cursor_visible(false);
                            self.captured = true;
                        }
                        self.inventory_resume_capture = false;
                        return;
                    }
                    if matches!(transition, inventory_screen::Transition::ReleasedCapture) {
                        self.captured = false;
                        self.mouse_delta.clear();
                        if let Some(w) = &self.window {
                            w.set_cursor_visible(true);
                            let _ = w.set_cursor_grab(CursorGrabMode::None);
                        }
                        return;
                    }
                    if self.inventory_screen.is_open() {
                        return;
                    }
                    let number = match code {
                        KeyCode::Digit1 => Some(1),
                        KeyCode::Digit2 => Some(2),
                        KeyCode::Digit3 => Some(3),
                        KeyCode::Digit4 => Some(4),
                        KeyCode::Digit5 => Some(5),
                        KeyCode::Digit6 => Some(6),
                        KeyCode::Digit7 => Some(7),
                        KeyCode::Digit8 => Some(8),
                        KeyCode::Digit9 => Some(9),
                        _ => None,
                    };
                    if let Some(number) = number {
                        hotbar_input::number_key(
                            &mut self.inventory,
                            number,
                            pressed,
                            event.repeat,
                            self.focused,
                        );
                    }
                    match code {
                        KeyCode::KeyW => self.controls.forward = pressed,
                        KeyCode::KeyS => self.controls.backward = pressed,
                        KeyCode::KeyA => self.controls.left = pressed,
                        KeyCode::KeyD => self.controls.right = pressed,
                        KeyCode::Space => self.controls.jump = pressed,
                        KeyCode::ShiftLeft => self.controls.sneak = pressed,
                        _ => {}
                    }
                    if pressed && !event.repeat && code == KeyCode::KeyF {
                        if self.controller.debug_flight {
                            self.controller.debug_flight = false;
                            self.controller.land_from_flight(
                                self.camera.position.to_array(),
                                -self.camera.yaw.to_degrees(),
                            );
                        } else {
                            self.controller
                                .begin_flight(self.camera.position.to_array());
                        }
                    }
                    if pressed && !event.repeat && code == KeyCode::KeyR {
                        eprintln!("viewer: reset key R pressed; returning to safe spawn");
                        self.controller.debug_flight = false;
                        self.controller.reset(self.safe_spawn);
                        self.camera.position = glam::Vec3::new(
                            self.safe_spawn[0] as f32,
                            self.safe_spawn[1] as f32 + 1.62,
                            self.safe_spawn[2] as f32,
                        );
                        self.camera.yaw = -self.controller.state.yaw_degrees.to_radians();
                    }
                    if event.state == ElementState::Pressed
                        && code == KeyCode::Enter
                        && let Some(w) = &self.window
                    {
                        self.mouse_delta.clear();
                        let _ = w.set_cursor_grab(CursorGrabMode::Locked);
                        w.set_cursor_visible(false);
                        self.captured = true
                    }
                }
            }
            _ => {}
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent) {
        if self.captured
            && let DeviceEvent::MouseMotion { delta } = event
        {
            let (dx, dy) = self.mouse_delta.take_integer_deltas(delta.0, delta.1);
            if dx == 0 && dy == 0 {
                return;
            }
            let (native_yaw, native_pitch) = apply_native_mouse_delta(
                self.controller.state.yaw_degrees,
                self.native_pitch_degrees,
                dx,
                dy,
            );
            self.controller.state.yaw_degrees = native_yaw;
            self.native_pitch_degrees = native_pitch;
            self.camera.yaw = -native_yaw.to_radians();
            self.camera.pitch = -self.native_pitch_degrees.to_radians();
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if self.focused
            && let Some(w) = &self.window
        {
            w.request_redraw()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LightDimension, Vertex, build_atlas_mips, fluid_frame_upload_bytes, interaction_shape,
        local_world_manifest_path, native_packed_vertex_color, preferred_surface_format,
        saved_vertex_brightness, sort_fluid_quad_indices, srgb_to_linear,
    };
    use beta_core::block::RayShape;
    use beta_view::{BlockMapping, BlockStateKey};

    #[test]
    fn dirty_chunk_union_keeps_edit_corner_and_relight_neighbors() {
        assert_eq!(
            super::dirty_chunk_union(&[(1, 1), (0, 0), (1, 1)], &[(2, 1), (0, 0), (-1, 0)]),
            vec![(-1, 0), (0, 0), (1, 1), (2, 1)]
        );
    }

    #[test]
    fn transparent_water_quad_indices_sort_groups_without_reordering_triangles() {
        let quad = |x: f32| {
            let points = [[x, 0.0, 0.0], [x, 1.0, 0.0], [x, 1.0, 1.0], [x, 0.0, 1.0]];
            [0, 1, 2, 0, 2, 3].map(|index| beta_view::fluid::Vertex {
                position: points[index],
                uv: [index as f32, 0.0],
                color: index as u32,
            })
        };
        let mut submitted = Vec::new();
        submitted.extend(quad(1.0));
        submitted.extend(quad(2.0));
        submitted.extend(quad(3.0));
        let indices = sort_fluid_quad_indices(&submitted, [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(
            indices,
            (12..18).chain(6..12).chain(0..6).collect::<Vec<_>>()
        );
        assert_eq!(indices.len(), submitted.len());
        assert_eq!(
            indices.chunks_exact(6).next().unwrap(),
            &(12..18).collect::<Vec<_>>()
        );
        assert_eq!(submitted[indices[0] as usize].uv[0], 0.0);
        assert_eq!(submitted[indices[1] as usize].uv[0], 1.0);
        assert_eq!(submitted[indices[2] as usize].uv[0], 2.0);
    }

    #[test]
    fn transparent_water_index_order_preserves_equal_depth_submission_order() {
        let make_quad = |x: f32| {
            [0, 0, 0, 0, 0, 0].map(|_| beta_view::fluid::Vertex {
                position: [x, 0.0, 0.0],
                uv: [x, 0.0],
                color: 0,
            })
        };
        let submitted: Vec<_> = [make_quad(-2.0), make_quad(2.0), make_quad(1.0)]
            .into_iter()
            .flatten()
            .collect();
        let indices = sort_fluid_quad_indices(&submitted, [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(indices, (0..18).collect::<Vec<_>>());
    }

    #[test]
    fn transparent_water_overlapping_depths_keep_farther_quad_first() {
        let quad = |depths: [f32; 4]| {
            let points = [
                [0.0, 0.0, depths[0]],
                [0.0, 1.0, depths[1]],
                [1.0, 1.0, depths[2]],
                [1.0, 0.0, depths[3]],
            ];
            [0, 1, 2, 0, 2, 3].map(|corner| beta_view::fluid::Vertex {
                position: points[corner],
                uv: [corner as f32, depths[corner]],
                color: 0,
            })
        };
        // These quads cover the same projected square and their depth ranges
        // overlap ([4, 6] and [4.3, 5.3]). Their weighted six-vertex centers are
        // at z=5 and z=4.8, so the first quad is unambiguously farther away.
        let submitted: Vec<_> = [quad([4.0, 6.0, 6.0, 4.0]), quad([4.3, 5.3, 5.3, 4.3])]
            .into_iter()
            .flatten()
            .collect();
        let indices = sort_fluid_quad_indices(&submitted, [0.0; 3]).unwrap();
        assert_eq!(indices, (0..6).chain(6..12).collect::<Vec<_>>());
        assert_eq!(
            indices
                .iter()
                .map(|&index| submitted[index as usize].position)
                .collect::<Vec<_>>(),
            submitted
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn fluid_frame_upload_accounting_excludes_static_vertices() {
        let bytes = fluid_frame_upload_bytes(6 * 257).unwrap();
        assert_eq!(bytes.vertex, 0);
        assert_eq!(bytes.indices, 6 * 257 * std::mem::size_of::<u32>());
        assert_eq!(bytes.indices, 6 * 257 * 4);
        assert!(fluid_frame_upload_bytes(usize::MAX).is_none());
    }

    #[test]
    fn fluid_index_sort_rejects_incomplete_quad() {
        let vertex = beta_view::fluid::Vertex {
            position: [0.0; 3],
            uv: [0.0; 2],
            color: 0,
        };
        assert_eq!(
            sort_fluid_quad_indices(&[vertex], [0.0; 3]),
            Err("fluid mesh vertex count is not a whole number of quads")
        );
    }

    #[test]
    fn fluid_gpu_layout_checks_byte_sizes_and_draw_count() {
        assert_eq!(super::fluid_gpu_layout(12).unwrap(), (384, 48, 12));
        assert!(super::fluid_gpu_layout(1).is_err());
        let aligned_huge = usize::MAX - usize::MAX % 6;
        assert!(super::fluid_gpu_layout(aligned_huge).is_err());
    }

    #[test]
    fn local_world_manifest_is_shared_by_sibling_worktrees() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
        let canonical_root = root.canonicalize().unwrap();
        assert_eq!(
            local_world_manifest_path(&root).unwrap(),
            canonical_root
                .parent()
                .unwrap()
                .join("artifacts/user-world-postplay.sha256")
        );
    }

    #[test]
    fn targeting_uses_only_verified_full_cubes_slab_liquid_and_unknown_is_explicit() {
        let mapping = BlockMapping {
            key: BlockStateKey {
                id: 1,
                metadata: 0,
                above_id: None,
            },
            render_type: 0,
            opaque: true,
            full_cube: true,
            alpha_opaque_faces: [true; 6],
            tiles: [1; 6],
            supported: true,
        };
        let mappings = [mapping];
        assert_eq!(interaction_shape(&mappings, 1, 0), RayShape::FullCube);
        assert_eq!(interaction_shape(&mappings, 44, 0), RayShape::LowerSlab);
        assert_eq!(interaction_shape(&mappings, 8, 0), RayShape::Liquid);
        assert_eq!(interaction_shape(&mappings, 44, 1), RayShape::Unsupported);
        assert_eq!(interaction_shape(&mappings, 3, 0), RayShape::Unsupported);
    }

    #[test]
    fn surface_format_prefers_native_numeric_unorm_with_srgb_fallback() {
        use wgpu::TextureFormat as F;

        assert_eq!(
            preferred_surface_format(&[F::Rgba8UnormSrgb, F::Rgba8Unorm, F::Bgra8Unorm]),
            Some(F::Bgra8Unorm)
        );
        assert_eq!(
            preferred_surface_format(&[F::Bgra8UnormSrgb, F::Rgba8UnormSrgb]),
            Some(F::Bgra8UnormSrgb)
        );
        assert_eq!(preferred_surface_format(&[F::Rgba16Float]), None);
    }

    #[test]
    fn inverse_srgb_attachment_conversion_matches_known_channels() {
        let byte_cases = [
            (0_u8, 0.0),
            (10, 0.003_035_27),
            (11, 0.003_346_536),
            (128, 0.215_860_5),
            (255, 1.0),
        ];
        for (encoded_byte, expected_linear) in byte_cases {
            let encoded = f32::from(encoded_byte) / 255.0;
            assert!((srgb_to_linear(encoded) - expected_linear).abs() < 1.0e-7);
        }
        assert!((srgb_to_linear(0.04045) - (0.04045 / 12.92)).abs() < 1.0e-7);
        assert!(include_str!("shader.wgsl").contains("override OUTPUT_SRGB: u32 = 0u;"));
        assert!(include_str!("shader.wgsl").contains("if OUTPUT_SRGB != 0u {"));
        assert!(include_str!("shader.wgsl").contains("srgb_to_linear(native_rgb)"));
    }

    #[test]
    fn saved_time_brightness_applies_dimension_subtraction_and_emission_minimum() {
        let vertex = Vertex {
            position: [0.0; 3],
            uv: [0.0; 2],
            block_light: 0,
            sky_light: 15,
            minimum_light: 0,
            shade: 1.0,
            tint: [1.0; 3],
            tint_after_light: false,
        };
        let day_subtraction =
            beta_core::light::sky_subtraction(6_000, 1.0, LightDimension::Overworld).unwrap();
        let night_subtraction =
            beta_core::light::sky_subtraction(18_000, 1.0, LightDimension::Overworld).unwrap();
        assert_eq!(day_subtraction, 0);
        assert_eq!(night_subtraction, 11);
        assert_eq!(
            saved_vertex_brightness(&vertex, day_subtraction, LightDimension::Overworld),
            Ok(1.0)
        );
        assert_eq!(
            saved_vertex_brightness(&vertex, night_subtraction, LightDimension::Overworld),
            Ok(beta_core::light::brightness(4, LightDimension::Overworld).unwrap())
        );
        let dark_sample = Vertex {
            sky_light: 0,
            ..vertex
        };
        assert_eq!(
            saved_vertex_brightness(&dark_sample, 0, LightDimension::Overworld),
            Ok(0.05)
        );
        assert_eq!(
            saved_vertex_brightness(&dark_sample, 0, LightDimension::Nether),
            Ok(0.1)
        );
        let emitted = Vertex {
            minimum_light: 15,
            ..vertex
        };
        assert_eq!(
            saved_vertex_brightness(&emitted, night_subtraction, LightDimension::Overworld),
            Ok(1.0)
        );
    }

    #[test]
    fn packed_vertex_colors_match_direct_native_capture() {
        const FIXTURE: &str = include_str!("../../../tests/fixtures/light/vertex-color.tsv");
        let mut cases = 0;
        let mut vertices = 0;
        for line in FIXTURE.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<_> = line.split('\t').collect();
            if fields[0] == "case" {
                assert_eq!(fields[4], "1", "native renderer should render this case");
                cases += 1;
            } else if fields[0] == "vertex" {
                let block_id = fields[1].parse::<u8>().unwrap();
                let brightness = f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap());
                let face_shade = match fields[4] {
                    "down" => 0.5,
                    "up" => 1.0,
                    "north" | "south" => 0.8,
                    "west" | "east" => 0.6,
                    other => panic!("unknown native face {other}"),
                };
                let tint_after_light = fields[5] == "overlay";
                let mut tint = [0.0; 3];
                for (channel, bits) in tint.iter_mut().zip(&fields[7..10]) {
                    *channel = f32::from_bits(u32::from_str_radix(bits, 16).unwrap());
                }
                // Native grass base sides use white; the top and overlays use palette tint.
                if block_id == 2 && fields[5] == "base" && fields[4] != "up" {
                    tint = [1.0; 3];
                }
                let vertex = Vertex {
                    position: [0.0; 3],
                    uv: [0.0; 2],
                    block_light: 0,
                    sky_light: 0,
                    minimum_light: 0,
                    shade: face_shade,
                    tint,
                    tint_after_light,
                };
                let actual = native_packed_vertex_color(&vertex, brightness);
                let packed = u32::from_str_radix(fields[10], 16).unwrap();
                let expected = [packed & 0xff, (packed >> 8) & 0xff, (packed >> 16) & 0xff];
                for channel in 0..3 {
                    let actual = (actual[channel] * 255.0).round() as u32;
                    assert_eq!(actual, expected[channel], "native row: {line}");
                }
                vertices += 1;
            } else {
                panic!("unknown vertex-color fixture row: {line}");
            }
        }
        assert_eq!(cases, 12);
        assert_eq!(vertices, 504);
    }

    #[test]
    fn atlas_mips_preserve_tile_colors_without_neighbor_bleed() {
        let mut atlas = image::RgbaImage::new(256, 256);
        for tile_y in 0..16_u32 {
            for tile_x in 0..16_u32 {
                let color = image::Rgba([(tile_x + tile_y * 16) as u8, 80, 160, 255]);
                for y in tile_y * 16..tile_y * 16 + 16 {
                    for x in tile_x * 16..tile_x * 16 + 16 {
                        atlas.put_pixel(x, y, color);
                    }
                }
            }
        }

        let levels = build_atlas_mips(&atlas);
        assert_eq!(levels.len(), 5);
        for (level, mip) in levels.iter().enumerate() {
            let tile_size = 16_u32 >> level;
            assert_eq!(mip.dimensions(), (256 >> level, 256 >> level));
            for tile_y in 0..16_u32 {
                for tile_x in 0..16_u32 {
                    let expected = image::Rgba([(tile_x + tile_y * 16) as u8, 80, 160, 255]);
                    for y in tile_y * tile_size..tile_y * tile_size + tile_size {
                        for x in tile_x * tile_size..tile_x * tile_size + tile_size {
                            assert_eq!(*mip.get_pixel(x, y), expected);
                        }
                    }
                }
            }
        }
    }
}
