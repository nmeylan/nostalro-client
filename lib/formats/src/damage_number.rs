#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum TextureSource {
    Number,
    Message,
}

#[repr(C)]
pub struct DamageNumberQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub source: TextureSource,
    pub tex_idx: usize,
}
