//! D3D blend factor table used by STR effect frames.
//!
//! STR files store source/destination blend factors as the integer D3DBLEND
//! constants the original game uses. The renderer needs the equivalent
//! `wgpu::BlendFactor` value for pipeline creation.

/// Pre-classified blend mode for primitives that don't carry raw D3D factors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendKind {
    /// `src.rgb * src.a + dst.rgb * (1 - src.a)`
    Alpha,
    /// `src.rgb * src.a + dst.rgb` - used by most STR layers, auras, sparks.
    Additive,
    /// `src.rgb * dst.rgb` - darkening / shadow overlays.
    Multiply,
    /// Raw D3D source/dest factor pair from an STR frame.
    Raw { src: i32, dst: i32 },
}

/// Map a D3DBLEND_* integer to its `wgpu::BlendFactor` equivalent.
///
/// Reference: D3D8 `D3DBLEND` enum.
/// Unknown / unsupported values fall back to `One` (sane additive default).
pub fn d3d_blend_to_wgpu(d3d_blend: i32) -> wgpu::BlendFactor {
    match d3d_blend {
        1 => wgpu::BlendFactor::Zero,
        2 => wgpu::BlendFactor::One,
        3 => wgpu::BlendFactor::Src,
        4 => wgpu::BlendFactor::OneMinusSrc,
        5 => wgpu::BlendFactor::SrcAlpha,
        6 => wgpu::BlendFactor::OneMinusSrcAlpha,
        7 => wgpu::BlendFactor::DstAlpha,
        8 => wgpu::BlendFactor::OneMinusDstAlpha,
        9 => wgpu::BlendFactor::Dst,
        10 => wgpu::BlendFactor::OneMinusDst,
        11 => wgpu::BlendFactor::SrcAlphaSaturated,
        _ => wgpu::BlendFactor::One,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_d3d_factors() {
        assert_eq!(d3d_blend_to_wgpu(5), wgpu::BlendFactor::SrcAlpha);
        assert_eq!(d3d_blend_to_wgpu(6), wgpu::BlendFactor::OneMinusSrcAlpha);
        assert_eq!(d3d_blend_to_wgpu(2), wgpu::BlendFactor::One);
        assert_eq!(d3d_blend_to_wgpu(1), wgpu::BlendFactor::Zero);
    }

    #[test]
    fn unknown_factor_falls_back_to_one() {
        assert_eq!(d3d_blend_to_wgpu(0), wgpu::BlendFactor::One);
        assert_eq!(d3d_blend_to_wgpu(999), wgpu::BlendFactor::One);
    }
}
