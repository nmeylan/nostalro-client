use std::cell::RefCell;
use std::collections::HashMap;

use crate::FormatError;

use super::{Gr2Container, read_u32};

#[derive(Clone, Copy, PartialEq, Eq)]
enum MemberType {
    End,
    Inline,
    Reference,
    ReferenceToArray,
    ArrayOfReferences,
    VariantReference,
    Unsupported,
    ReferenceToVariantArray,
    String,
    Transform,
    Real32,
    Int8,
    UInt8,
    BinormalInt8,
    NormalUInt8,
    Int16,
    UInt16,
    BinormalInt16,
    NormalUInt16,
    Int32,
    UInt32,
    Real16,
    EmptyReference,
}

impl MemberType {
    fn from_u32(v: u32) -> Option<Self> {
        use MemberType::*;
        Some(match v {
            0 => End,
            1 => Inline,
            2 => Reference,
            3 => ReferenceToArray,
            4 => ArrayOfReferences,
            5 => VariantReference,
            6 => Unsupported,
            7 => ReferenceToVariantArray,
            8 => String,
            9 => Transform,
            10 => Real32,
            11 => Int8,
            12 => UInt8,
            13 => BinormalInt8,
            14 => NormalUInt8,
            15 => Int16,
            16 => UInt16,
            17 => BinormalInt16,
            18 => NormalUInt16,
            19 => Int32,
            20 => UInt32,
            21 => Real16,
            22 => EmptyReference,
            _ => return None,
        })
    }
}

const TYPE_DEF_SIZE: usize = 32;
const TRANSFORM_SIZE: usize = 68;

struct RawMember {
    mtype: MemberType,
    name: Option<String>,
    ref_off: usize,
    array_width: u32,
}

struct Field {
    name: Option<String>,
    ref_off: usize,
    slot: usize,
}

/// Navigates the GR2 type tree over the decompressed buffer: reads scalars at
/// byte offsets, resolves a type definition's members into their byte slots, and
/// caches computed type sizes (member layout is recursive, so sizing the same
/// type repeatedly is common).
struct Nav<'a> {
    data: &'a [u8],
    size_cache: RefCell<HashMap<usize, usize>>,
}

impl<'a> Nav<'a> {
    fn new(data: &'a [u8]) -> Self {
        Nav {
            data,
            size_cache: RefCell::new(HashMap::new()),
        }
    }

    fn u32(&self, off: usize) -> Result<u32, FormatError> {
        read_u32(self.data, off)
    }

    fn i32(&self, off: usize) -> Result<i32, FormatError> {
        Ok(self.u32(off)? as i32)
    }

    fn f32(&self, off: usize) -> Result<f32, FormatError> {
        Ok(f32::from_bits(self.u32(off)?))
    }

    fn cstr(&self, off: usize) -> Option<String> {
        if off == 0 {
            return None;
        }
        let bytes = self.data.get(off..)?;
        let end = bytes.iter().position(|&b| b == 0)?;
        Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }

    fn members(&self, type_off: usize) -> Result<Vec<RawMember>, FormatError> {
        let mut out = Vec::new();
        let mut a = type_off;
        loop {
            let mtype = MemberType::from_u32(self.u32(a)?)
                .ok_or_else(|| FormatError::DecompressionFailed("gr2: bad member type".into()))?;
            if mtype == MemberType::End {
                break;
            }
            out.push(RawMember {
                mtype,
                name: self.cstr(self.u32(a + 4)? as usize),
                ref_off: self.u32(a + 8)? as usize,
                array_width: self.u32(a + 12)?,
            });
            a += TYPE_DEF_SIZE;
        }
        Ok(out)
    }

    fn type_size(&self, type_off: usize) -> Result<usize, FormatError> {
        if let Some(&s) = self.size_cache.borrow().get(&type_off) {
            return Ok(s);
        }
        self.size_cache.borrow_mut().insert(type_off, 0);
        let mut total = 0;
        for m in self.members(type_off)? {
            total += self.member_size(&m)?;
        }
        self.size_cache.borrow_mut().insert(type_off, total);
        Ok(total)
    }

    fn member_size(&self, m: &RawMember) -> Result<usize, FormatError> {
        use MemberType::*;
        let n = m.array_width.max(1) as usize;
        Ok(match m.mtype {
            Inline => self.type_size(m.ref_off)? * n,
            Reference | String | EmptyReference => 4,
            ReferenceToArray | ArrayOfReferences | VariantReference => 8,
            ReferenceToVariantArray => 12,
            Transform => TRANSFORM_SIZE,
            Real32 | Int32 | UInt32 => 4 * n,
            Int16 | UInt16 | BinormalInt16 | NormalUInt16 | Real16 => 2 * n,
            Int8 | UInt8 | BinormalInt8 | NormalUInt8 => n,
            Unsupported | End => 0,
        })
    }

    fn fields(&self, type_off: usize, obj_off: usize) -> Result<Vec<Field>, FormatError> {
        let mut out = Vec::new();
        let mut slot = obj_off;
        for m in self.members(type_off)? {
            let size = self.member_size(&m)?;
            out.push(Field {
                name: m.name,
                ref_off: m.ref_off,
                slot,
            });
            slot += size;
        }
        Ok(out)
    }

    fn array(&self, slot: usize) -> Result<(usize, usize), FormatError> {
        Ok((
            self.i32(slot)?.max(0) as usize,
            self.u32(slot + 4)? as usize,
        ))
    }

    fn variant_array(&self, slot: usize) -> Result<(usize, usize, usize), FormatError> {
        Ok((
            self.u32(slot)? as usize,
            self.i32(slot + 4)?.max(0) as usize,
            self.u32(slot + 8)? as usize,
        ))
    }
}

fn find<'f>(fields: &'f [Field], name: &str) -> Option<&'f Field> {
    fields.iter().find(|f| f.name.as_deref() == Some(name))
}

fn i32_field(nav: &Nav, fields: &[Field], name: &str) -> i32 {
    find(fields, name)
        .and_then(|f| nav.i32(f.slot).ok())
        .unwrap_or(0)
}

fn f32_field(nav: &Nav, fields: &[Field], name: &str) -> f32 {
    find(fields, name)
        .and_then(|f| nav.f32(f.slot).ok())
        .unwrap_or(0.0)
}

fn string_field(nav: &Nav, fields: &[Field], name: &str) -> String {
    find(fields, name)
        .and_then(|f| nav.cstr(nav.u32(f.slot).ok()? as usize))
        .unwrap_or_default()
}

fn read_transform(nav: &Nav, slot: usize) -> Result<Gr2Transform, FormatError> {
    let mut pos = [0.0f32; 3];
    for (i, p) in pos.iter_mut().enumerate() {
        *p = nav.f32(slot + 4 + i * 4)?;
    }
    let mut rotation = [0.0f32; 4];
    for (i, r) in rotation.iter_mut().enumerate() {
        *r = nav.f32(slot + 16 + i * 4)?;
    }
    let mut scale_shear = [0.0f32; 9];
    for (i, s) in scale_shear.iter_mut().enumerate() {
        *s = nav.f32(slot + 32 + i * 4)?;
    }
    Ok(Gr2Transform {
        flags: nav.u32(slot)?,
        position: pos,
        rotation,
        scale_shear,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Gr2Transform {
    pub flags: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale_shear: [f32; 9],
}

impl Gr2Transform {
    /// Identity transform: no translation, identity quaternion, identity
    /// scale-shear matrix. Used when a bone/model/track omits its Transform.
    pub const IDENTITY: Gr2Transform = Gr2Transform {
        flags: 0,
        position: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale_shear: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };
}

pub const TEXTURE_ENCODING_RAW: i32 = 1;
pub const TEXTURE_ENCODING_S3TC: i32 = 2;
pub const TEXTURE_ENCODING_BINK: i32 = 3;

#[derive(Clone, Debug)]
pub struct Gr2Texture {
    pub from_file_name: String,
    pub width: i32,
    pub height: i32,
    pub encoding: i32,
    pub sub_format: i32,
    pub bytes_per_pixel: i32,
    /// `Layout.BitsForComponent` (RGBA); alpha is present iff `[3] != 0`.
    pub component_bits: [i32; 4],
    pub pixels: Vec<u8>,
}

impl Gr2Texture {
    pub fn has_alpha(&self) -> bool {
        self.component_bits[3] != 0
    }

    /// Decode the top mip level to RGBA8888 (`width*height*4` bytes).
    pub fn to_rgba(&self) -> Result<Vec<u8>, FormatError> {
        match self.encoding {
            TEXTURE_ENCODING_RAW if self.bytes_per_pixel == 4 => {
                let expected = self.width as usize * self.height as usize * 4;
                if self.pixels.len() < expected {
                    return Err(FormatError::UnexpectedEof);
                }
                Ok(self.pixels[..expected].to_vec())
            }
            TEXTURE_ENCODING_BINK => {
                crate::gr2::bink::decode(&self.pixels, self.width, self.height, self.has_alpha())
            }
            other => Err(FormatError::DecompressionFailed(format!(
                "gr2: unsupported texture encoding {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gr2Material {
    pub name: String,
    pub texture_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Gr2Bone {
    pub name: String,
    pub parent_index: i32,
    pub transform: Gr2Transform,
    pub inverse_world: [f32; 16],
}

#[derive(Clone, Debug)]
pub struct Gr2Skeleton {
    pub name: String,
    pub bones: Vec<Gr2Bone>,
}

#[derive(Clone, Copy, Debug)]
pub struct Gr2Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub bone_weights: [u8; 4],
    pub bone_indices: [u8; 4],
}

#[derive(Clone, Debug)]
pub struct Gr2VertexData {
    pub vertices: Vec<Gr2Vertex>,
}

#[derive(Clone, Copy, Debug)]
pub struct Gr2TriGroup {
    pub material_index: i32,
    pub tri_first: i32,
    pub tri_count: i32,
}

#[derive(Clone, Debug)]
pub struct Gr2TriTopology {
    pub groups: Vec<Gr2TriGroup>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct Gr2Mesh {
    pub name: String,
    pub vertex_data_index: Option<usize>,
    pub topology_index: Option<usize>,
    pub material_indices: Vec<usize>,
    pub bone_bindings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Gr2Model {
    pub name: String,
    pub skeleton_index: Option<usize>,
    pub initial_placement: Gr2Transform,
    pub mesh_indices: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Gr2Curve {
    pub degree: i32,
    pub knots: Vec<f32>,
    pub controls: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct Gr2TransformTrack {
    pub name: String,
    pub position: Gr2Curve,
    pub orientation: Gr2Curve,
    pub scale_shear: Gr2Curve,
}

#[derive(Clone, Debug)]
pub struct Gr2TrackGroup {
    pub name: String,
    pub transform_tracks: Vec<Gr2TransformTrack>,
    pub initial_placement: Gr2Transform,
}

#[derive(Clone, Debug)]
pub struct Gr2Animation {
    pub name: String,
    pub duration: f32,
    pub time_step: f32,
    pub track_group_indices: Vec<usize>,
}

pub struct Gr2File {
    pub textures: Vec<Gr2Texture>,
    pub materials: Vec<Gr2Material>,
    pub skeletons: Vec<Gr2Skeleton>,
    pub vertex_datas: Vec<Gr2VertexData>,
    pub tri_topologies: Vec<Gr2TriTopology>,
    pub meshes: Vec<Gr2Mesh>,
    pub models: Vec<Gr2Model>,
    pub track_groups: Vec<Gr2TrackGroup>,
    pub animations: Vec<Gr2Animation>,
}

impl Gr2File {
    pub fn parse(container: &Gr2Container) -> Result<Self, FormatError> {
        let nav = Nav::new(&container.data);
        let root_type = container.ref_offset(container.type_ref);
        let root_obj = container.ref_offset(container.root_ref);
        let root = nav.fields(root_type, root_obj)?;

        // Each collection is parsed alongside the file offset of every element;
        // references between collections store a target's file offset, which we
        // resolve to an array index through `offset_index` maps below.
        let (textures, texture_offs) = parse_array(&nav, &root, "Textures", parse_texture)?;
        let (materials, material_offs) = parse_array(&nav, &root, "Materials", parse_material_raw)?;
        let (skeletons, skeleton_offs) = parse_array(&nav, &root, "Skeletons", parse_skeleton)?;
        let (vertex_datas, vertex_data_offs) =
            parse_array(&nav, &root, "VertexDatas", parse_vertex_data)?;
        let (tri_topologies, topology_offs) =
            parse_array(&nav, &root, "TriTopologies", parse_topology)?;
        let (meshes, mesh_offs) = parse_array(&nav, &root, "Meshes", parse_mesh_raw)?;
        let (models, _) = parse_array(&nav, &root, "Models", parse_model_raw)?;
        let (track_groups, track_group_offs) =
            parse_array(&nav, &root, "TrackGroups", parse_track_group)?;
        let (animations, _) = parse_array(&nav, &root, "Animations", parse_animation_raw)?;

        let texture_index = offset_index(&texture_offs);
        let material_index = offset_index(&material_offs);
        let skeleton_index = offset_index(&skeleton_offs);
        let vertex_data_index = offset_index(&vertex_data_offs);
        let topology_index = offset_index(&topology_offs);
        let mesh_index = offset_index(&mesh_offs);
        let track_group_index = offset_index(&track_group_offs);

        let (mut materials, material_refs): (Vec<Gr2Material>, Vec<MaterialRefs>) =
            materials.into_iter().unzip();
        for (m, refs) in materials.iter_mut().zip(&material_refs) {
            m.texture_index = refs.texture.and_then(|o| texture_index.get(&o).copied());
        }
        // Sub-materials ("NN - Default" parents that meshes bind) carry no direct
        // texture; they inherit it through a referenced map. A material's map may
        // itself be such a parent, so iterate to a fixed point to follow chains.
        for _ in 0..4 {
            let snapshot: Vec<Option<usize>> = materials.iter().map(|m| m.texture_index).collect();
            let mut changed = false;
            for (m, refs) in materials.iter_mut().zip(&material_refs) {
                if m.texture_index.is_none() {
                    m.texture_index = refs
                        .maps
                        .iter()
                        .filter_map(|&o| material_index.get(&o).copied())
                        .find_map(|mi| snapshot[mi]);
                    changed |= m.texture_index.is_some();
                }
            }
            if !changed {
                break;
            }
        }

        let meshes = meshes
            .into_iter()
            .map(|(mut mesh, refs)| {
                mesh.vertex_data_index = refs
                    .vertex_data
                    .and_then(|o| vertex_data_index.get(&o).copied());
                mesh.topology_index = refs.topology.and_then(|o| topology_index.get(&o).copied());
                mesh.material_indices = refs
                    .materials
                    .iter()
                    .filter_map(|&o| material_index.get(&o).copied())
                    .collect();
                mesh
            })
            .collect();

        let models = models
            .into_iter()
            .map(|(mut model, refs)| {
                model.skeleton_index = refs.skeleton.and_then(|o| skeleton_index.get(&o).copied());
                model.mesh_indices = refs
                    .meshes
                    .iter()
                    .filter_map(|&o| mesh_index.get(&o).copied())
                    .collect();
                model
            })
            .collect();

        let animations = animations
            .into_iter()
            .map(|(mut anim, tg_offs)| {
                anim.track_group_indices = tg_offs
                    .iter()
                    .filter_map(|&o| track_group_index.get(&o).copied())
                    .collect();
                anim
            })
            .collect();

        Ok(Gr2File {
            textures,
            materials,
            skeletons,
            vertex_datas,
            tri_topologies,
            meshes,
            models,
            track_groups,
            animations,
        })
    }
}

/// Map each element's file offset to its index in the parsed vector. References
/// between collections are stored as file offsets; this inverts that into an
/// O(1) lookup during cross-reference resolution.
fn offset_index(offs: &[usize]) -> HashMap<usize, usize> {
    offs.iter().enumerate().map(|(i, &o)| (o, i)).collect()
}

/// Parse an array-of-references root field, returning the parsed elements and
/// each element's file offset (aligned by index  null entries are skipped in
/// both). Callers turn the offsets into an [`offset_index`] map.
fn parse_array<T>(
    nav: &Nav,
    root: &[Field],
    name: &str,
    f: impl Fn(&Nav, usize, usize) -> Result<T, FormatError>,
) -> Result<(Vec<T>, Vec<usize>), FormatError> {
    let Some(field) = find(root, name) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let (count, ptr) = nav.array(field.slot)?;
    let elem_type = field.ref_off;
    let mut out = Vec::with_capacity(count);
    let mut offs = Vec::with_capacity(count);
    for i in 0..count {
        let obj = nav.u32(ptr + i * 4)? as usize;
        if obj == 0 {
            continue;
        }
        offs.push(obj);
        out.push(f(nav, elem_type, obj)?);
    }
    Ok((out, offs))
}

fn parse_texture(nav: &Nav, type_off: usize, obj: usize) -> Result<Gr2Texture, FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut pixels = Vec::new();
    if let Some(images) = find(&fields, "Images") {
        let (img_count, img_ptr) = nav.array(images.slot)?;
        if img_count > 0 && img_ptr != 0 {
            let img_fields = nav.fields(images.ref_off, img_ptr)?;
            if let Some(mips) = find(&img_fields, "MIPLevels") {
                let (mip_count, mip_ptr) = nav.array(mips.slot)?;
                if mip_count > 0 && mip_ptr != 0 {
                    let mip_fields = nav.fields(mips.ref_off, mip_ptr)?;
                    if let Some(px) = find(&mip_fields, "Pixels") {
                        let (px_count, px_ptr) = nav.array(px.slot)?;
                        if px_ptr != 0 {
                            pixels = nav
                                .data
                                .get(px_ptr..px_ptr + px_count)
                                .unwrap_or_default()
                                .to_vec();
                        }
                    }
                }
            }
        }
    }
    let mut bytes_per_pixel = 0;
    let mut component_bits = [0i32; 4];
    if let Some(layout) = find(&fields, "Layout") {
        let layout_fields = nav.fields(layout.ref_off, layout.slot).unwrap_or_default();
        bytes_per_pixel = i32_field(nav, &layout_fields, "BytesPerPixel");
        if let Some(bits) = find(&layout_fields, "BitsForComponent") {
            for (i, b) in component_bits.iter_mut().enumerate() {
                *b = nav.i32(bits.slot + i * 4).unwrap_or(0);
            }
        }
    }
    Ok(Gr2Texture {
        from_file_name: string_field(nav, &fields, "FromFileName"),
        width: i32_field(nav, &fields, "Width"),
        height: i32_field(nav, &fields, "Height"),
        encoding: i32_field(nav, &fields, "Encoding"),
        sub_format: i32_field(nav, &fields, "SubFormat"),
        bytes_per_pixel,
        component_bits,
        pixels,
    })
}

struct MaterialRefs {
    /// Direct `Texture` reference, if any.
    texture: Option<usize>,
    /// `Maps[i].Map` sub-material references; materials without a direct
    /// texture (the "NN - Default" parents meshes bind) carry it there.
    maps: Vec<usize>,
}

fn parse_material_raw(
    nav: &Nav,
    type_off: usize,
    obj: usize,
) -> Result<(Gr2Material, MaterialRefs), FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let texture = find(&fields, "Texture")
        .and_then(|f| nav.u32(f.slot).ok())
        .filter(|&p| p != 0)
        .map(|p| p as usize);
    let mut maps = Vec::new();
    if let Some(maps_f) = find(&fields, "Maps") {
        let (count, ptr) = nav.array(maps_f.slot)?;
        let stride = nav.type_size(maps_f.ref_off)?;
        for i in 0..count {
            let entry = nav.fields(maps_f.ref_off, ptr + i * stride)?;
            if let Some(m) = find(&entry, "Map") {
                let p = nav.u32(m.slot)? as usize;
                if p != 0 {
                    maps.push(p);
                }
            }
        }
    }
    Ok((
        Gr2Material {
            name: string_field(nav, &fields, "Name"),
            texture_index: None,
        },
        MaterialRefs { texture, maps },
    ))
}

fn parse_skeleton(nav: &Nav, type_off: usize, obj: usize) -> Result<Gr2Skeleton, FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut bones = Vec::new();
    if let Some(bones_f) = find(&fields, "Bones") {
        let (count, ptr) = nav.array(bones_f.slot)?;
        let stride = nav.type_size(bones_f.ref_off)?;
        for i in 0..count {
            let bf = nav.fields(bones_f.ref_off, ptr + i * stride)?;
            let transform = find(&bf, "Transform")
                .map(|f| read_transform(nav, f.slot))
                .transpose()?
                .unwrap_or(Gr2Transform::IDENTITY);
            let mut inverse_world = [0.0f32; 16];
            if let Some(iw) = find(&bf, "InverseWorldTransform") {
                for (j, v) in inverse_world.iter_mut().enumerate() {
                    *v = nav.f32(iw.slot + j * 4)?;
                }
            }
            bones.push(Gr2Bone {
                name: string_field(nav, &bf, "Name"),
                parent_index: i32_field(nav, &bf, "ParentIndex"),
                transform,
                inverse_world,
            });
        }
    }
    Ok(Gr2Skeleton {
        name: string_field(nav, &fields, "Name"),
        bones,
    })
}

fn parse_vertex_data(nav: &Nav, type_off: usize, obj: usize) -> Result<Gr2VertexData, FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut vertices = Vec::new();
    if let Some(vf) = find(&fields, "Vertices") {
        let (vtype, count, ptr) = nav.variant_array(vf.slot)?;
        if vtype != 0 && ptr != 0 {
            let stride = nav.type_size(vtype)?;
            let layout = nav.fields(vtype, 0)?;
            let comp = |name: &str| find(&layout, name).map(|f| f.slot);
            let pos = comp("Position");
            let normal = comp("Normal");
            let uv = comp("TextureCoordinates0");
            let bw = comp("BoneWeights");
            let bi = comp("BoneIndices");
            for i in 0..count {
                let base = ptr + i * stride;
                let read3 = |o: Option<usize>| -> Result<[f32; 3], FormatError> {
                    match o {
                        None => Ok([0.0; 3]),
                        Some(o) => Ok([
                            nav.f32(base + o)?,
                            nav.f32(base + o + 4)?,
                            nav.f32(base + o + 8)?,
                        ]),
                    }
                };
                let mut uv2 = [0.0f32; 2];
                if let Some(o) = uv {
                    uv2[0] = nav.f32(base + o)?;
                    uv2[1] = nav.f32(base + o + 4)?;
                }
                let read4u8 = |o: Option<usize>| -> [u8; 4] {
                    let mut v = [0u8; 4];
                    if let Some(o) = o {
                        for (k, x) in v.iter_mut().enumerate() {
                            *x = nav.data.get(base + o + k).copied().unwrap_or(0);
                        }
                    }
                    v
                };
                vertices.push(Gr2Vertex {
                    position: read3(pos)?,
                    normal: read3(normal)?,
                    uv: uv2,
                    bone_weights: read4u8(bw),
                    bone_indices: read4u8(bi),
                });
            }
        }
    }
    Ok(Gr2VertexData { vertices })
}

fn parse_topology(nav: &Nav, type_off: usize, obj: usize) -> Result<Gr2TriTopology, FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut groups = Vec::new();
    if let Some(gf) = find(&fields, "Groups") {
        let (count, ptr) = nav.array(gf.slot)?;
        let stride = nav.type_size(gf.ref_off)?;
        for i in 0..count {
            let g = nav.fields(gf.ref_off, ptr + i * stride)?;
            groups.push(Gr2TriGroup {
                material_index: i32_field(nav, &g, "MaterialIndex"),
                tri_first: i32_field(nav, &g, "TriFirst"),
                tri_count: i32_field(nav, &g, "TriCount"),
            });
        }
    }
    let mut indices = Vec::new();
    if let Some(idx) = find(&fields, "Indices") {
        let (count, ptr) = nav.array(idx.slot)?;
        if ptr != 0 {
            for i in 0..count {
                indices.push(nav.u32(ptr + i * 4)?);
            }
        }
    }
    // Fall back to the 16-bit index buffer if the 32-bit one was absent.
    if indices.is_empty()
        && let Some(idx) = find(&fields, "Indices16")
    {
        let (count, ptr) = nav.array(idx.slot)?;
        if ptr != 0 {
            for i in 0..count {
                let s = nav
                    .data
                    .get(ptr + i * 2..ptr + i * 2 + 2)
                    .ok_or(FormatError::UnexpectedEof)?;
                indices.push(u16::from_le_bytes([s[0], s[1]]) as u32);
            }
        }
    }
    Ok(Gr2TriTopology { groups, indices })
}

struct MeshRefs {
    vertex_data: Option<usize>,
    topology: Option<usize>,
    materials: Vec<usize>,
}

fn parse_mesh_raw(
    nav: &Nav,
    type_off: usize,
    obj: usize,
) -> Result<(Gr2Mesh, MeshRefs), FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let ref_ptr = |name: &str| {
        find(&fields, name)
            .and_then(|f| nav.u32(f.slot).ok())
            .filter(|&p| p != 0)
            .map(|p| p as usize)
    };
    let mut materials = Vec::new();
    if let Some(mb) = find(&fields, "MaterialBindings") {
        let (count, ptr) = nav.array(mb.slot)?;
        let stride = nav.type_size(mb.ref_off)?;
        for i in 0..count {
            let b = nav.fields(mb.ref_off, ptr + i * stride)?;
            if let Some(mf) = find(&b, "Material") {
                let p = nav.u32(mf.slot)? as usize;
                if p != 0 {
                    materials.push(p);
                }
            }
        }
    }
    let mut bone_bindings = Vec::new();
    if let Some(bb) = find(&fields, "BoneBindings") {
        let (count, ptr) = nav.array(bb.slot)?;
        let stride = nav.type_size(bb.ref_off)?;
        for i in 0..count {
            let b = nav.fields(bb.ref_off, ptr + i * stride)?;
            bone_bindings.push(string_field(nav, &b, "BoneName"));
        }
    }
    Ok((
        Gr2Mesh {
            name: string_field(nav, &fields, "Name"),
            vertex_data_index: None,
            topology_index: None,
            material_indices: Vec::new(),
            bone_bindings,
        },
        MeshRefs {
            vertex_data: ref_ptr("PrimaryVertexData"),
            topology: ref_ptr("PrimaryTopology"),
            materials,
        },
    ))
}

struct ModelRefs {
    skeleton: Option<usize>,
    meshes: Vec<usize>,
}

fn parse_model_raw(
    nav: &Nav,
    type_off: usize,
    obj: usize,
) -> Result<(Gr2Model, ModelRefs), FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let skeleton = find(&fields, "Skeleton")
        .and_then(|f| nav.u32(f.slot).ok())
        .filter(|&p| p != 0)
        .map(|p| p as usize);
    let mut meshes = Vec::new();
    if let Some(mb) = find(&fields, "MeshBindings") {
        let (count, ptr) = nav.array(mb.slot)?;
        let stride = nav.type_size(mb.ref_off)?;
        for i in 0..count {
            let b = nav.fields(mb.ref_off, ptr + i * stride)?;
            if let Some(mf) = find(&b, "Mesh") {
                let p = nav.u32(mf.slot)? as usize;
                if p != 0 {
                    meshes.push(p);
                }
            }
        }
    }
    let initial_placement = find(&fields, "InitialPlacement")
        .map(|f| read_transform(nav, f.slot))
        .transpose()?
        .unwrap_or(Gr2Transform::IDENTITY);
    Ok((
        Gr2Model {
            name: string_field(nav, &fields, "Name"),
            skeleton_index: None,
            initial_placement,
            mesh_indices: Vec::new(),
        },
        ModelRefs { skeleton, meshes },
    ))
}

fn parse_curve(nav: &Nav, type_off: usize, slot: usize) -> Result<Gr2Curve, FormatError> {
    let fields = nav.fields(type_off, slot)?;
    let read_reals = |name: &str| -> Result<Vec<f32>, FormatError> {
        let Some(f) = find(&fields, name) else {
            return Ok(Vec::new());
        };
        let (count, ptr) = nav.array(f.slot)?;
        let mut out = Vec::with_capacity(count);
        if ptr != 0 {
            for i in 0..count {
                out.push(nav.f32(ptr + i * 4)?);
            }
        }
        Ok(out)
    };
    Ok(Gr2Curve {
        degree: i32_field(nav, &fields, "Degree"),
        knots: read_reals("Knots")?,
        controls: read_reals("Controls")?,
    })
}

fn parse_track_group(nav: &Nav, type_off: usize, obj: usize) -> Result<Gr2TrackGroup, FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut transform_tracks = Vec::new();
    if let Some(tt) = find(&fields, "TransformTracks") {
        let (count, ptr) = nav.array(tt.slot)?;
        let stride = nav.type_size(tt.ref_off)?;
        for i in 0..count {
            let t = nav.fields(tt.ref_off, ptr + i * stride)?;
            let curve = |name: &str| -> Result<Gr2Curve, FormatError> {
                match find(&t, name) {
                    Some(f) => parse_curve(nav, f.ref_off, f.slot),
                    None => Ok(Gr2Curve::default()),
                }
            };
            transform_tracks.push(Gr2TransformTrack {
                name: string_field(nav, &t, "Name"),
                position: curve("PositionCurve")?,
                orientation: curve("OrientationCurve")?,
                scale_shear: curve("ScaleShearCurve")?,
            });
        }
    }
    let initial_placement = find(&fields, "InitialPlacement")
        .map(|f| read_transform(nav, f.slot))
        .transpose()?
        .unwrap_or(Gr2Transform::IDENTITY);
    Ok(Gr2TrackGroup {
        name: string_field(nav, &fields, "Name"),
        transform_tracks,
        initial_placement,
    })
}

fn parse_animation_raw(
    nav: &Nav,
    type_off: usize,
    obj: usize,
) -> Result<(Gr2Animation, Vec<usize>), FormatError> {
    let fields = nav.fields(type_off, obj)?;
    let mut tg_offs = Vec::new();
    if let Some(tg) = find(&fields, "TrackGroups") {
        let (count, ptr) = nav.array(tg.slot)?;
        for i in 0..count {
            let p = nav.u32(ptr + i * 4)? as usize;
            if p != 0 {
                tg_offs.push(p);
            }
        }
    }
    Ok((
        Gr2Animation {
            name: string_field(nav, &fields, "Name"),
            duration: f32_field(nav, &fields, "Duration"),
            time_step: f32_field(nav, &fields, "TimeStep"),
            track_group_indices: Vec::new(),
        },
        tg_offs,
    ))
}
