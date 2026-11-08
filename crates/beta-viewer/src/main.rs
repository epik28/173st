#![forbid(unsafe_code)]

//! Read-only winit/wgpu rendering shell. The scene and CPU mesh are prepared once,
//! then held in static GPU buffers while the camera flies independently.

use beta_view::{Camera, Mesh};
use std::{fs, sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use beta_viewer::loader;

const TERRAIN_HASH: &str = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf";
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
    let input = copy_local_world()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let scene_path = root.join("tests/fixtures/view/scene.tsv");
    let (scene, spawn) = loader::load_scene_for_viewer(&input, &terrain_path, &scene_path)?;
    let loaded_chunks = scene.loaded_chunks();
    let mesh = scene.mesh(beta_view::MeshLimits::default())?;
    let event = EventLoop::new()?;
    let mut app = App {
        mesh,
        loaded_chunks,
        atlas: image,
        window: None,
        gpu: None,
        camera: Camera::beta_start(spawn),
        keys: [false; 7],
        last: Instant::now(),
        captured: false,
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

fn copy_local_world() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    const SOURCES: [(&str, usize, &str); 5] = [
        (
            "level.dat",
            411,
            "2711d16242afd13909575955d4a34502da01f6f666ee79341fe1c1ba96c8a960",
        ),
        (
            "r.-1.-1.mcr",
            761856,
            "b41220e2e389bf303984fa7211192debc7e1db3e49b89c168101a3ed748c435f",
        ),
        (
            "r.-1.0.mcr",
            843776,
            "e189d75077763fd00813f5929a14a9de85704b55ef8042afb8fd2fb2f5801004",
        ),
        (
            "r.0.-1.mcr",
            1019904,
            "59ce84edc6afcec1f962e0d88f519e359137c7245b688a58ab340414b12eb0ba",
        ),
        (
            "r.0.0.mcr",
            782336,
            "67f39f4c77538f178fa7acb155030908f84c079b70f27d44d85f4a20fe965b7a",
        ),
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
    for (name, expected_size, expected_hash) in SOURCES {
        let parent = if name == "level.dat" {
            source_world.clone()
        } else {
            source_world.join("region")
        };
        let source = parent.join(name);
        let metadata = fs::symlink_metadata(&source)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() as usize != expected_size
        {
            return Err(format!("unsafe or unexpected local world source {name}").into());
        }
        let data = fs::read(&source)?;
        let hash = format!("{:x}", sha2::Sha256::digest(&data));
        if hash != expected_hash {
            return Err(format!("pinned world source hash mismatch: {name}").into());
        }
        let destination = if name == "level.dat" {
            input.join(name)
        } else {
            input.join("region").join(name)
        };
        fs::write(destination, data)?;
    }
    // Loader protocol requires all five top-level basenames. Keep region files at
    // the top level as the checked native input-copy format does.
    for (name, _, _) in SOURCES.iter().filter(|(name, _, _)| *name != "level.dat") {
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
    Ok(input)
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

struct App {
    mesh: Mesh,
    loaded_chunks: usize,
    atlas: image::RgbaImage,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    camera: Camera,
    keys: [bool; 7],
    last: Instant,
    captured: bool,
}
struct Gpu {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
    depth: wgpu::TextureView,
    bind: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
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
    fn init(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let w = Arc::new(
            el.create_window(
                Window::default_attributes()
                    .with_title("Beta 1.7.3 — fixed scene")
                    .with_inner_size(PhysicalSize::new(1280, 720)),
            )?,
        );
        w.set_title(&format!(
            "Beta 1.7.3 — loaded chunks: {}, faces: {}, unsupported blocks: {}",
            self.loaded_chunks, self.mesh.visible_faces, self.mesh.unsupported_blocks
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
            let c = surface
                .get_default_config(&a, 1280, 720)
                .ok_or("surface has no format")?;
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 28,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32,3=>Float32],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
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
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let packed: Vec<[f32; 7]> = if self.mesh.vertices.is_empty() {
            vec![[0.0; 7]]
        } else {
            self.mesh
                .vertices
                .iter()
                .map(|v| {
                    [
                        v.position[0],
                        v.position[1],
                        v.position[2],
                        v.uv[0],
                        v.uv[1],
                        v.shade,
                        (f32::from(v.sky_light.max(v.block_light)) / 15.0).max(0.08),
                    ]
                })
                .collect()
        };
        let vertex_bytes = packed
            .len()
            .checked_mul(std::mem::size_of::<[f32; 7]>())
            .ok_or("vertex buffer size overflow")?;
        let index_bytes = self
            .mesh
            .indices
            .len()
            .checked_mul(std::mem::size_of::<u32>())
            .ok_or("index buffer size overflow")?;
        if vertex_bytes > 128 * 1024 * 1024 || index_bytes > 64 * 1024 * 1024 {
            return Err(format!("GPU mesh resource limit exceeded (vertices {vertex_bytes} bytes, indices {index_bytes} bytes)").into());
        }
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static scene mesh"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = if self.mesh.indices.is_empty() {
            vec![0_u32]
        } else {
            self.mesh.indices.clone()
        };
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static scene indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let dep = depth(&device, w.inner_size());
        self.gpu = Some(Gpu {
            instance,
            surface,
            device,
            queue,
            config,
            pipeline,
            vertex,
            index,
            index_count: self.mesh.indices.len() as u32,
            depth: dep,
            bind,
            camera_buffer,
        });
        self.window = Some(w);
        Ok(())
    }
    fn frame(&mut self) {
        let window = self.window.clone();
        let Some(g) = self.gpu.as_mut() else { return };
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;
        let f = self.keys[0] as i32 - self.keys[1] as i32;
        let s = self.keys[3] as i32 - self.keys[2] as i32;
        let v = self.keys[4] as i32 - self.keys[5] as i32;
        let _ = self.camera.move_local(
            f as f32,
            s as f32,
            v as f32,
            if self.keys[6] { 28.0 } else { 8.0 },
            dt,
        );
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
                    p.set_vertex_buffer(0, g.vertex.slice(..));
                    p.set_index_buffer(g.index.slice(..), wgpu::IndexFormat::Uint32);
                    p.draw_indexed(0..g.index_count, 0, 0..1)
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape {
                        self.captured = false;
                        if let Some(w) = &self.window {
                            w.set_cursor_visible(true);
                            let _ = w.set_cursor_grab(CursorGrabMode::None);
                        }
                        return;
                    }
                    let idx = match code {
                        KeyCode::KeyW => Some(0),
                        KeyCode::KeyS => Some(1),
                        KeyCode::KeyA => Some(2),
                        KeyCode::KeyD => Some(3),
                        KeyCode::Space => Some(4),
                        KeyCode::ControlLeft => Some(5),
                        KeyCode::ShiftLeft => Some(6),
                        _ => None,
                    };
                    if let Some(i) = idx {
                        self.keys[i] = event.state == ElementState::Pressed
                    }
                    if event.state == ElementState::Pressed
                        && code == KeyCode::Enter
                        && let Some(w) = &self.window
                    {
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
            let _ = self
                .camera
                .rotate(-delta.0 as f32 * 0.002, -delta.1 as f32 * 0.002);
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_atlas_mips;

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
