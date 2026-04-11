use std::collections::HashMap;

use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsm::{RsmFile, RsmNode};
use ragnarok_formats::rsw::{RswFile, RswObject};

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use crate::texture::TextureCache;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub alpha: f32,
}

impl ModelVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ModelVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Float32,
        ],
    };
}

struct DrawBatch {
    texture_name: String,
    start_index: u32,
    index_count: u32,
}

pub struct ModelRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    batches: Vec<DrawBatch>,
}

struct BoundingBox {
    min: glam::Vec3,
    max: glam::Vec3,
    center: glam::Vec3,
}

impl BoundingBox {
    fn new() -> Self {
        Self {
            min: glam::Vec3::splat(f32::INFINITY),
            max: glam::Vec3::splat(f32::NEG_INFINITY),
            center: glam::Vec3::ZERO,
        }
    }

    fn extend(&mut self, point: glam::Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    fn finalize(&mut self) {
        let range = (self.max - self.min) * 0.5;
        self.center = self.min + range;
    }
}

impl ModelRenderer {
    pub fn from_rsw(
        rsw: &RswFile,
        gnd: &GndFile,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        global_uniforms: &GlobalUniforms,
        texture_cache: &mut TextureCache,
        surface_format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let rsw_models: Vec<_> = rsw.objects.iter().filter_map(|obj| {
            if let RswObject::Model(m) = obj { Some(m) } else { None }
        }).collect();

        if rsw_models.is_empty() {
            return None;
        }

        let start = std::time::Instant::now();

        let (vertices, indices, batches) = build_mesh(
            &rsw_models, gnd, grf, texture_cache, device, queue,
        );

        tracing::info!(
            "Loaded {} model instances ({} verts, {} batches) in {:.1}s",
            rsw_models.len(), vertices.len(), batches.len(), start.elapsed().as_secs_f32(),
        );

        if vertices.is_empty() {
            return None;
        }

        let vertex_buffer = create_buffer(device, "model_vertices", &vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(device, "model_indices", &indices, wgpu::BufferUsages::INDEX);

        let pipeline = create_pipeline(
            device,
            surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );

        Some(Self { pipeline, vertex_buffer, index_buffer, batches })
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        texture_cache: &'a TextureCache,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for batch in &self.batches {
            if let Some(tex_bg) = texture_cache.get(&batch.texture_name) {
                pass.set_bind_group(1, tex_bg, &[]);
                pass.draw_indexed(
                    batch.start_index..batch.start_index + batch.index_count,
                    0,
                    0..1,
                );
            }
        }
    }
}

fn build_mesh(
    rsw_models: &[&ragnarok_formats::rsw::RswModel],
    gnd: &GndFile,
    grf: &GrfArchive,
    texture_cache: &mut TextureCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (Vec<ModelVertex>, Vec<u32>, Vec<DrawBatch>) {
    let zoom = gnd.zoom;
    let scale_factor = zoom / 10.0;
    let center_x = gnd.width as f32 * zoom / 2.0;
    let center_z = gnd.height as f32 * zoom / 2.0;

    // Cache parsed RSM files: filename → RsmFile
    let mut rsm_cache: HashMap<String, Option<RsmFile>> = HashMap::new();

    // Group RSW models by their RSM filename
    struct ModelInstance {
        instance_matrix: glam::Mat4,
        alpha: f32,
    }
    let mut rsm_instances: HashMap<String, Vec<ModelInstance>> = HashMap::new();

    for rsw_model in rsw_models {
        let rsm_path = format!("data\\model\\{}", rsw_model.model_name);

        // Parse RSM if not cached
        if !rsm_cache.contains_key(&rsm_path) {
            let rsm = match grf.read_file(&rsm_path) {
                Ok(data) => match RsmFile::parse(&data) {
                    Ok(rsm) => Some(rsm),
                    Err(e) => {
                        tracing::warn!("Failed to parse RSM {rsm_path}: {e}");
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read RSM {rsm_path}: {e}");
                    None
                }
            };
            rsm_cache.insert(rsm_path.clone(), rsm);
        }

        let instance_matrix = build_instance_matrix(rsw_model, scale_factor, center_x, center_z);
        let alpha = rsm_cache.get(&rsm_path)
            .and_then(|r| r.as_ref())
            .and_then(|r| r.alpha)
            .map(|a| a as f32 / 255.0)
            .unwrap_or(1.0);

        rsm_instances.entry(rsm_path).or_default().push(ModelInstance {
            instance_matrix,
            alpha,
        });
    }

    // Compile all models into texture-grouped vertices
    let mut texture_quads: HashMap<String, (Vec<ModelVertex>, Vec<u32>)> = HashMap::new();

    for (rsm_path, instances) in &rsm_instances {
        let rsm = match rsm_cache.get(rsm_path).and_then(|r| r.as_ref()) {
            Some(r) => r,
            None => continue,
        };

        if rsm.nodes.is_empty() {
            continue;
        }

        // Preload textures for this RSM
        preload_rsm_textures(rsm, grf, texture_cache, device, queue);

        let is_only = rsm.nodes.len() == 1;

        // Phase 1: compute bounding box and accumulate node matrices
        let (bbox, node_matrices) = calc_bounding_box(rsm);

        // Phase 2: compile each instance
        for inst in instances {
            for (node_idx, node) in rsm.nodes.iter().enumerate() {
                compile_node(
                    node,
                    rsm,
                    &node_matrices[node_idx],
                    &bbox,
                    &inst.instance_matrix,
                    is_only,
                    inst.alpha,
                    &mut texture_quads,
                );
            }
        }
    }

    // Flatten into single vertex/index buffers with draw batches
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut batches = Vec::new();

    for (tex_name, (verts, idxs)) in texture_quads {
        let vertex_offset = all_vertices.len() as u32;
        let start_index = all_indices.len() as u32;
        all_vertices.extend_from_slice(&verts);
        all_indices.extend(idxs.iter().map(|i| i + vertex_offset));
        batches.push(DrawBatch {
            texture_name: tex_name,
            start_index,
            index_count: idxs.len() as u32,
        });
    }

    (all_vertices, all_indices, batches)
}

fn build_instance_matrix(
    model: &ragnarok_formats::rsw::RswModel,
    scale_factor: f32,
    center_x: f32,
    center_z: f32,
) -> glam::Mat4 {
    let pos = glam::Vec3::new(
        model.position[0] * scale_factor + center_x,
        model.position[1] * scale_factor,
        model.position[2] * scale_factor + center_z,
    );

    let rot_x = model.rotation[0].to_radians();
    let rot_y = model.rotation[1].to_radians();
    let rot_z = model.rotation[2].to_radians();

    let scale = glam::Vec3::new(
        model.scale[0] * scale_factor,
        model.scale[1] * scale_factor,
        model.scale[2] * scale_factor,
    );

    glam::Mat4::from_translation(pos)
        * glam::Mat4::from_rotation_z(rot_z)
        * glam::Mat4::from_rotation_x(rot_x)
        * glam::Mat4::from_rotation_y(rot_y)
        * glam::Mat4::from_scale(scale)
}

fn mat3_to_mat4(m: &ragnarok_formats::Mat3) -> glam::Mat4 {
    // RSM local_transform is read row-major, gl-matrix treats as column-major
    // Our row 0 = their column 0, etc.
    glam::Mat4::from_cols(
        glam::Vec4::new(m[0][0], m[0][1], m[0][2], 0.0),
        glam::Vec4::new(m[1][0], m[1][1], m[1][2], 0.0),
        glam::Vec4::new(m[2][0], m[2][1], m[2][2], 0.0),
        glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
    )
}

fn vec3_from_arr(v: &ragnarok_formats::Vec3) -> glam::Vec3 {
    glam::Vec3::new(v[0], v[1], v[2])
}

fn calc_bounding_box(rsm: &RsmFile) -> (BoundingBox, Vec<glam::Mat4>) {
    let mut node_matrices = vec![glam::Mat4::IDENTITY; rsm.nodes.len()];
    let mut bbox = BoundingBox::new();
    let is_only = rsm.nodes.len() == 1;

    // Build name → index map for hierarchy
    let name_to_idx: HashMap<&str, usize> = rsm.nodes.iter().enumerate()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();

    // Find root node(s)
    let root_indices: Vec<usize> = if !rsm.root_node_names.is_empty() {
        rsm.root_node_names.iter()
            .filter_map(|name| name_to_idx.get(name.as_str()).copied())
            .collect()
    } else {
        vec![0]
    };

    // Recursive traversal
    fn traverse(
        node_idx: usize,
        parent_matrix: glam::Mat4,
        nodes: &[RsmNode],
        name_to_idx: &HashMap<&str, usize>,
        node_matrices: &mut [glam::Mat4],
        bbox: &mut BoundingBox,
        is_only: bool,
    ) {
        let node = &nodes[node_idx];

        // Accumulate: parent × translate(translation2) × rotation × scale
        let mut accumulated = parent_matrix
            * glam::Mat4::from_translation(vec3_from_arr(&node.translation2));

        // Apply rotation: static axis-angle or first keyframe quaternion
        if node.rot_keyframes.is_empty() {
            if let (Some(angle), Some(axis)) = (node.rotation_angle, node.rotation_axis) {
                let axis_vec = vec3_from_arr(&axis);
                if axis_vec.length_squared() > 0.0 {
                    accumulated = accumulated * glam::Mat4::from_axis_angle(axis_vec.normalize(), angle);
                }
            }
        } else {
            let q = &node.rot_keyframes[0].quaternion;
            let quat = glam::Quat::from_xyzw(q[0], q[1], q[2], q[3]);
            accumulated = accumulated * glam::Mat4::from_quat(quat);
        }

        // Apply scale
        if let Some(scale) = node.scale {
            accumulated = accumulated * glam::Mat4::from_scale(vec3_from_arr(&scale));
        }

        node_matrices[node_idx] = accumulated;

        // Local matrix for bounding box: accumulated × [offset] × mat3
        let mut local = accumulated;
        if !is_only {
            if let Some(offset) = node.translation1 {
                local = local * glam::Mat4::from_translation(vec3_from_arr(&offset));
            }
        }
        local = local * mat3_to_mat4(&node.local_transform);

        // Transform all vertices to compute bounds
        for vert in &node.vertices {
            let v = vec3_from_arr(vert);
            let world = local.transform_point3(v);
            bbox.extend(world);
        }

        // Recurse to children
        for (i, child) in nodes.iter().enumerate() {
            if i != node_idx && child.parent_name == node.name && node.name != node.parent_name {
                traverse(i, accumulated, nodes, name_to_idx, node_matrices, bbox, is_only);
            }
        }
    }

    for &root_idx in &root_indices {
        traverse(root_idx, glam::Mat4::IDENTITY, &rsm.nodes, &name_to_idx, &mut node_matrices, &mut bbox, is_only);
    }

    // Guard against models with no vertices (bbox stays at infinity → NaN center)
    if bbox.min.x == f32::INFINITY {
        bbox.min = glam::Vec3::ZERO;
        bbox.max = glam::Vec3::ZERO;
    }

    bbox.finalize();
    (bbox, node_matrices)
}

/// Phase 2: compile a single node's faces into world-space vertices.
fn compile_node(
    node: &RsmNode,
    rsm: &RsmFile,
    node_matrix: &glam::Mat4,
    bbox: &BoundingBox,
    instance_matrix: &glam::Mat4,
    is_only: bool,
    alpha: f32,
    texture_quads: &mut HashMap<String, (Vec<ModelVertex>, Vec<u32>)>,
) {
    // Build compile matrix: translate(-center) × node_matrix × [offset] × mat3
    let mut matrix = glam::Mat4::from_translation(glam::Vec3::new(
        -bbox.center.x,
        -bbox.max.y,
        -bbox.center.z,
    )) * *node_matrix;

    if !is_only {
        if let Some(offset) = node.translation1 {
            matrix = matrix * glam::Mat4::from_translation(vec3_from_arr(&offset));
        }
    }

    matrix = matrix * mat3_to_mat4(&node.local_transform);

    // Final model-view matrix
    let model_view = *instance_matrix * matrix;
    let normal_sign = if model_view.determinant() < 0.0 { -1.0 } else { 1.0 };

    // Transform all vertices to world space
    let world_verts: Vec<glam::Vec3> = node.vertices.iter()
        .map(|v| model_view.transform_point3(vec3_from_arr(v)))
        .collect();

    let vert_count = world_verts.len();

    // Emit faces grouped by texture
    for face in &node.faces {
        let v0_idx = face.vertex_ids[0] as usize;
        let v1_idx = face.vertex_ids[1] as usize;
        let v2_idx = face.vertex_ids[2] as usize;
        if v0_idx >= vert_count || v1_idx >= vert_count || v2_idx >= vert_count {
            continue;
        }

        let tex_name = resolve_texture_name(rsm, node, face.texture_index);
        let tex_path = format!("data/texture/{tex_name}");

        let v0 = world_verts[v0_idx];
        let v1 = world_verts[v1_idx];
        let v2 = world_verts[v2_idx];

        // Flat shading: normal from world-space edges
        // Flip if transform includes reflection (negative determinant)
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero() * normal_sign;
        let n = [normal.x, normal.y, normal.z];

        let entry = texture_quads.entry(tex_path).or_insert_with(|| (Vec::new(), Vec::new()));
        let base = entry.0.len() as u32;

        for i in 0..3 {
            let vid = face.vertex_ids[i] as usize;
            let tid = face.tex_vertex_ids[i] as usize;
            let pos = world_verts[vid];
            let (u, v) = if tid < node.tex_vertices.len() {
                (node.tex_vertices[tid].u, node.tex_vertices[tid].v)
            } else {
                (0.0, 0.0)
            };

            entry.0.push(ModelVertex {
                position: [pos.x, pos.y, pos.z],
                normal: n,
                tex_coord: [u, v],
                alpha,
            });
        }

        entry.1.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

fn resolve_texture_name(rsm: &RsmFile, node: &RsmNode, face_texture_index: u16) -> String {
    if !node.texture_names.is_empty() {
        // v >= 2.3: per-node texture names
        node.texture_names.get(face_texture_index as usize)
            .cloned()
            .unwrap_or_default()
    } else {
        // v < 2.3: global textures via indirection
        let tex_id = node.texture_ids.get(face_texture_index as usize)
            .copied()
            .unwrap_or(0) as usize;
        rsm.textures.get(tex_id)
            .cloned()
            .unwrap_or_default()
    }
}

fn preload_rsm_textures(
    rsm: &RsmFile,
    grf: &GrfArchive,
    texture_cache: &mut TextureCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    if !rsm.textures.is_empty() {
        for tex_name in &rsm.textures {
            let path = format!("data/texture/{tex_name}");
            texture_cache.get_or_load(&path, grf, device, queue, false);
        }
    }
    for node in &rsm.nodes {
        for tex_name in &node.texture_names {
            let path = format!("data/texture/{tex_name}");
            texture_cache.get_or_load(&path, grf, device, queue, false);
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

fn create_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    global_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("model"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/model.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("model"),
        bind_group_layouts: &[global_bind_group_layout, texture_bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("model"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[ModelVertex::LAYOUT],
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

    #[test]
    fn instance_matrix_places_model_at_map_center_when_position_is_zero() {
        let model = ragnarok_formats::rsw::RswModel {
            name: None,
            anim_type: None,
            anim_speed: None,
            block_type: None,
            model_name: String::new(),
            node_name: String::new(),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [10.0, 10.0, 10.0],
        };
        let mat = build_instance_matrix(&model, 1.0, 500.0, 500.0);
        let origin = mat.transform_point3(glam::Vec3::ZERO);
        assert!((origin.x - 500.0).abs() < 0.01, "x={}", origin.x);
        assert!(origin.y.abs() < 0.01, "y={}", origin.y);
        assert!((origin.z - 500.0).abs() < 0.01, "z={}", origin.z);

        // Native RO coords: local Y=100 maps directly with scale=10 → world Y = 1000
        let point = mat.transform_point3(glam::Vec3::new(0.0, 100.0, 0.0));
        assert!((point.y - 1000.0).abs() < 0.01, "y={}", point.y);
    }

    #[test]
    fn mat3_to_mat4_identity() {
        let m: ragnarok_formats::Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let result = mat3_to_mat4(&m);
        let expected = glam::Mat4::IDENTITY;
        for i in 0..16 {
            assert!((result.to_cols_array()[i] - expected.to_cols_array()[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn bounding_box_single_node_model() {
        let rsm = RsmFile {
            version: (1, 4),
            anim_length: 0,
            shade_type: 0,
            alpha: Some(255),
            fps: None,
            textures: vec!["test.bmp".into()],
            root_node_names: vec![],
            nodes: vec![RsmNode {
                name: "root".into(),
                parent_name: String::new(),
                texture_ids: vec![0],
                texture_names: vec![],
                local_transform: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                translation1: Some([0.0, 0.0, 0.0]),
                translation2: [0.0, 0.0, 0.0],
                rotation_angle: Some(0.0),
                rotation_axis: Some([0.0, 1.0, 0.0]),
                scale: Some([1.0, 1.0, 1.0]),
                vertices: vec![
                    [-1.0, -1.0, -1.0],
                    [1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0],
                ],
                tex_vertices: vec![],
                faces: vec![],
                scale_keyframes: vec![],
                rot_keyframes: vec![],
                translation_keyframes: vec![],
                textures_keyframes: vec![],
            }],
        };

        let (bbox, matrices) = calc_bounding_box(&rsm);
        assert_eq!(matrices.len(), 1);
        assert!((bbox.min.x - (-1.0)).abs() < 0.01, "min.x={}", bbox.min.x);
        assert!((bbox.max.x - 1.0).abs() < 0.01, "max.x={}", bbox.max.x);
        assert!((bbox.center.x - 0.0).abs() < 0.01, "center.x={}", bbox.center.x);
    }
}
