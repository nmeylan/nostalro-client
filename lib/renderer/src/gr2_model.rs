use std::collections::HashMap;
use std::rc::Rc;

use ragnarok_formats::gr2::Gr2File;

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use crate::texture::{TextureCache, create_texture_bind_group_from_rgba};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Gr2ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub bone_indices: [u8; 4],
    pub bone_weights: [u8; 4],
}

impl Gr2ModelVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Gr2ModelVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Uint8x4,
            4 => Unorm8x4,
        ],
    };
}

#[derive(Clone, Copy)]
struct DrawBatch {
    texture_index: usize,
    start_index: u32,
    index_count: u32,
}

/// CPU-side geometry of one GR2 model: a single skinned vertex/index buffer
/// with draw batches grouped by texture. Vertex bone indices are remapped from
/// per-mesh bone bindings to skeleton bone indices, so one skinning palette
/// drives every mesh.
pub struct Gr2Geometry {
    pub vertices: Vec<Gr2ModelVertex>,
    pub indices: Vec<u32>,
    /// `(texture_index, start_index, index_count)` into `indices`.
    pub batches: Vec<(usize, u32, u32)>,
    pub bone_count: usize,
    pub center: [f32; 3],
    pub size: [f32; 3],
}

/// A decoded GR2 texture, ready for upload.
pub struct Gr2TextureData {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Decode every texture embedded in `file`. One that fails to decode becomes a
/// single white pixel, so the batches drawn with it stay visible.
pub fn decode_textures(file: &Gr2File) -> Vec<Gr2TextureData> {
    file.textures
        .iter()
        .map(|tex| match tex.to_rgba() {
            Ok(rgba) => Gr2TextureData {
                rgba,
                width: tex.width as u32,
                height: tex.height as u32,
            },
            Err(e) => {
                tracing::warn!("gr2 texture {} decode failed: {e}", tex.from_file_name);
                Gr2TextureData {
                    rgba: vec![255u8; 4],
                    width: 1,
                    height: 1,
                }
            }
        })
        .collect()
}

/// Index of the model's emblem texture slot, swapped at runtime for the owning
/// guild's emblem. Only the guild flag has one.
pub fn emblem_texture_index(file: &Gr2File) -> Option<usize> {
    file.textures
        .iter()
        .position(|t| t.from_file_name.to_ascii_lowercase().contains("emblem"))
}

/// Upload an emblem for `set_emblem_texture`, matching how the model's own
/// embedded textures are uploaded.
pub fn create_emblem_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[u8],
    width: u32,
    height: u32,
    texture_cache: &TextureCache,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_from_rgba(
        device,
        queue,
        rgba,
        width,
        height,
        &texture_cache.bind_group_layout,
        label,
        wgpu::FilterMode::Linear,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::AddressMode::ClampToEdge,
    )
}

pub fn build_gr2_geometry(file: &Gr2File, model_index: usize) -> Option<Gr2Geometry> {
    let model = file.models.get(model_index)?;
    let skeleton = file.skeletons.get(model.skeleton_index?)?;
    let bone_by_name: HashMap<&str, u8> = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i as u8))
        .collect();

    let mut vertices: Vec<Gr2ModelVertex> = Vec::new();
    let mut per_texture: HashMap<usize, Vec<u32>> = HashMap::new();
    let emblem_index = emblem_texture_index(file);

    for &mi in &model.mesh_indices {
        let mesh = file.meshes.get(mi)?;
        let Some(vd) = mesh
            .vertex_data_index
            .and_then(|i| file.vertex_datas.get(i))
        else {
            continue;
        };
        let Some(topo) = mesh.topology_index.and_then(|i| file.tri_topologies.get(i)) else {
            continue;
        };

        let binding_to_bone: Vec<u8> = mesh
            .bone_bindings
            .iter()
            .map(|name| bone_by_name.get(name.as_str()).copied().unwrap_or(0))
            .collect();
        let remap = |slot: u8| binding_to_bone.get(slot as usize).copied().unwrap_or(0);

        let group_texture = |group: &ragnarok_formats::gr2::model::Gr2TriGroup| {
            mesh.material_indices
                .get(group.material_index.max(0) as usize)
                .and_then(|&m| file.materials.get(m))
                .and_then(|m| m.texture_index)
                .unwrap_or(0)
        };

        // The guild flag's emblem overlay quad is authored ~1.5 units right of
        // the banner's centerline; recenter its bind-pose X span on the model's
        // symmetry axis. Emblem-textured meshes are the only ones nudged.
        let is_emblem_mesh = !topo.groups.is_empty()
            && topo
                .groups
                .iter()
                .all(|g| Some(group_texture(g)) == emblem_index);
        let x_shift = if is_emblem_mesh {
            let (min, max) = vd
                .vertices
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), v| {
                    (lo.min(v.position[0]), hi.max(v.position[0]))
                });
            (min + max) * 0.5
        } else {
            0.0
        };

        let base = vertices.len() as u32;
        for v in &vd.vertices {
            // Rigid meshes carry no weight stream; bind them to their first bone.
            let (slots, weights) = if v.bone_weights == [0; 4] {
                ([0u8; 4], [255u8, 0, 0, 0])
            } else {
                (v.bone_indices, v.bone_weights)
            };
            vertices.push(Gr2ModelVertex {
                position: [v.position[0] - x_shift, v.position[1], v.position[2]],
                normal: v.normal,
                tex_coord: v.uv,
                bone_indices: slots.map(remap),
                bone_weights: weights,
            });
        }

        for group in &topo.groups {
            let texture_index = group_texture(group);
            let first = group.tri_first.max(0) as usize * 3;
            let count = group.tri_count.max(0) as usize * 3;
            let Some(tris) = topo.indices.get(first..first + count) else {
                continue;
            };
            per_texture
                .entry(texture_index)
                .or_default()
                .extend(tris.iter().map(|&i| base + i));
        }
    }

    if vertices.is_empty() {
        return None;
    }

    let mut indices = Vec::new();
    let mut batches = Vec::new();
    let mut keys: Vec<usize> = per_texture.keys().copied().collect();
    keys.sort_unstable();
    for key in keys {
        let idxs = &per_texture[&key];
        batches.push((key, indices.len() as u32, idxs.len() as u32));
        indices.extend_from_slice(idxs);
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &vertices {
        for k in 0..3 {
            min[k] = min[k].min(v.position[k]);
            max[k] = max[k].max(v.position[k]);
        }
    }
    let center = std::array::from_fn(|k| (min[k] + max[k]) * 0.5);
    let size = std::array::from_fn(|k| max[k] - min[k]);

    Some(Gr2Geometry {
        vertices,
        indices,
        batches,
        bone_count: skeleton.bones.len(),
        center,
        size,
    })
}

/// Shader, layouts and pipeline shared by every GR2 model draw.
pub struct Gr2ModelPipeline {
    pipeline: wgpu::RenderPipeline,
    skin_bind_group_layout: wgpu::BindGroupLayout,
}

impl Gr2ModelPipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        global_uniforms: &GlobalUniforms,
        texture_cache: &TextureCache,
    ) -> Self {
        let skin_bind_group_layout = create_skin_layout(device);
        let pipeline = create_pipeline(
            device,
            surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
            &skin_bind_group_layout,
        );
        Gr2ModelPipeline {
            pipeline,
            skin_bind_group_layout,
        }
    }
}

/// The per-model half of a GR2 draw: skinned geometry batched by embedded
/// texture. Shared by every instance of the same model file.
pub struct Gr2ModelAsset {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    batches: Vec<DrawBatch>,
    textures: Vec<wgpu::BindGroup>,
    bone_count: usize,
    emblem_texture_index: Option<usize>,
    /// Bind-pose bounding-box center/size in model space (before the instance
    /// transform), for camera framing.
    pub center: [f32; 3],
    pub size: [f32; 3],
}

impl Gr2ModelAsset {
    pub fn from_gr2(
        file: &Gr2File,
        model_index: usize,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_cache: &TextureCache,
    ) -> Option<Self> {
        let geometry = build_gr2_geometry(file, model_index)?;
        let textures = decode_textures(file);
        Self::from_parts(
            device,
            queue,
            texture_cache,
            geometry,
            textures,
            emblem_texture_index(file),
        )
    }

    /// Build from geometry and decoded textures produced elsewhere — the
    /// background loader decodes off the frame thread and uploads here.
    pub fn from_parts(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_cache: &TextureCache,
        geometry: Gr2Geometry,
        textures: Vec<Gr2TextureData>,
        emblem_texture_index: Option<usize>,
    ) -> Option<Self> {
        if textures.is_empty() {
            tracing::warn!("gr2 model has no textures");
            return None;
        }
        let textures: Vec<wgpu::BindGroup> = textures
            .iter()
            .enumerate()
            .map(|(i, tex)| {
                create_texture_bind_group_from_rgba(
                    device,
                    queue,
                    &tex.rgba,
                    tex.width,
                    tex.height,
                    &texture_cache.bind_group_layout,
                    &format!("gr2_texture_{i}"),
                    wgpu::FilterMode::Linear,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::AddressMode::Repeat,
                )
            })
            .collect();

        let vertex_buffer = create_buffer(
            device,
            "gr2_vertices",
            &geometry.vertices,
            wgpu::BufferUsages::VERTEX,
        );
        let index_buffer = create_buffer(
            device,
            "gr2_indices",
            &geometry.indices,
            wgpu::BufferUsages::INDEX,
        );

        Some(Gr2ModelAsset {
            vertex_buffer,
            index_buffer,
            batches: geometry
                .batches
                .iter()
                .map(|&(texture_index, start_index, index_count)| DrawBatch {
                    texture_index,
                    start_index,
                    index_count,
                })
                .collect(),
            textures,
            bone_count: geometry.bone_count.max(1),
            emblem_texture_index,
            center: geometry.center,
            size: geometry.size,
        })
    }

    pub fn bone_count(&self) -> usize {
        self.bone_count
    }
}

/// One drawn GR2 entity: a shared model asset placed by its own world transform
/// and posed by its own bone-matrix palette, both uploaded each frame.
pub struct Gr2ModelDraw {
    asset: Rc<Gr2ModelAsset>,
    instance_buffer: wgpu::Buffer,
    bone_buffer: wgpu::Buffer,
    skin_bind_group: wgpu::BindGroup,
    emblem_texture: Option<wgpu::BindGroup>,
}

impl Gr2ModelDraw {
    pub fn new(
        device: &wgpu::Device,
        pipeline: &Gr2ModelPipeline,
        asset: Rc<Gr2ModelAsset>,
    ) -> Self {
        let identity = glam::Mat4::IDENTITY.to_cols_array();
        let instance_buffer = create_buffer(
            device,
            "gr2_instance",
            &[identity],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let bind_palette = vec![identity; asset.bone_count];
        let bone_buffer = create_buffer(
            device,
            "gr2_bones",
            &bind_palette,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let skin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gr2_skin"),
            layout: &pipeline.skin_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bone_buffer.as_entire_binding(),
                },
            ],
        });

        Gr2ModelDraw {
            asset,
            instance_buffer,
            bone_buffer,
            skin_bind_group,
            emblem_texture: None,
        }
    }

    pub fn asset(&self) -> &Gr2ModelAsset {
        &self.asset
    }

    pub fn bone_count(&self) -> usize {
        self.asset.bone_count
    }

    /// Swap the guild flag's embedded default emblem for a real one. Returns
    /// false when the model has no emblem slot.
    pub fn set_emblem_texture(&mut self, bind_group: wgpu::BindGroup) -> bool {
        if self.asset.emblem_texture_index.is_none() {
            return false;
        }
        self.emblem_texture = Some(bind_group);
        true
    }

    pub fn set_transform(&self, queue: &wgpu::Queue, matrix: glam::Mat4) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[matrix.to_cols_array()]),
        );
    }

    /// Upload the current skinning palette (`world[i] * inverse_world[i]` per
    /// bone). Extra matrices beyond the model's bone count are ignored.
    pub fn set_palette(&self, queue: &wgpu::Queue, palette: &[glam::Mat4]) {
        let data: Vec<[f32; 16]> = palette
            .iter()
            .take(self.asset.bone_count)
            .map(|m| m.to_cols_array())
            .collect();
        if !data.is_empty() {
            queue.write_buffer(&self.bone_buffer, 0, bytemuck::cast_slice(&data));
        }
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a Gr2ModelPipeline,
        global_uniforms: &'a GlobalUniforms,
    ) {
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_bind_group(2, &self.skin_bind_group, &[]);
        pass.set_vertex_buffer(0, self.asset.vertex_buffer.slice(..));
        pass.set_index_buffer(self.asset.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for batch in &self.asset.batches {
            let texture = match &self.emblem_texture {
                Some(emblem) if Some(batch.texture_index) == self.asset.emblem_texture_index => {
                    emblem
                }
                _ => {
                    let Some(texture) = self.asset.textures.get(batch.texture_index) else {
                        continue;
                    };
                    texture
                }
            };
            pass.set_bind_group(1, texture, &[]);
            pass.draw_indexed(
                batch.start_index..batch.start_index + batch.index_count,
                0,
                0..1,
            );
        }
    }
}

fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(data),
        usage,
    })
}

fn create_skin_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gr2_skin"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    global_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    skin_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gr2_model"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gr2_model.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gr2_model"),
        bind_group_layouts: &[
            global_bind_group_layout,
            texture_bind_group_layout,
            skin_bind_group_layout,
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gr2_model"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Gr2ModelVertex::LAYOUT],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::gr2::Gr2Container;
    use ragnarok_formats::grf::GrfArchive;
    use std::path::Path;

    fn load_gr2(name: &str) -> Option<Gr2File> {
        // The last candidate reaches the main checkout from a `.worktree/<name>` worktree.
        let grf = [
            "data/data.grf",
            "../../data/data.grf",
            "../../../../data/data.grf",
        ]
        .iter()
        .find(|p| Path::new(p).exists())
        .map(|p| GrfArchive::open(Path::new(p)).expect("open grf"))?;
        let bytes = grf.read_file(name).expect("read gr2");
        let container = Gr2Container::parse(&bytes).expect("parse container");
        Some(Gr2File::parse(&container).expect("extract"))
    }

    #[test]
    fn emperium_geometry_is_single_batch_and_fully_weighted() {
        let Some(file) = load_gr2("data/model/3dmob/empelium90_0.gr2") else {
            eprintln!("skip: no grf");
            return;
        };
        let g = build_gr2_geometry(&file, 0).expect("geometry");
        assert_eq!(g.vertices.len(), 159);
        assert_eq!(g.indices.len(), 414);
        assert_eq!(g.batches.len(), 1);
        assert_eq!(g.bone_count, 18);
        assert_eq!(g.batches[0], (0, 0, 414));
        assert_eq!(emblem_texture_index(&file), None);
        for v in &g.vertices {
            let sum: u32 = v.bone_weights.iter().map(|&w| w as u32).sum();
            assert_eq!(sum, 255, "weights must sum to 1");
            assert!(v.bone_indices.iter().all(|&b| (b as usize) < g.bone_count));
        }
        assert!(g.indices.iter().all(|&i| (i as usize) < g.vertices.len()));
        assert!(g.size.iter().all(|&s| s > 0.0));
        eprintln!(
            "emperium bind-pose size = {:?} center = {:?}",
            g.size, g.center
        );
    }

    #[test]
    fn guild_flag_emblem_quad_is_recentered() {
        let Some(file) = load_gr2("data/model/3dmob/guildflag90_1.gr2") else {
            eprintln!("skip: no grf");
            return;
        };
        let g = build_gr2_geometry(&file, 0).expect("geometry");
        let emblem_tex = emblem_texture_index(&file).expect("emblem texture");
        let &(_, start, count) = g
            .batches
            .iter()
            .find(|&&(t, _, _)| t == emblem_tex)
            .expect("emblem batch");
        let (min, max) = g.indices[start as usize..(start + count) as usize]
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &i| {
                let x = g.vertices[i as usize].position[0];
                (lo.min(x), hi.max(x))
            });
        assert!(
            (min + max).abs() < 1e-3,
            "emblem quad not centered: min={min} max={max}"
        );
    }

    #[test]
    fn guardian_geometry_merges_meshes_and_batches_by_texture() {
        let Some(file) = load_gr2("data/model/3dmob/kguardian90_7.gr2") else {
            eprintln!("skip: no grf");
            return;
        };
        let g = build_gr2_geometry(&file, 0).expect("geometry");
        let total_vertices: usize = file.vertex_datas.iter().map(|v| v.vertices.len()).sum();
        let total_indices: usize = file.tri_topologies.iter().map(|t| t.indices.len()).sum();
        assert_eq!(g.vertices.len(), total_vertices);
        assert_eq!(g.indices.len(), total_indices);
        assert_eq!(g.bone_count, 35);
        // Each of the 5 meshes binds a "Default" parent material whose texture
        // resolves through its Maps chain  one batch per distinct texture.
        assert_eq!(g.batches.len(), 5);
        let batch_total: u32 = g.batches.iter().map(|&(_, _, n)| n).sum();
        assert_eq!(batch_total as usize, total_indices);
        for v in &g.vertices {
            let sum: u32 = v.bone_weights.iter().map(|&w| w as u32).sum();
            assert_eq!(sum, 255, "weights must sum to 1");
            assert!(v.bone_indices.iter().all(|&b| (b as usize) < g.bone_count));
        }
        assert!(g.indices.iter().all(|&i| (i as usize) < g.vertices.len()));
    }
}
