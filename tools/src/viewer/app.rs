//! Unified viewer host: combines map rendering (rsw_viewer), character
//! composition (sprite_viewer), and effect playback (effect_viewer) into a
//! single binary. The character stands on the map's GAT center; effects
//! spawn at that world position so the user can preview skills on a body
//! with the real ground beneath.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use models::enums::EnumWithNumberValue;
use models::enums::EnumWithStringValue;
use models::enums::effect_id::EffectId;
use ragnarok_formats::act::{MotionType, SpriteActionType, SpriteAnimationState};
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::effect::spec::EffectAnchor;
use ragnarok_game::effect::{
    EffectQueue, EffectSpec, body_attached, effect_spec, effect_texture_paths, is_trail_effect,
    str_aliases,
};
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::map_loader::{self, MapData};
use ragnarok_game::sprite_loader as game_sprite_loader;
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_renderer::effect::holder::AfterimageSnapshot;
use ragnarok_renderer::effect::{
    EffectFrameInputs, EffectHolder, EffectUpdateCtx, StrEffectCache, compose_effect_frame,
};
use ragnarok_renderer::EffectSpriteCache;
use ragnarok_renderer::sprite::{EntitySprite, build_entity_sprite, upload_sprite_textures};
use ragnarok_renderer::sprite_projection::{cell_world_pos, project_entity_screen};
use ragnarok_renderer::{BackgroundMode, Renderer, UiDrawCall, block_on};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use crate::sprite_viewer::browser::SpriteBrowser;
use crate::viewer::controls::{ViewerAction, map_key};
use crate::viewer::overlay::{self, StatusLine};

pub struct Args {
    pub grf_path: String,
    pub map_name: String,
    pub effect_id: Option<u16>,
    /// Character cell position; `None` falls back to walkable GAT center.
    pub cell: Option<(i32, i32)>,
    /// Character facing direction (0-7); `None` keeps the default (0).
    pub direction: Option<u8>,
}

/// Build the list of effects N/P cycles through. Mirrors
/// `tools/effect-viewer-hot`'s `build_filtered(Filter::All)`: every valid
/// `EffectId` whose spec is non-`Noop`, in numeric order.
fn build_effect_list() -> Vec<EffectId> {
    (0..=2027usize)
        .filter_map(|v| EffectId::try_from_value(v).ok())
        .filter(|id| match effect_spec(*id) {
            Some(EffectSpec::Noop) | None => false,
            _ => true,
        })
        .collect()
}

pub struct App {
    args: Args,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    grf: Option<Arc<GrfArchive>>,
    map_data: Option<MapData>,

    // Character
    composite_job: u16,
    composite_sex: u8,
    composite_head: u16,
    weapon_view_id: u16,
    headgear_top_id: u16,
    shield_view_id: u16,
    accessory_table: AccessoryTable,
    entity_sprite: Option<EntitySprite>,
    animation: SpriteAnimationState,
    character_cell: (f32, f32),
    paused: bool,

    // Effects
    effect_holder: EffectHolder,
    effect_queue: EffectQueue,
    str_effects: StrEffectCache,
    /// Per-frame SPR billboards for Custom effects that emit
    /// `SpriteParticle` primitives (Sight, Ruwach, Exit, Hit). Lazily
    /// loaded on first spawn that needs a given path; subsequent spawns
    /// reuse the cached sprite.
    effect_sprites: EffectSpriteCache,
    attempted_spr_files: std::collections::HashSet<String>,
    effect_list: Vec<EffectId>,
    current_effect_idx: usize,
    current_effect_id: Option<EffectId>,
    current_effect_label: String,

    // Effect browser (Tab to open)
    browser: Option<SpriteBrowser>,
    browser_lookup: HashMap<String, EffectId>,
    ctrl_pressed: bool,

    // Trail target placement
    trail_target_override: Option<[f32; 3]>,
    placing_target: bool,

    // Input
    mouse_pos: (f32, f32),
    last_mouse: (f32, f32),
    mouse_down_left: bool,
    mouse_down_right: bool,

    last_frame: Instant,
    /// Whether `BackgroundMode::Clear` should clear to black this frame.
    /// Toggled by the B-cycle: blue clear -> black clear -> RswMap.
    clear_is_black: bool,
}

impl App {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            window: None,
            renderer: None,
            grf: None,
            map_data: None,
            composite_job: 14,
            composite_sex: 1,
            composite_head: 1,
            weapon_view_id: 0,
            headgear_top_id: 0,
            shield_view_id: 0,
            accessory_table: AccessoryTable::empty(),
            entity_sprite: None,
            animation: SpriteAnimationState::new(0),
            character_cell: (0.0, 0.0),
            paused: false,
            effect_holder: EffectHolder::new(),
            effect_queue: EffectQueue::new(),
            str_effects: StrEffectCache::new(),
            effect_sprites: EffectSpriteCache::new(),
            attempted_spr_files: std::collections::HashSet::new(),
            effect_list: build_effect_list(),
            current_effect_idx: 0,
            current_effect_id: None,
            current_effect_label: "(none)".to_string(),
            browser: None,
            browser_lookup: HashMap::new(),
            ctrl_pressed: false,
            trail_target_override: None,
            placing_target: false,
            mouse_pos: (0.0, 0.0),
            last_mouse: (0.0, 0.0),
            mouse_down_left: false,
            mouse_down_right: false,
            last_frame: Instant::now(),
            clear_is_black: false,
        }
    }

    fn load_world(&mut self) {
        let grf = match GrfArchive::open(Path::new(&self.args.grf_path)) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to open GRF '{}': {e}", self.args.grf_path);
                return;
            }
        };

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        renderer.try_load_grf_font(&grf);
        self.accessory_table = AccessoryTable::load_from_grf(&grf);

        let map_name = self.args.map_name.clone();
        let Some(map_data) = map_loader::load_map_data(&grf, &map_name) else {
            tracing::error!("Failed to load map '{map_name}'");
            return;
        };
        renderer.load_map(&map_data.gnd, &map_data.rsw, &grf, map_data.fog);
        renderer.preload_effect_textures(&effect_texture_paths(), &grf);

        self.character_cell = self
            .args
            .cell
            .map(|(x, y)| (x as f32, y as f32))
            .unwrap_or_else(|| pick_character_cell(map_data.gat.as_ref()));
        let anchor = compute_world_anchor(self.character_cell, &map_data);
        renderer.camera.target = glam::Vec3::from(anchor);
        renderer.camera.distance = 120.0;
        renderer.camera.fov_y = 30_f32.to_radians();
        renderer.camera.yaw = 0.0;

        self.map_data = Some(map_data);
        self.grf = Some(Arc::new(grf));

        self.reload_character();
        if let Some(window) = &self.window {
            window.set_title(&format!("Viewer - {}", map_name));
        }

        if let Some(id_num) = self.args.effect_id {
            if let Ok(id) = EffectId::try_from_value(id_num as usize) {
                self.spawn_effect_on_character(id);
            } else {
                tracing::warn!("Unknown effect id: {id_num}");
            }
        }
    }

    fn world_anchor(&self, map: &MapData) -> [f32; 3] {
        compute_world_anchor(self.character_cell, map)
    }

    fn initial_animation(&self) -> SpriteAnimationState {
        let mut anim = SpriteAnimationState::new(0);
        if let Some(dir) = self.args.direction {
            anim.set_direction(dir & 0x07);
        } else {
            anim.set_action(1, MotionType::Static);
            anim.set_direction(0x04);
        }
        anim
    }

    fn reload_character(&mut self) {
        let initial_anim = self.initial_animation();
        let (Some(renderer), Some(grf)) = (&mut self.renderer, &self.grf) else {
            return;
        };
        let weapon_type = weapon_view_id_to_type(self.weapon_view_id);
        let data = match game_sprite_loader::load_player_sprite_data(
            grf.as_ref(),
            &self.accessory_table,
            self.composite_job,
            self.composite_sex,
            self.composite_head,
            0,
            0,
            weapon_type,
            self.headgear_top_id,
            0,
            0,
            self.shield_view_id,
        ) {
            Some(d) => d,
            None => {
                tracing::warn!(
                    "Failed to load body sprite job={} sex={}",
                    self.composite_job,
                    self.composite_sex
                );
                return;
            }
        };
        self.animation = initial_anim;
        self.entity_sprite = Some(build_entity_sprite(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
            data.body,
            data.head,
            data.weapon,
            data.headgear_top,
            data.headgear_mid,
            data.headgear_bottom,
            data.shield,
            None,
        ));
    }

    fn reload_weapon(&mut self) {
        let (Some(renderer), Some(grf), Some(entity)) =
            (&mut self.renderer, &self.grf, &mut self.entity_sprite)
        else {
            return;
        };
        let (textures, act) = if let Some(wt) = weapon_view_id_to_type(self.weapon_view_id) {
            if let Some(wd) = game_sprite_loader::load_weapon_sprite(
                grf.as_ref(),
                self.composite_job,
                self.composite_sex,
                wt,
            ) {
                let tex = upload_sprite_textures(
                    &wd.images,
                    wd.indexed_count,
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                );
                (Some(tex), Some(wd.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        entity.weapon_textures = textures;
        entity.weapon_act = act;
    }

    fn reload_headgear(&mut self) {
        let (Some(renderer), Some(grf), Some(entity)) =
            (&mut self.renderer, &self.grf, &mut self.entity_sprite)
        else {
            return;
        };
        let (textures, act) = if self.headgear_top_id > 0 {
            let suffix = self
                .accessory_table
                .get_suffix(self.headgear_top_id)
                .unwrap_or("")
                .to_string();
            if let Some(data) =
                game_sprite_loader::load_headgear_sprite(grf.as_ref(), &suffix, self.composite_sex)
            {
                let tex = upload_sprite_textures(
                    &data.images,
                    data.indexed_count,
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                );
                (Some(tex), Some(data.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        entity.headgear_top_textures = textures;
        entity.headgear_top_act = act;
    }

    fn reload_shield(&mut self) {
        let (Some(renderer), Some(grf), Some(entity)) =
            (&mut self.renderer, &self.grf, &mut self.entity_sprite)
        else {
            return;
        };
        let (textures, act) = if self.shield_view_id > 0 {
            if let Some(data) = game_sprite_loader::load_shield_sprite(
                grf.as_ref(),
                self.shield_view_id,
                self.composite_job,
                self.composite_sex,
            ) {
                let tex = upload_sprite_textures(
                    &data.images,
                    data.indexed_count,
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                );
                (Some(tex), Some(data.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        entity.shield_textures = textures;
        entity.shield_act = act;
    }

    fn spawn_effect_on_character(&mut self, id: EffectId) {
        // Lazy STR load — the holder won't render until the bind groups exist.
        self.ensure_str_loaded(id);
        self.ensure_spr_loaded_for(id);
        let pos = match (&self.map_data, true) {
            (Some(map), _) => self.world_anchor(map),
            _ => [0.0, 0.0, 0.0],
        };
        self.effect_holder.clear();
        if body_attached(id) {
            // Body shake / tint effects attach to the previewed actor so the
            // character pass can apply them (mirrors in-game `spawn_on`).
            self.effect_queue.spawn_on(id, VIEWER_ACTOR_ID);
        } else if is_trail_effect(id) {
            let to = self
                .trail_target_override
                .unwrap_or([pos[0], pos[1], pos[2] + 22.0]);
            self.effect_queue.spawn_trail(id, pos, to);
        } else {
            self.effect_queue.spawn_at(id, pos);
        }
        self.current_effect_id = Some(id);
        if let Some(idx) = self.effect_list.iter().position(|x| *x == id) {
            self.current_effect_idx = idx;
        }
        self.current_effect_label = format_effect_label(id);
        tracing::info!(
            "Spawning effect {} ({:?}) at {:?}",
            id.value(),
            id,
            pos
        );
    }

    fn open_browser(&mut self) {
        self.browser_lookup.clear();
        let mut items: Vec<String> = Vec::new();
        for raw in 0..=2027u16 {
            let Ok(id) = EffectId::try_from_value(raw as usize) else {
                continue;
            };
            let label = format!("{:?} ({}) [{}]", id, id.as_str(), raw);
            self.browser_lookup.insert(label.clone(), id);
            items.push(label);
        }
        let mut browser = SpriteBrowser::new(items, "effects");
        if let Some(renderer) = &self.renderer {
            let h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            browser.update_visible_rows(h);
        }
        self.browser = Some(browser);
    }

    fn browser_is_open(&self) -> bool {
        self.browser.as_ref().is_some_and(|b| b.open)
    }

    fn handle_browser_key(&mut self, key: &Key) {
        let ctrl = self.ctrl_pressed;
        let Some(browser) = &mut self.browser else {
            return;
        };
        match key.as_ref() {
            Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                browser.open = false;
            }
            Key::Named(NamedKey::Enter) => {
                self.handle_browser_select();
            }
            Key::Named(NamedKey::ArrowUp) => browser.handle_up(),
            Key::Named(NamedKey::ArrowDown) => browser.handle_down(),
            Key::Named(NamedKey::PageUp) => browser.handle_page_up(),
            Key::Named(NamedKey::PageDown) => browser.handle_page_down(),
            Key::Named(NamedKey::Backspace) => browser.handle_backspace(),
            Key::Character(ch) => {
                if ctrl && ch == "v" {
                    if let Ok(mut clipboard) = arboard::Clipboard::new()
                        && let Ok(text) = clipboard.get_text()
                    {
                        browser.handle_paste(&text);
                    }
                    return;
                }
                for c in ch.chars() {
                    if !c.is_control() {
                        browser.handle_char(c);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_browser_select(&mut self) {
        let Some(browser) = &mut self.browser else {
            return;
        };
        let Some(selected) = browser.selected_item().map(|s| s.to_string()) else {
            return;
        };
        browser.open = false;
        if let Some(&id) = self.browser_lookup.get(&selected) {
            self.spawn_effect_on_character(id);
        }
    }

    /// Lazy-load SPR sprites for `EffectSpec::Spr`/`SprBurst` (driven by
    /// the spec) and for `EffectSpec::Custom` (the aggregated module
    /// `SPRITES` constants, since the holder doesn't know which sprites
    /// the custom effect will emit until it runs). Mirrors
    /// `effect_viewer::App::ensure_spr_loaded_for`.
    fn ensure_spr_loaded_for(&mut self, id: EffectId) {
        let mut sprites: Vec<&'static str> = Vec::new();
        match effect_spec(id) {
            Some(EffectSpec::Spr { sprite, .. }) => sprites.push(sprite),
            Some(EffectSpec::SprBurst { sprite, .. }) => sprites.push(sprite),
            Some(EffectSpec::Custom { .. }) => {
                sprites.extend(ragnarok_game::effect::custom_effect_sprite_paths());
            }
            _ => return,
        }
        for sprite in sprites {
            if self.attempted_spr_files.contains(sprite) {
                continue;
            }
            self.attempted_spr_files.insert(sprite.to_string());
            let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) else {
                return;
            };
            self.effect_sprites.load(
                sprite,
                grf,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
        }
    }

    fn ensure_str_loaded(&mut self, id: EffectId) {
        let file: &'static str = match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => file,
            Some(EffectSpec::Custom { .. }) => {
                let probe = ragnarok_game::effect::factory::make_effect(
                    id,
                    EffectAnchor::Point([0.0, 0.0, 0.0]),
                    None,
                );
                let Some(probe) = probe else { return };
                let Some(overlay) = probe.str_overlay() else { return };
                overlay
            }
            _ => return,
        };
        let (Some(renderer), Some(grf)) = (&mut self.renderer, &self.grf) else {
            return;
        };
        if self.str_effects.get(file).is_some() {
            return;
        }
        let aliases = str_aliases(id);
        let fallbacks: &[&str] = if aliases.first().copied() == Some(file) {
            &aliases[1..]
        } else {
            aliases
        };
        self.str_effects.load(
            file,
            fallbacks,
            grf.as_ref(),
            &mut renderer.texture_cache,
            &renderer.device.device,
            &renderer.device.queue,
        );
    }

    /// Cast a ray from the cursor into the world, intersect the camera's
    /// target plane, and teleport the character to the GAT cell under that
    /// hit. No path/animation — the sprite snaps to the new cell so
    /// effects spawn at the new world position. Mirrors `rsw_viewer`'s
    /// click-to-move but moves the character instead of the camera.
    fn try_move_character_to_cursor(&mut self) {
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let Some(map) = self.map_data.as_ref() else {
            return;
        };
        let Some(coords) = map.coordinates.as_ref() else {
            return;
        };

        let (mx, my) = self.mouse_pos;
        let (origin, dir) = renderer.camera.screen_to_ray(
            mx,
            my,
            renderer.device.surface_config.width as f32,
            renderer.device.surface_config.height as f32,
        );
        if dir.y.abs() < 1e-6 {
            return;
        }
        let mut plane_y = renderer.camera.target.y;
        let mut hit_cell: Option<(i32, i32)> = None;
        for _ in 0..5 {
            let t = (plane_y - origin.y) / dir.y;
            if t < 0.0 {
                return;
            }
            let hit = origin + dir * t;
            let (cx, cy) = coords.world_to_cell(hit.x, hit.z);
            if !coords.is_valid_cell(cx, cy) {
                return;
            }
            if hit_cell == Some((cx, cy)) {
                break;
            }
            hit_cell = Some((cx, cy));
            plane_y = map
                .gat
                .as_ref()
                .map_or(0.0, |g| g.get_height(cx as f32 + 0.5, cy as f32 + 0.5));
        }
        let Some((cx, cy)) = hit_cell else { return };
        if let Some(gat) = map.gat.as_ref() {
            if !gat.is_walkable(cx, cy) {
                return;
            }
        }

        self.character_cell = (cx as f32, cy as f32);
        let anchor = compute_world_anchor(self.character_cell, map);
        if let Some(renderer) = &mut self.renderer {
            renderer.camera.target = glam::Vec3::from(anchor);
        }
        // Respawn the current effect at the new position so the user sees
        // it follow the body instead of staying behind on the old cell.
        if let Some(id) = self.current_effect_id {
            if !self.effect_holder.is_empty() {
                self.spawn_effect_on_character(id);
            }
        }
    }

    fn try_place_target_at_cursor(&mut self) {
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
        let Some(map) = self.map_data.as_ref() else {
            return;
        };
        let Some(coords) = map.coordinates.as_ref() else {
            return;
        };

        let (mx, my) = self.mouse_pos;
        let (origin, dir) = renderer.camera.screen_to_ray(
            mx,
            my,
            renderer.device.surface_config.width as f32,
            renderer.device.surface_config.height as f32,
        );
        if dir.y.abs() < 1e-6 {
            return;
        }
        let mut plane_y = renderer.camera.target.y;
        let mut hit_cell: Option<(i32, i32)> = None;
        for _ in 0..5 {
            let t = (plane_y - origin.y) / dir.y;
            if t < 0.0 {
                return;
            }
            let hit = origin + dir * t;
            let (cx, cy) = coords.world_to_cell(hit.x, hit.z);
            if !coords.is_valid_cell(cx, cy) {
                return;
            }
            if hit_cell == Some((cx, cy)) {
                break;
            }
            hit_cell = Some((cx, cy));
            plane_y = map
                .gat
                .as_ref()
                .map_or(0.0, |g| g.get_height(cx as f32 + 0.5, cy as f32 + 0.5));
        }
        let Some((cx, cy)) = hit_cell else { return };
        let target_pos = compute_world_anchor((cx as f32, cy as f32), map);
        self.trail_target_override = Some(target_pos);
        self.placing_target = false;
        if let Some(id) = self.current_effect_id {
            self.spawn_effect_on_character(id);
        }
    }

    fn handle_action(&mut self, action: ViewerAction) {
        match action {
            ViewerAction::CycleBackground => {
                if let Some(renderer) = &mut self.renderer {
                    // Four-phase cycle, with Clear split into blue + black:
                    //   RswMap -> GroundProxy -> Clear(blue) -> Clear(black) -> RswMap
                    let (next_mode, next_black) =
                        match (renderer.background_mode, self.clear_is_black) {
                            (BackgroundMode::RswMap, _) => (BackgroundMode::GroundProxy, false),
                            (BackgroundMode::GroundProxy, _) => (BackgroundMode::Clear, false),
                            (BackgroundMode::Clear, false) => (BackgroundMode::Clear, true),
                            (BackgroundMode::Clear, true) => (BackgroundMode::RswMap, false),
                        };
                    if matches!(next_mode, BackgroundMode::GroundProxy) {
                        renderer.enable_ground_proxy();
                    }
                    if matches!(next_mode, BackgroundMode::Clear) {
                        renderer.clear_color = if next_black {
                            wgpu::Color::BLACK
                        } else {
                            wgpu::Color {
                                r: 0.392,
                                g: 0.584,
                                b: 0.929,
                                a: 1.0,
                            }
                        };
                    }
                    self.clear_is_black = next_black;
                    renderer.set_background_mode(next_mode);
                    tracing::info!(
                        "Background: {:?}{}",
                        next_mode,
                        if matches!(next_mode, BackgroundMode::Clear) {
                            if next_black { " (black)" } else { " (blue)" }
                        } else {
                            ""
                        }
                    );
                }
            }
            ViewerAction::TogglePause => {
                self.paused = !self.paused;
            }
            ViewerAction::NextAction => {
                if let Some(entity) = &self.entity_sprite {
                    let next = self.animation.action() + 1;
                    self.animation
                        .set_action_clamped(next, MotionType::Loop, &entity.body_act);
                }
            }
            ViewerAction::PrevAction => {
                if let Some(entity) = &self.entity_sprite {
                    let prev = if self.animation.action() == 0 {
                        entity.body_act.actions.len() - 1
                    } else {
                        self.animation.action() - 1
                    };
                    self.animation
                        .set_action_clamped(prev, MotionType::Loop, &entity.body_act);
                }
            }
            ViewerAction::NextDirection => {
                let dir = ((self.animation.direction() + 1) % 16) as u8;
                self.animation.set_direction(dir);
                self.animation.reset_motion();
            }
            ViewerAction::PrevDirection => {
                let dir = if self.animation.direction() == 0 {
                    7u8
                } else {
                    (self.animation.direction() - 1) as u8
                };
                self.animation.set_direction(dir);
                self.animation.reset_motion();
            }
            ViewerAction::NextWeapon => {
                self.weapon_view_id = if self.weapon_view_id >= 17 {
                    0
                } else {
                    self.weapon_view_id + 1
                };
                self.reload_weapon();
            }
            ViewerAction::PrevWeapon => {
                self.weapon_view_id = if self.weapon_view_id == 0 {
                    17
                } else {
                    self.weapon_view_id - 1
                };
                self.reload_weapon();
            }
            ViewerAction::ToggleSex => {
                self.composite_sex = if self.composite_sex == 0 { 1 } else { 0 };
                self.reload_character();
            }
            ViewerAction::NextHead => {
                self.composite_head = if self.composite_head >= 30 {
                    1
                } else {
                    self.composite_head + 1
                };
                self.reload_character();
            }
            ViewerAction::PrevHead => {
                self.composite_head = if self.composite_head <= 1 {
                    30
                } else {
                    self.composite_head - 1
                };
                self.reload_character();
            }
            ViewerAction::NextHeadgear => {
                self.headgear_top_id = self.accessory_table.next_id(self.headgear_top_id);
                self.reload_headgear();
            }
            ViewerAction::PrevHeadgear => {
                self.headgear_top_id = self.accessory_table.prev_id(self.headgear_top_id);
                self.reload_headgear();
            }
            ViewerAction::NextShield => {
                self.shield_view_id = if self.shield_view_id >= 4 {
                    0
                } else {
                    self.shield_view_id + 1
                };
                self.reload_shield();
            }
            ViewerAction::PrevShield => {
                self.shield_view_id = if self.shield_view_id == 0 {
                    4
                } else {
                    self.shield_view_id - 1
                };
                self.reload_shield();
            }
            ViewerAction::NextEffect => {
                let n = self.effect_list.len();
                if n > 0 {
                    self.current_effect_idx = (self.current_effect_idx + 1) % n;
                    self.spawn_effect_on_character(self.effect_list[self.current_effect_idx]);
                }
            }
            ViewerAction::PrevEffect => {
                let n = self.effect_list.len();
                if n > 0 {
                    self.current_effect_idx = if self.current_effect_idx == 0 {
                        n - 1
                    } else {
                        self.current_effect_idx - 1
                    };
                    self.spawn_effect_on_character(self.effect_list[self.current_effect_idx]);
                }
            }
            ViewerAction::PlayEffect | ViewerAction::ReplayEffect => {
                let id = self
                    .current_effect_id
                    .or_else(|| self.effect_list.get(self.current_effect_idx).copied());
                if let Some(id) = id {
                    self.spawn_effect_on_character(id);
                }
            }
            ViewerAction::ResetCamera => {
                let anchor = self
                    .map_data
                    .as_ref()
                    .map(|m| compute_world_anchor(self.character_cell, m));
                if let (Some(anchor), Some(renderer)) = (anchor, &mut self.renderer) {
                    renderer.camera.target = glam::Vec3::from(anchor);
                    renderer.camera.yaw = 0.0;
                    renderer.camera.pitch = 55_f32.to_radians();
                    renderer.camera.distance = 120.0;
                }
            }
            ViewerAction::ZoomIn => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.camera.distance = (renderer.camera.distance - 20.0).max(20.0);
                }
            }
            ViewerAction::ZoomOut => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.camera.distance = (renderer.camera.distance + 20.0).min(1500.0);
                }
            }
            ViewerAction::ToggleTargetMode => {
                self.placing_target = !self.placing_target;
            }
            ViewerAction::ClearTarget => {
                self.trail_target_override = None;
                self.placing_target = false;
                if let Some(id) = self.current_effect_id {
                    self.spawn_effect_on_character(id);
                }
            }
        }
    }

    fn render_frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let sim_dt = if self.paused { 0.0 } else { dt };

        self.effect_holder.drain_queue(&mut self.effect_queue);
        self.effect_holder.update(
            &EffectUpdateCtx { delta: sim_dt, camera_target: None, caster_yaw: None },
            &|_| None,
        );
        let body_shake = self.effect_holder.body_shake_for_entity(VIEWER_ACTOR_ID);
        let body_tint = self.effect_holder.body_tint_for_entity(VIEWER_ACTOR_ID);
        // Caster-attached effects (buff STR overlays) resolve to the previewed
        // actor's world anchor.
        let actor_pos = self
            .map_data
            .as_ref()
            .map(|m| compute_world_anchor(self.character_cell, m))
            .unwrap_or([0.0, 0.0, 0.0]);

        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
        let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;

        let zoom = self
            .map_data
            .as_ref()
            .and_then(|m| m.coordinates.as_ref())
            .map_or(10.0, |c| c.zoom());
        let frame = compose_effect_frame(&EffectFrameInputs {
            effect_holder: &self.effect_holder,
            effect_sprites: &self.effect_sprites,
            str_effects: &self.str_effects,
            camera: &renderer.camera,
            screen_w,
            screen_h,
            zoom,
            elapsed: 0.0,
            resolve_entity: &|id| (id == VIEWER_ACTOR_ID).then_some(actor_pos),
            extra_spr_emitters: &[],
            extra_str_emitters: &[],
            extra_sprite_particles: &[],
        });
        let effect_batches = frame.effect_batches;
        let effect_draws = frame.effect_draws;
        let sprite_particle_records = frame.sprite_particle_records;

        let sprite_batches: Vec<ragnarok_renderer::sprite::SpriteBatch<'_>> =
            match (&self.entity_sprite, &self.map_data) {
                (Some(entity), Some(map)) => {
                    // Trail only while the walk action plays (the viewer's
                    // stand-in for the in-game `Moving` state).
                    let emitting =
                        self.animation.action() == SpriteActionType::Walk as usize;
                    build_character_batches(
                        entity,
                        map,
                        self.character_cell,
                        &self.animation,
                        &renderer.camera,
                        screen_w,
                        screen_h,
                        body_shake,
                        body_tint,
                        &mut self.effect_holder,
                        VIEWER_ACTOR_ID,
                        emitting,
                    )
                }
                _ => Vec::new(),
            };

        let _ = sim_dt;
        let mut ui_calls: Vec<UiDrawCall> = Vec::new();
        let status = StatusLine {
            map_name: self.args.map_name.as_str(),
            effect_label: self.current_effect_label.as_str(),
            paused: self.paused,
            background: renderer.background_mode,
            clear_is_black: self.clear_is_black,
            target_mode: self.placing_target,
            has_target: self.trail_target_override.is_some(),
        };
        ui_calls.extend(overlay::build_status(&renderer.font_atlas, screen_w, &status));
        ui_calls.extend(overlay::build_legend(
            &renderer.font_atlas,
            screen_w,
            screen_h,
        ));
        if let Some(browser) = &self.browser
            && browser.open
        {
            ui_calls.extend(browser.build_draw_calls(&renderer.font_atlas, screen_w, screen_h));
        }
        if let Some(target) = self.trail_target_override {
            ui_calls.extend(overlay::build_target_crosshair(
                &renderer.camera,
                target,
                screen_w,
                screen_h,
            ));
        }
        renderer.render(
            &ui_calls,
            &effect_batches,
            &effect_draws,
            sprite_particle_records,
            &sprite_batches,
            &[],
            &[],
            dt,
        );
    }
}

/// Synthetic entity id for the previewed actor, so body-attached effects
/// (`spawn_on`) can shake / tint it like an in-game caster.
const VIEWER_ACTOR_ID: u32 = 1;

#[allow(clippy::too_many_arguments)]
fn build_character_batches<'a>(
    entity: &'a EntitySprite,
    map: &MapData,
    cell: (f32, f32),
    animation: &SpriteAnimationState,
    camera: &ragnarok_renderer::Camera,
    screen_w: f32,
    screen_h: f32,
    body_shake: [f32; 2],
    body_tint: Option<[u8; 3]>,
    effect_holder: &mut EffectHolder,
    entity_id: u32,
    emitting: bool,
) -> Vec<ragnarok_renderer::sprite::SpriteBatch<'a>> {
    let coords: &MapCoordinates = match map.coordinates.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let Some((screen_anchor, depth, camera_dir, sprite_scale, _depth_gradient)) =
        project_entity_screen(cell, map.gat.as_ref(), coords, camera, screen_w, screen_h)
    else {
        return Vec::new();
    };

    // Movement afterimage (`CBlurPC`): snapshot the moving actor on the emit
    // interval and draw every fading copy *before* the live sprite.
    let mut batches: Vec<ragnarok_renderer::sprite::SpriteBatch<'a>> = Vec::new();
    if let Some(ai) = effect_holder.afterimage_params_for_entity(entity_id) {
        if emitting && effect_holder.afterimage_emit_due(entity_id) {
            effect_holder.push_afterimage(AfterimageSnapshot::new(
                entity_id,
                animation.clone(),
                Some(camera_dir),
                animation.direction() as u8,
                screen_anchor,
                depth,
                sprite_scale,
                &ai,
            ));
        }
        for img in effect_holder.afterimages_for_entity(entity_id) {
            let mut copy = entity.build_batches(
                &img.anim,
                img.camera_dir,
                img.head_dir,
                img.anchor,
                img.depth,
                img.scale,
                0.0,
            );
            let (tr, tg, tb) = (
                img.tint[0] as f32 / 255.0,
                img.tint[1] as f32 / 255.0,
                img.tint[2] as f32 / 255.0,
            );
            for batch in &mut copy {
                for v in &mut batch.vertices {
                    v.color[0] *= tr;
                    v.color[1] *= tg;
                    v.color[2] *= tb;
                    v.color[3] *= img.alpha;
                }
            }
            batches.append(&mut copy);
        }
    }

    let anchor = [screen_anchor[0] + body_shake[0], screen_anchor[1] + body_shake[1]];
    let mut live = entity.build_batches(
        animation,
        Some(camera_dir),
        animation.direction() as u8,
        anchor,
        depth,
        sprite_scale,
        0.0,
    );
    if let Some([tr, tg, tb]) = body_tint {
        let (tr, tg, tb) = (tr as f32 / 255.0, tg as f32 / 255.0, tb as f32 / 255.0);
        for batch in &mut live {
            for v in &mut batch.vertices {
                v.color[0] *= tr;
                v.color[1] *= tg;
                v.color[2] *= tb;
            }
        }
    }
    batches.append(&mut live);
    batches
}

fn format_effect_label(id: EffectId) -> String {
    format!("{:?} ({})", id, id.value())
}

fn compute_world_anchor(cell: (f32, f32), map: &MapData) -> [f32; 3] {
    let Some(coords) = map.coordinates.as_ref() else {
        return [0.0, 0.0, 0.0];
    };
    cell_world_pos(cell, map.gat.as_ref(), coords)
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280u32, 800u32));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = block_on(Renderer::new(window.clone(), 14.0, 1.0));
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.last_frame = Instant::now();
        self.load_world();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_pressed = modifiers.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.browser_is_open() {
                    if event.state == ElementState::Pressed {
                        self.handle_browser_key(&event.logical_key);
                    }
                    return;
                }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Tab))
                {
                    self.open_browser();
                    return;
                }
                if let Some(action) = map_key(&event.logical_key, event.state) {
                    self.handle_action(action);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        self.mouse_down_left = pressed;
                        if pressed {
                            self.last_mouse = self.mouse_pos;
                            if self.placing_target {
                                self.try_place_target_at_cursor();
                            } else {
                                self.try_move_character_to_cursor();
                            }
                        }
                    }
                    MouseButton::Right => {
                        self.mouse_down_right = pressed;
                        if pressed {
                            self.last_mouse = self.mouse_pos;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);
                if self.mouse_down_right {
                    let dx = self.mouse_pos.0 - self.last_mouse.0;
                    let dy = self.mouse_pos.1 - self.last_mouse.1;
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera.yaw += dx * 0.0175;
                        renderer.camera.pitch = (renderer.camera.pitch - dy * 0.005)
                            .clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
                    }
                    self.last_mouse = self.mouse_pos;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                if let Some(renderer) = &mut self.renderer {
                    renderer.camera.distance =
                        (renderer.camera.distance - dy * 10.0).clamp(20.0, 1500.0);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Pick the nearest walkable cell to the GAT center. Falls back to (0,0)
/// when the map has no GAT; falls back to plain center when nothing is
/// walkable within a small radius (very small / synthetic maps).
fn pick_character_cell(gat: Option<&GatFile>) -> (f32, f32) {
    let Some(gat) = gat else {
        return (0.0, 0.0);
    };
    let cx = gat.width / 2;
    let cy = gat.height / 2;
    if gat.is_walkable(cx, cy) {
        return (cx as f32, cy as f32);
    }
    let radius: i32 = 32;
    for r in 1..=radius {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let x = cx + dx;
                let y = cy + dy;
                if gat.is_walkable(x, y) {
                    return (x as f32, y as f32);
                }
            }
        }
    }
    (cx as f32, cy as f32)
}

pub fn run(args: Args) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(args);
    event_loop.run_app(&mut app).unwrap();
}
