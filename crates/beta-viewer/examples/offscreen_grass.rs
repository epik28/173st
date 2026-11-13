//! Offscreen shader smoke: opaque/transparent grass overlays at two light levels.
//! The shadowed case has a manually set light value after a synthetic stone edit.
//! Uses a synthetic atlas; does not measure native screenshots, world relighting,
//! palette selection, alpha-to-coverage, or window/swapchain behavior.
use beta_core::{
    chunk::{BlockData, ChunkData},
    light::{LightDimension, brightness},
};
use beta_view::{BlockMapping, BlockStateKey, MeshLimits, Scene, SceneBounds, SceneChunk, Vertex};
use std::sync::mpsc;
use wgpu::util::DeviceExt;

fn packed(v: &Vertex) -> [u8; 3] {
    let light = brightness(v.sky_light, LightDimension::Overworld).unwrap();
    std::array::from_fn(|i| {
        let value = if v.tint_after_light {
            (v.shade * light) * v.tint[i]
        } else {
            (v.shade * v.tint[i]) * light
        };
        (value * 255.0).trunc().clamp(0.0, 255.0) as u8
    })
}
fn mesh_colors(level: u8) -> ([u8; 3], [u8; 3]) {
    let mut chunk = ChunkData::new();
    chunk
        .set(
            8,
            64,
            8,
            BlockData {
                id: 2,
                ..Default::default()
            },
        )
        .unwrap();
    if level == 3 {
        chunk
            .set(
                8,
                66,
                8,
                BlockData {
                    id: 1,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    chunk
        .set(
            8,
            64,
            7,
            BlockData {
                sky_light: level,
                ..Default::default()
            },
        )
        .unwrap();
    let mapping = BlockMapping {
        key: BlockStateKey {
            id: 2,
            metadata: 0,
            above_id: None,
        },
        render_type: 0,
        opaque: true,
        full_cube: true,
        alpha_opaque_faces: [true; 6],
        tiles: [2, 0, 3, 3, 3, 3],
        supported: true,
    };
    let scene = Scene::new(
        SceneBounds::new(0, 0, 1, 1).unwrap(),
        vec![SceneChunk {
            x: 0,
            z: 0,
            data: chunk,
        }],
        vec![
            mapping.clone(),
            BlockMapping {
                key: BlockStateKey {
                    id: 1,
                    metadata: 0,
                    above_id: None,
                },
                tiles: [1; 6],
                ..mapping
            },
        ],
    )
    .unwrap();
    let mesh = scene
        .mesh_with_tint(MeshLimits::default(), |_, _, _, _| 0x3aa457)
        .unwrap();
    // Quad order: down, up, north base, north overlay.
    (packed(&mesh.vertices[8]), packed(&mesh.vertices[12]))
}
fn append_quad(vertices: &mut Vec<[f32; 8]>, slot: usize, uv: f32, color: [u8; 3]) {
    let x0 = -1.0 + slot as f32 * 0.5;
    let x1 = x0 + 0.5;
    let c = color.map(|v| f32::from(v) / 255.0);
    for [x, y] in [
        [x0, -1.0],
        [x1, -1.0],
        [x1, 1.0],
        [x0, -1.0],
        [x1, 1.0],
        [x0, 1.0],
    ] {
        vertices.push([x, y, 0.5, uv, 0.5, c[0], c[1], c[2]]);
    }
}
fn expected(texture: [u8; 4], color: [u8; 3]) -> [u8; 4] {
    [
        ((f32::from(texture[0]) * f32::from(color[0])) / 255.0).round() as u8,
        ((f32::from(texture[1]) * f32::from(color[1])) / 255.0).round() as u8,
        ((f32::from(texture[2]) * f32::from(color[2])) / 255.0).round() as u8,
        255,
    ]
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no headless GPU adapter: {e}"))?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("grass offscreen probe"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await?;
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("synthetic atlas"),
        size: wgpu::Extent3d {
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let atlas: [[u8; 4]; 4] = [
        [160, 120, 80, 255],
        [40, 200, 80, 255],
        [40, 200, 80, 0],
        [0, 0, 0, 255],
    ];
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&atlas),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let view = tex.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
        label: None,
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
                resource: camera.as_entire_binding(),
            },
        ],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("../src/shader.wgsl").into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let mut vertices = Vec::new();
    let mut expected_pixels = Vec::new();
    for slot in 0..4 {
        let (base, overlay) = mesh_colors(if slot % 2 == 0 { 15 } else { 3 });
        append_quad(&mut vertices, slot, 0.125, base);
        let alpha = if slot < 2 { 1 } else { 2 };
        append_quad(
            &mut vertices,
            slot,
            if alpha == 1 { 0.375 } else { 0.625 },
            overlay,
        );
        expected_pixels.push(if alpha == 1 {
            expected(atlas[1], overlay)
        } else {
            expected(atlas[0], base)
        });
    }
    let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let constants = [("OUTPUT_SRGB", if format.is_srgb() { 1.0 } else { 0.0 })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
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
                    constants: &constants,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        let size = wgpu::Extent3d {
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.create_view(&Default::default()),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.create_view(&Default::default()),
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
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_vertex_buffer(0, vertex.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            size,
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = buffer.slice(..).get_mapped_range()?;
        for (slot, expected) in expected_pixels.iter().enumerate() {
            let actual = &pixels[slot * 4..slot * 4 + 4];
            for channel in 0..4 {
                if (i16::from(actual[channel]) - i16::from(expected[channel])).abs() > 1 {
                    return Err(format!(
                        "{format:?} pixel {slot}: actual {actual:?}, expected {expected:?}"
                    )
                    .into());
                }
            }
        }
        drop(pixels);
        buffer.unmap();
        println!("PASS {format:?}: 4 pixels, light levels 15/3, opaque and discarded overlays");
    }
    println!(
        "adapter: {} {:?} {:?}; synthetic atlas, shader only",
        info.name, info.backend, info.device_type
    );
    Ok(())
}
