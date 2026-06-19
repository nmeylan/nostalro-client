//! Renderer-agnostic effect draw primitives.
//!
//! Effect implementations (in `effects/`) emit `EffectPrimitiveDraw` entries
//! into an `EffectDrawList`; the renderer crate consumes the list each frame
//! and dispatches them to dedicated GPU pipelines.

/// Selects how `Frustum` modulates each top-ring vertex's height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrustumWaveMode {
    /// `wave_amplitude * sin(angle * wave_frequency + wave_phase)` — full
    /// sine around the ring. Default; preserves legacy LandProtector / Volcano
    /// behaviour.
    #[default]
    Sine,
    /// Single positive lobe centred opposite the seam — matches the original
    /// game's casting-cone flame-tip envelope. `wave_amplitude` may
    /// go negative across frames to flip the lobe inward.
    SaintBell,
}

/// Pre-classified blend mode for effect primitives.
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

/// Lifecycle signal returned by `Effect::update`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStatus {
    Running,
    Dead,
}

/// `aim_target` point that makes a `SpriteParticle` face **away** from
/// `target` — a 180° screen flip — for SPRs whose head points opposite the
/// arrow convention the shared aim assumes (a 180° yaw flip,
/// for `fireball.spr` / brandish spear). Mirrors `position` across the sprite;
/// the near-orthographic effect camera makes that world mirror project to ≈ a
/// half-turn in screen space.
pub fn aim_backward(position: [f32; 3], target: [f32; 3]) -> [f32; 3] {
    [
        2.0 * position[0] - target[0],
        2.0 * position[1] - target[1],
        2.0 * position[2] - target[2],
    ]
}

/// One renderable primitive emitted by an effect. Effects don't depend on
/// wgpu types directly - they describe what they want drawn, and the effect
/// render pass turns each variant into pipeline calls.
#[derive(Clone, Debug)]
pub enum EffectPrimitiveDraw {
    /// Camera-facing textured quad.
    ///
    /// `rotation` rotates the quad in screen space around its centre (CCW in
    /// radians). 0 keeps the existing axis-aligned behaviour — used by every
    /// pre-Hit2 caller (aura, placeholder, animated-texture-billboard) and
    /// the renderer is a literal identity transform in that case. The Hit2
    /// petals set it per-petal to align each lens-flare quad's long axis
    /// with its radial direction.
    Billboard {
        pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Like [`Billboard`] but drawn as a near-plane 2D overlay that ignores 3D
    /// depth (a camera-locked flash). Screen placement, sizing and
    /// rotation are identical to [`Billboard`]; only the depth differs — the
    /// quad is never occluded by the ground, so entity-centred flash bursts
    /// (Bash/Flasher rays, etc.) whose geometry dips at or below the floor draw
    /// over it instead of being swallowed. Matches `BillboardDisc`/`BillboardRing`.
    ///
    /// [`Billboard`]: EffectPrimitiveDraw::Billboard
    BillboardFlash {
        pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing textured quad whose **depth** is taken from a separate
    /// ground anchor instead of its own (elevated) position.
    ///
    /// Identical to [`Billboard`] for screen placement and sizing, but the
    /// depth used for occlusion / sorting comes from `depth_pos` (typically the
    /// quad's position projected down onto the caster's feet plane) rather than
    /// `pos`. A character is drawn at a single flat depth (its feet anchor), so
    /// an orb lifted to shoulder height is *nearer* in view space than the feet
    /// plane (the camera looks down) and would always draw over the body. By
    /// deriving depth from where the orb stands on the ground, an orb orbiting
    /// behind the caster sits behind the body's depth plane (occluded) while one
    /// in front passes — reproducing the original game's behaviour where the
    /// spheres pass behind the back and reappear in front. The renderer biases
    /// this depth by the same world distance the entity sprite uses, so the
    /// comparison is purely front/back.
    ///
    /// [`Billboard`]: EffectPrimitiveDraw::Billboard
    BillboardDepthAnchored {
        pos: [f32; 3],
        depth_pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing filled disc with polar UV mapping.
    ///
    /// Like `Billboard` (anchor projected to screen, then geometry built in
    /// screen pixels via `ppu`), but the shape is a circle of `radius` world
    /// units instead of a quad. UV mapping matches `GroundDisc`'s
    /// radial convention: V=1 at the centre, V=0 at the
    /// outer rim — so a vertical-gradient texture like `alpha_down.tga`
    /// renders as a radial alpha gradient (opaque centre → transparent
    /// edge). `segments` controls the perimeter tessellation; `uv_repeat`
    /// is how many times the texture wraps around the circumference (U
    /// direction), matching `GroundDisc`'s convention.
    BillboardDisc {
        pos: [f32; 3],
        radius: f32,
        segments: u32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing textured annulus (screen-space ring).
    ///
    /// Like [`BillboardDisc`] (anchor projected, geometry built in screen
    /// pixels via `ppu`), but the shape is a thin ring of outer radius
    /// `radius` and radial `thickness` instead of a filled disc. UV
    /// mapping matches the screen-space circle convention: V=0 at the
    /// outer rim, V=1 at the inner rim, U wrapping `uv_repeat` times
    /// around the perimeter. `segments` controls perimeter tessellation.
    ///
    /// Use this for screen-space ring effects (a circle with a
    /// non-zero inner radius). For a filled disc with
    /// radial UV, use [`BillboardDisc`].
    ///
    /// [`BillboardDisc`]: EffectPrimitiveDraw::BillboardDisc
    BillboardRing {
        pos: [f32; 3],
        radius: f32,
        thickness: f32,
        segments: u32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Flat-on-ground textured annulus or partial-arc wedge.
    ///
    /// `arc_angle_deg` is the **total arc span** in degrees (`>= 360` = full
    /// ring; smaller clips to a wedge starting at `+X` and sweeping CCW).
    /// `uv_repeat` is how many times the texture tiles around the ring along
    /// `u`; `v` runs 0 at the outer edge to 1 at the inner edge (matching the
    /// original game's flat-ring convention). `rotation` is a u-offset in
    /// radians, useful for spinning textures.
    GroundDisc {
        center: [f32; 3],
        radius: f32,
        thickness: f32,
        rotation: f32,
        arc_angle_deg: f32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Vertical "tube" between two coaxial polygons.
    ///
    /// `base` is the bottom-ring centre; the top ring sits `height` units
    /// above it (native RO coords, so `-Y` is up — the renderer applies the
    /// sign). `bottom_size`/`top_size` are the bottom/top radii (equal →
    /// cylinder; `top_size == 0` → cone). `sides` controls the segment count
    /// (4 = square pillar, 16-24 ≈ smooth circle). `rotation` is the angle
    /// in radians around the vertical axis the geometry starts at (also
    /// shifts the u-coordinate so the texture follows the geometry).
    /// `uv_scroll` is an additive `[u, v]` scroll, `uv_repeat` how many
    /// times the texture tiles around the circumference.
    ///
    /// `Frustum` and [`Cylinder`] describe the same geometry but differ in
    /// UV convention: `Frustum` lets a caller spread the texture continuously
    /// across the lateral surface, [`Cylinder`] mimics the original game's
    /// cylinder UV (`u += 0.25` per segment with wrap, four tiles
    /// per ring regardless of segment count). Callers wanting a
    /// straight cylinder/cone should prefer [`Cylinder`].
    ///
    /// [`Cylinder`]: EffectPrimitiveDraw::Cylinder
    ///
    /// Each top-ring vertex's height is offset by a wave function selected
    /// by `wave_mode`.
    /// * [`FrustumWaveMode::Sine`] (default) — `wave_amplitude *
    ///   sin(angle * wave_frequency + wave_phase)`. Driving `wave_phase`
    ///   over time makes the height wave travel around the ring
    ///   (LandProtector flame peaks rotate around the curtain). One peak
    ///   and one trough per cycle of `wave_frequency`.
    /// * [`FrustumWaveMode::SaintBell`] — `wave_amplitude * max(sin(across), 0)`
    ///   where `across = π * (segment_index / segments * 2 − 1)`. Single
    ///   positive lobe centred opposite the seam (`rotation`), zero at the
    ///   seam itself. Time-modulate by varying `wave_amplitude` per frame
    ///   (it may go negative to invert the lobe). Used by the casting
    ///   cones (BeginSpell, BeginSpell6): a fixed-azimuth
    ///   flame-tip bump that pulses in amplitude rather than rotating.
    /// `wave_amplitude == 0` produces a flat top ring in either mode.
    Frustum {
        base: [f32; 3],
        bottom_size: f32,
        top_size: f32,
        height: f32,
        sides: u32,
        /// Total arc span in degrees. `>= 360` (or any closed-loop value)
        /// draws a full ring; smaller values produce an **open strip**
        /// sweeping CCW from `rotation`, leaving a `360 - arc_angle_deg`
        /// gap. Matches the original game's cast-aura arc span:
        /// cast-circle petals use `315`, columns and
        /// most other emitters use `360`.
        arc_angle_deg: f32,
        rotation: f32,
        uv_repeat: f32,
        uv_scroll: [f32; 2],
        wave_amplitude: f32,
        wave_frequency: f32,
        wave_phase: f32,
        wave_mode: FrustumWaveMode,
        /// Tilt around the local X axis (radians), applied *after* the
        /// local-frame vertices are built and *before* `rotation_y_deg`.
        /// 0 = vertical pillar (default — preserves the existing
        /// behaviour for BottomSanc, cast-circle, volcano, BeginSpell6).
        /// The Hit family pitches its cone by `-90°`,
        /// which corresponds to `tilt_x_rad = -π/2`, laying the
        /// flared cone on its side so its axis points horizontally;
        /// `rotation_y_rad` then aims that axis at a compass heading.
        /// When `tilt_x_rad != 0`, `cull_back`'s outward-direction
        /// calculation no longer matches the physical front/back of the
        /// geometry — `cull_back: true` plus tilt is unsupported and will
        /// produce arbitrary fading; current users with tilt always set
        /// `cull_back: false`.
        tilt_x_rad: f32,
        /// Rotation around the world Y axis applied *after* `tilt_x_rad`,
        /// rotating the whole tilted geometry around the vertical axis
        /// through `base`. For a vertical pillar (`tilt_x_rad == 0`) this
        /// is equivalent to `rotation` (both spin the geometry around the
        /// cylinder's axis), but for a tilted cone this is the heading
        /// the cone's tip points at.
        rotation_y_rad: f32,
        /// When `true`, the renderer skips quads whose outward-facing normal
        /// points away from the camera — back-face culling
        /// for cones that should only show their outer surface
        /// (e.g. the flat-flaring cast aura). Default `false`
        /// preserves the existing behaviour where both faces of the cone
        /// are visible (BottomSanc pillar, cast-circle petals).
        cull_back: bool,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Square-based pyramid spike — four outward-facing triangles meeting at
    /// a single apex above the centre of a `2*size` × `2*size` base. Matches
    /// the original game's quad-horn spike.
    /// Used by Stormgust ice shards plus eight other effects in the
    /// original game.
    ///
    /// Local frame: base square on the
    /// local XY plane, apex at `(0, 0, height)` along local +Z. Rotations
    /// use row-vector matrix conventions, so callers can pass the original
    /// game's pitch / yaw values directly:
    /// * `tilt_x_deg`  — pitch. 0° = horizontal along
    ///   world +Z, 90° = straight UP (native RO), 100° = nearly vertical
    ///   with slight backward lean (Stormgust), 270° = straight DOWN.
    /// * `rotation_y_deg` — yaw. Rotates the tilted
    ///   spike around the world up-axis to face a compass heading.
    ///
    /// `base` is the bottom-centre point in world coords; the unrotated
    /// spike's apex sits at `base + (0, 0, height)`. This primitive uses
    /// native RO `-Y = up`, so no Y flip is applied at world
    /// output — `base[1]` lives directly in world Y.
    ///
    /// UV layout: each of the four triangles is a vertical strip
    /// on the texture; base vertices at `v = 1`, apex at `v = 0`; `u`
    /// advances by 0.2 per triangle starting at 0 (so the four strips
    /// cover `[0, 0.8]`, leaving `[0.8, 1.0]` unused).
    QuadHorn {
        base: [f32; 3],
        size: f32,
        height: f32,
        tilt_x_deg: f32,
        rotation_y_deg: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// UV sphere centered at `center`. A closed mesh with latitude sweeping
    /// `-90°..+90°` and
    /// longitude sweeping `0°..360°`, two triangles per `(lat × lon)` cell.
    ///
    /// `sides_lat` / `sides_lon` are segment counts (the original game's
    /// default `36°` arc gives `sides_lat = 5`, `sides_lon = 10`).
    /// `longitude_offset` is added to every vertex's longitude angle in
    /// radians — driving it over time slides the texture pattern around
    /// the sphere, as in the original game.
    ///
    /// `uv_repeat` tiles the texture: `[u_repeat, v_repeat]`. v runs from 0
    /// at the south pole to `v_repeat` at the north pole, u from 0 to
    /// `u_repeat` around the equator.
    ///
    /// `longitude_arc` is the longitude sweep in radians: `TAU` is a full
    /// sphere; `π` a half-dome split by a vertical plane (a half-sphere
    /// with a 180° arc span).
    ///
    /// The lower hemisphere can sit below the impact ground plane — depth
    /// testing against the ground geometry hides whatever portion is below.
    Sphere {
        center: [f32; 3],
        radius: f32,
        sides_lat: u32,
        sides_lon: u32,
        longitude_offset: f32,
        longitude_arc: f32,
        uv_repeat: [f32; 2],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Swept-quad cylinder / truncated cone matching the original game's
    /// cylinder primitive.
    ///
    /// Geometry is identical to [`Frustum`] (bottom ring of radius
    /// `bottom_size` at `base`, top ring of radius `top_size` at
    /// `base + (0, -height, 0)` in native RO coords), but UV mapping
    /// follows the original game: `u` advances by 0.25 per segment with
    /// wrap-to-0 at 1.0 (four texture tiles around the ring regardless of
    /// `sides`); `v = 1` at the bottom and `v = 0` at the top.
    /// `uv_scroll` adds an `[u, v]` offset each frame.
    ///
    /// `tilt_x_rad` pitches the geometry around the local X axis before
    /// `rotation_y_rad` rotates the tilted result around the world Y axis
    /// — an X-pitch followed by a Y-yaw, matching the original
    /// game's orientation. With `tilt_x_rad = -π/2` the
    /// cylinder lays on its side and `rotation_y_rad` aims its axis at a
    /// compass heading (used by Pierce). `rotation` is a per-segment angle
    /// offset around the cylinder's own axis (post-tilt, pre-yaw), letting
    /// callers spin the texture seam without moving the geometry.
    ///
    /// [`Frustum`]: EffectPrimitiveDraw::Frustum
    Cylinder {
        base: [f32; 3],
        bottom_size: f32,
        top_size: f32,
        height: f32,
        sides: u32,
        rotation: f32,
        tilt_x_rad: f32,
        rotation_y_rad: f32,
        uv_scroll: [f32; 2],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Connected ribbon ring driven by a `RadialEmitterSlot`'s `distance`
    /// / `rise_angle` / `height[]`.
    ///
    /// `segments + 1` positions are sampled around the ring at angles
    /// `θ_i = rot_start_rad + i / segments * full_arc_rad` for
    /// `i ∈ 0..=segments`. At each position the bottom vertex sits on the
    /// ring (`center.y`, radius `distance`) and the top vertex is offset
    /// by `heights[i] * height_scale` along the rise direction: pure
    /// upward for `rise_angle_rad = π/2`, pure radial-outward for
    /// `rise_angle_rad = 0`. Native RO `-Y = up`, so the renderer
    /// subtracts the upward component from Y.
    ///
    /// Each segment is one quad connecting position `i` to position `i+1`
    /// — a continuous strip (matches the original game's cast ribbon,
    /// drawing `(prev_bot, this_bot, this_top,
    /// prev_top)` per step). For a closed loop pass `full_arc_rad = TAU`
    /// (or `0.0` as a sentinel) and the last segment will wrap back to
    /// position 0. UV `u` advances 0..1 across the strip (one full
    /// texture tile per ring); UV `v` is 0 at the top, 1 at the bottom.
    ///
    /// `heights[i]` is read for `i ∈ 0..=segments` (with wrap-around at
    /// position `segments` for closed rings). Matches the original
    /// game's per-emitter height packing.
    RadialRing {
        center: [f32; 3],
        distance: f32,
        rise_angle_rad: f32,
        rot_start_rad: f32,
        full_arc_rad: f32,
        segments: u32,
        height_scale: f32,
        heights: [f32; crate::effect::radial_emitter::RADIAL_EMITTER_DIVISION],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Rotating textured ring around an actor (Lv99 aura).
    AuraQuad {
        center: [f32; 3],
        radius: f32,
        rotation: f32,
        vertical_offset: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Connected line strip (Grand Cross beams, Spear Boomerang trail).
    ///
    /// Rendered as a camera-facing ribbon: each path point becomes a pair of
    /// vertices offset by `±half_width` perpendicular to the path and the view
    /// direction. `uv_along` scales how fast the along-path texture coordinate
    /// accumulates with path length. By default that coordinate is V (texture
    /// height runs along the path); `u_along` swaps the axes so U runs along
    /// the path and V spans the ribbon width — for textures whose art (e.g. a
    /// lightning arc) is painted along the horizontal axis.
    ///
    /// `colors` optionally tints each path point individually (gradient /
    /// per-segment alpha falloff); when `None` the flat `color` applies to the
    /// whole strip.
    LineStrip {
        points: Vec<[f32; 3]>,
        uv_along: f32,
        u_along: bool,
        half_width: f32,
        texture: &'static str,
        color: [f32; 4],
        colors: Option<Vec<[f32; 4]>>,
        blend: BlendKind,
    },
    /// Bezier/Catmull-Rom curve, CPU-tessellated into a line strip
    /// (Soul Strike, Napalm Beat). `segments` is the number of line segments
    /// the curve is split into; the resulting polyline is rendered as a
    /// `half_width` ribbon exactly like [`Self::LineStrip`].
    Spline {
        control_points: Vec<[f32; 3]>,
        segments: u32,
        half_width: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Single animated SPR sprite billboard at a world position.
    ///
    /// Used by Custom effects (e.g. Hit's debris bursts) to emit one
    /// sprite-textured billboard per particle per frame. The Custom
    /// effect owns the per-frame position / motion-index / alpha logic;
    /// the renderer's job is to look the sprite up in the
    /// `EffectSpriteCache`, project the position to screen, and draw the
    /// selected motion's clip quad. This is the per-primitive
    /// counterpart to the `SpriteEffectEmitter::Smoke3D` path which is
    /// driven from spec-level `EffectSpec::SprBurst` entries — the
    /// difference is who chooses the particle's position: an emitter
    /// (Smoke3D) or the Custom effect itself (SpriteParticle).
    ///
    /// `sprite_path` is the full GRF lookup path **without** extension,
    /// e.g. `"data/sprite/이팩트/particle1"`.
    /// `action_index` selects which ACT action to render (the renderer
    /// applies `% action_count`). Most callers pass `0`; directional
    /// sprites (e.g. the wink emote's four diagonal fly-off actions)
    /// pick it from a camera-relative angle. `motion_index` then selects
    /// the motion within that action; the renderer applies
    /// `% motion_count` so callers can hand it a raw frame counter.
    /// `size_scale` is a per-particle multiplier on top of the
    /// renderer's per-pixel-unit scale.
    /// `color` is multiplied with the sprite's own ARGB; setting alpha
    /// drives the per-particle fade-out.
    SpriteParticle {
        sprite_path: &'static str,
        position: [f32; 3],
        action_index: usize,
        motion_index: usize,
        size_scale: f32,
        color: [f32; 4],
        blend: BlendKind,
        /// World-space target the sprite should point toward. `None` means
        /// no rotation. The renderer projects both `position` and this
        /// target to screen and rotates the sprite accordingly.
        aim_target: Option<[f32; 3]>,
        /// When `true`, the sprite draws without the depth test,
        /// so it renders over the floor instead of
        /// being depth-occluded by ground geometry it sits at or below. Routes
        /// the draw to the `AlphaNoDepth` / `AdditiveNoDepth` bucket.
        no_depth: bool,
    },
    /// Textured quad anchored in world space by four explicit corner
    /// points (not camera-facing).
    ///
    /// Used by effects whose silhouette is an arbitrary 3D rectangle —
    /// most notably the Bard/Dancer Bottom_Vertical songs, which paint
    /// thin vertical curtain strips anchored at two ground points and
    /// extending straight up to `max_height` (the original game's
    /// vertical curtain primitive).
    ///
    /// Corners are listed CCW when viewed from the "front" face; the
    /// renderer disables back-face culling so both sides are visible
    /// (matches the original game which always rendered these as
    /// two-sided quads). UVs map per-corner in the same order.
    WorldQuad {
        corners: [[f32; 3]; 4],
        uv: [[f32; 2]; 4],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
        /// When true the quad ignores the depth buffer:
        /// it draws over coincident terrain instead of being occluded by the
        /// ground it sits at or below (e.g. the Chemical Protection spokes
        /// radiating through the floor). Defaults to `false` — normal quads
        /// stay depth-tested.
        no_depth: bool,
    },
    /// Textured quad fixed in world space defined by a centre + half-extents
    /// + a plane orientation (not camera-facing). Convenience over `WorldQuad`
    /// for the common single-quad / cross-quad cases where the
    /// caller doesn't want to compute four corners by hand.
    ///
    /// `size` is the **half-extents** along the quad's two local axes (so
    /// world width / height = `2 * size`), matching the original game's
    /// textured-quad convention where vertices sit at `(±width, ±height, 0)`
    /// in the local frame.
    ///
    /// `plane` selects orientation:
    /// * [`QuadPlane::Horizontal`] — quad lies flat on world XZ at
    ///   `y = center.y` (rolled flat: Party, Detecting,
    ///   Yufitelhit ground splash).
    /// * [`QuadPlane::VerticalYaw`] — quad stands vertically with its base
    ///   axis spanning world X (width) and its up axis spanning world Y
    ///   (height); the whole quad is then yawed by the given angle (radians)
    ///   around the world Y axis. yaw=0 faces +Z. Used for upright single
    ///   quads (Toprank, Curseattack) and for both legs of
    ///   a crossed-quad pair (Blitzbeat — `yaw` and `yaw + π/2`).
    ///
    /// Per-corner UVs in the same order as `WorldQuad` (CCW from the front
    /// face).
    Texture3D {
        center: [f32; 3],
        size: [f32; 2],
        plane: QuadPlane,
        uv: [[f32; 2]; 4],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Screen-space textured quad, drawn in the effect pass (over the 3D
    /// scene, under the UI). Camera-independent: `corners` are already in NDC
    /// clip space (x/y in `[-1, 1]`, +y up) and the renderer passes them
    /// straight through, sampling `texture` with `uvs`.
    ///
    /// Used by the status-overlay family (Blind / Poison / Devil / Bleeding /
    /// CrystalBlue): the original game draws these as camera-locked screen
    /// overlays — a centred vignette (Blind/Devil, four mirrored quads), a
    /// full-viewport tint wash (Poison/CrystalBlue/Bleeding) or claw slashes
    /// (Bleeding). `color.rgb` is the tint, `color.a` the opacity.
    ScreenQuad {
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
        /// NDC corners, ordered to match `uvs`; triangulated `[0,1,2,0,2,3]`.
        corners: [[f32; 2]; 4],
        uvs: [[f32; 2]; 4],
    },
    /// Camera-locked, depth-disabled triangle mesh in NDC clip space with a
    /// colour per vertex — the same overlay pass as [`Self::ScreenQuad`] but
    /// for arbitrary geometry (the circular concentric Blind vignette). The
    /// texture is sampled at its centre, so a solid texture (`white02.bmp`)
    /// lets the per-vertex colour/alpha drive the result.
    ScreenMesh {
        texture: &'static str,
        blend: BlendKind,
        /// `(ndc_position, rgba)` per vertex.
        vertices: Vec<([f32; 2], [f32; 4])>,
        indices: Vec<u32>,
    },
}

/// Orientation selector for [`EffectPrimitiveDraw::Texture3D`].
///
/// In every variant `size = [width_half, height_half]`:
/// * `width_half` is along the quad's primary in-plane axis,
/// * `height_half` is along the quad's secondary in-plane axis.
///
/// The variants differ only in which world axes those map to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuadPlane {
    /// Flat on world XZ at `center.y`; width spans world X, height spans
    /// world Z (axis-aligned ground splash). Rolled flat
    /// with no extra yaw.
    Horizontal,
    /// Flat on world XZ at `center.y`; width spans the **yaw direction**
    /// (forward), height spans the perpendicular sideways direction
    /// (yaw + 90°). Used by the horizontal leg of crossed-quad
    /// needle effects (Blitzbeat) where the needle's long axis is the
    /// caster's facing direction.
    HorizontalYaw(f32),
    /// Standing vertically; width spans the world XZ direction at angle
    /// `yaw` (radians from +X around the Y axis), height spans world Y.
    /// Native RO `-Y = up`, so "top" sits at `center.y - height_half`.
    VerticalYaw(f32),
}

impl QuadPlane {
    /// Compute the four world-space corners of a [`Texture3D`] quad in the
    /// same CCW order as [`WorldQuad`] (front face viewer).
    ///
    /// [`Texture3D`]: EffectPrimitiveDraw::Texture3D
    /// [`WorldQuad`]: EffectPrimitiveDraw::WorldQuad
    pub fn corners(self, center: [f32; 3], size: [f32; 2]) -> [[f32; 3]; 4] {
        let [cx, cy, cz] = center;
        let [hx, hy] = size;
        match self {
            QuadPlane::Horizontal => [
                [cx - hx, cy, cz - hy],
                [cx + hx, cy, cz - hy],
                [cx + hx, cy, cz + hy],
                [cx - hx, cy, cz + hy],
            ],
            QuadPlane::HorizontalYaw(yaw) => {
                let (s, c) = yaw.sin_cos();
                // forward = (c, 0, s); right = (-s, 0, c)
                let fx = hx * c;
                let fz = hx * s;
                let rx = -hy * s;
                let rz = hy * c;
                [
                    [cx - fx - rx, cy, cz - fz - rz],
                    [cx + fx - rx, cy, cz + fz - rz],
                    [cx + fx + rx, cy, cz + fz + rz],
                    [cx - fx + rx, cy, cz - fz + rz],
                ]
            }
            QuadPlane::VerticalYaw(yaw) => {
                let (s, c) = yaw.sin_cos();
                let dx = hx * c;
                let dz = hx * s;
                let top = cy - hy;
                let bot = cy + hy;
                [
                    [cx - dx, bot, cz - dz],
                    [cx + dx, bot, cz + dz],
                    [cx + dx, top, cz + dz],
                    [cx - dx, top, cz - dz],
                ]
            }
        }
    }
}

/// Collected primitive draws for a single frame. Effects push into this;
/// the effect render pass drains it.
///
/// [`primitives`](Self::primitives) draw *after* the entity sprite pass (on top
/// of characters — the common case). [`behind`](Self::behind) draws *before* it,
/// so the entity occludes those primitives (e.g. an effect that should appear
/// to radiate from behind the caster). Both lists otherwise share the same
/// bucket-then-depth sorting inside the unified dispatch.
#[derive(Default)]
pub struct EffectDrawList {
    pub primitives: Vec<EffectPrimitiveDraw>,
    pub behind: Vec<EffectPrimitiveDraw>,
}

impl EffectDrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, prim: EffectPrimitiveDraw) {
        self.primitives.push(prim);
    }

    /// Push a primitive that renders behind the entity (occluded by the
    /// character sprite) — see [`behind`](Self::behind).
    pub fn push_behind(&mut self, prim: EffectPrimitiveDraw) {
        self.behind.push(prim);
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
        self.behind.clear();
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty() && self.behind.is_empty()
    }

    /// The behind-entity primitives as a standalone list, so the renderer can
    /// run them through the same `prepare_*_records` helpers (which read
    /// [`primitives`](Self::primitives)) for the pre-sprite dispatch pass.
    pub fn behind_as_list(&self) -> EffectDrawList {
        EffectDrawList {
            primitives: self.behind.clone(),
            behind: Vec::new(),
        }
    }
}
