use crate::sprite_viewer::browser::SpriteBrowser;
use crate::stress::{self, StressRunner, StressSet, StressTick, stress_label};
use crate::viewer::controls::{ViewerAction, map_key};
use crate::viewer::overlay::{self, StatusLine};
use models::enums::EnumWithNumberValue;
use models::enums::EnumWithStringValue;
use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use ragnarok_formats::act::{MotionType, SpriteActionType, SpriteAnimationState};
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::map_coordinates::MapCoordinates;
use ragnarok_game::damage_number::{
    DamageNumber, DamageNumberManager, DamageNumberRenderEntry, build_damage_number_quads,
};
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use ragnarok_game::effect::spec::EffectAnchor;
use ragnarok_game::effect::{
    EffectQueue, EffectSpec, body_attached, effect_spec, effect_texture_paths, is_link_effect,
    is_trail_effect, str_aliases,
};
use ragnarok_game::map_loader::{self, MapData};
use ragnarok_game::sprite_loader as game_sprite_loader;
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_renderer::EffectSpriteCache;
use ragnarok_renderer::Fps;
use ragnarok_renderer::effect::holder::AfterimageSnapshot;
use ragnarok_renderer::effect::{
    EffectFrameInputs, EffectHolder, EffectUpdateCtx, StrEffectCache, compose_effect_frame,
};
use ragnarok_renderer::sprite::{EntitySprite, build_entity_sprite, upload_sprite_textures};
use ragnarok_renderer::sprite_projection::{cell_world_pos, project_entity_screen};
use ragnarok_renderer::{BackgroundMode, FrameInputs, Renderer, UiDrawCall, block_on};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Args {
    pub grf_path: String,
    pub map_name: String,
    pub effect_id: Option<u16>,
    pub cell: Option<(i32, i32)>,
    pub direction: Option<u8>,
}

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

    effect_holder: EffectHolder,
    effect_queue: EffectQueue,
    str_effects: StrEffectCache,
    damage_numbers: DamageNumberManager,
    damage_number_textures: Option<ragnarok_renderer::sprite::SpriteTextures>,
    damage_number_act: Option<ragnarok_formats::act::ActFile>,
    effect_sprites: EffectSpriteCache,
    attempted_spr_files: std::collections::HashSet<String>,
    effect_list: Vec<EffectId>,
    current_effect_idx: usize,
    current_effect_id: Option<EffectId>,
    current_effect_label: String,

    browser: Option<SpriteBrowser>,
    browser_lookup: HashMap<String, EffectId>,
    ctrl_pressed: bool,

    stress_sets: Vec<StressSet>,
    stress: StressRunner,
    stress_browser: Option<SpriteBrowser>,
    fps: Fps,

    trail_target_override: Option<[f32; 3]>,
    placing_target: bool,

    mouse_pos: (f32, f32),
    last_mouse: (f32, f32),
    mouse_down_left: bool,
    mouse_down_right: bool,

    last_frame: Instant,
    next_frame: Instant,
    clear_is_black: bool,
}

const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

impl App {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            window: None,
            renderer: None,
            grf: None,
            map_data: None,
            composite_job: JobName::Clown.value() as u16,
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
            damage_numbers: DamageNumberManager::new(),
            damage_number_textures: None,
            damage_number_act: None,
            effect_sprites: EffectSpriteCache::new(),
            attempted_spr_files: std::collections::HashSet::new(),
            effect_list: build_effect_list(),
            current_effect_idx: 0,
            current_effect_id: None,
            current_effect_label: "(none)".to_string(),
            browser: None,
            browser_lookup: HashMap::new(),
            ctrl_pressed: false,
            stress_sets: stress::stress_sets(),
            stress: StressRunner::new(),
            stress_browser: None,
            fps: Fps::new(),
            trail_target_override: None,
            placing_target: false,
            mouse_pos: (0.0, 0.0),
            last_mouse: (0.0, 0.0),
            mouse_down_left: false,
            mouse_down_right: false,
            last_frame: Instant::now(),
            next_frame: Instant::now(),
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

        if let Some(sprite_data) = game_sprite_loader::load_damage_number_sprite(&grf) {
            self.damage_number_textures = Some(upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            ));
            self.damage_number_act = Some(sprite_data.act);
        }

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
            false,
            false,
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
            None,
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
            self.effect_queue.spawn_on(id, VIEWER_ACTOR_ID);
        } else if is_trail_effect(id) || is_link_effect(id) {
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
        tracing::info!("Spawning effect {} ({:?}) at {:?}", id.value(), id, pos);
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

    fn open_stress_browser(&mut self) {
        let items: Vec<String> = self.stress_sets.iter().map(stress_label).collect();
        let mut browser = SpriteBrowser::new(items, "stress tests");
        if let Some(renderer) = &self.renderer {
            let h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            browser.update_visible_rows(h);
        }
        self.stress_browser = Some(browser);
    }

    fn stress_browser_is_open(&self) -> bool {
        self.stress_browser.as_ref().is_some_and(|b| b.open)
    }

    fn handle_stress_browser_key(&mut self, key: &Key) {
        let Some(browser) = &mut self.stress_browser else {
            return;
        };
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => browser.open = false,
            Key::Named(NamedKey::Enter) => self.handle_stress_browser_select(),
            Key::Named(NamedKey::ArrowUp) => browser.handle_up(),
            Key::Named(NamedKey::ArrowDown) => browser.handle_down(),
            Key::Named(NamedKey::PageUp) => browser.handle_page_up(),
            Key::Named(NamedKey::PageDown) => browser.handle_page_down(),
            Key::Named(NamedKey::Backspace) => browser.handle_backspace(),
            Key::Character(ch) => {
                for c in ch.chars() {
                    if !c.is_control() {
                        browser.handle_char(c);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_stress_browser_select(&mut self) {
        let Some(browser) = &mut self.stress_browser else {
            return;
        };
        let Some(selected) = browser.selected_item().map(|s| s.to_string()) else {
            return;
        };
        browser.open = false;
        if let Some(idx) = self
            .stress_sets
            .iter()
            .position(|s| stress_label(s) == selected)
        {
            self.effect_holder.clear();
            self.stress.launch(idx);
        }
    }

    fn stop_stress(&mut self) {
        self.stress.stop();
        self.effect_holder.clear();
    }

    /// Clear the holder and re-spawn stress set `idx` at fresh random visible
    /// ground positions. Driven by `StressTick::Reseed` each cadence.
    fn reseed_stress(&mut self, idx: usize) {
        self.effect_holder.clear();
        let entries = match self.stress_sets.get(idx) {
            Some(s) => s.entries.clone(),
            None => return,
        };
        let Some(renderer) = &self.renderer else {
            return;
        };
        let screen_w = renderer.device.surface_config.width as f32;
        let screen_h = renderer.device.surface_config.height as f32;
        let plane_y = self
            .map_data
            .as_ref()
            .map(|m| self.world_anchor(m)[1])
            .unwrap_or(0.0);
        let mut rng = stress::Rng::new(self.stress.next_seed());
        for (id, count) in entries {
            self.ensure_str_loaded(id);
            self.ensure_spr_loaded_for(id);
            let positions = {
                let camera = &self.renderer.as_ref().unwrap().camera;
                stress::random_visible_ground_positions(
                    camera, screen_w, screen_h, plane_y, count, &mut rng,
                )
            };
            for pos in positions {
                stress::enqueue_effect(&mut self.effect_queue, id, pos, Some(VIEWER_ACTOR_ID));
            }
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
            Some(EffectSpec::Custom) => {
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
            Some(EffectSpec::Custom) => {
                let probe = ragnarok_game::effect::factory::make_effect(
                    id,
                    EffectAnchor::Point([0.0, 0.0, 0.0]),
                    None,
                    None,
                    None,
                );
                let Some(probe) = probe else { return };
                let Some(overlay) = probe.str_overlay() else {
                    return;
                };
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
        self.fps.tick(dt);

        // Continuous stress test: re-seed the active set on its cadence so the
        // population stays steady. Runs before the queue drain below.
        if let StressTick::Reseed(idx) = self.stress.tick(sim_dt) {
            self.reseed_stress(idx);
        }

        // Caster-attached effects (buff STR overlays, body-attached spawns)
        // resolve to the previewed actor's world anchor.
        let actor_pos = self
            .map_data
            .as_ref()
            .map(|m| compute_world_anchor(self.character_cell, m))
            .unwrap_or([0.0, 0.0, 0.0]);
        let resolve_actor = |id: u32| (id == VIEWER_ACTOR_ID).then_some(actor_pos);
        self.effect_holder
            .drain_queue(&mut self.effect_queue, &resolve_actor);
        self.effect_holder.update(
            &EffectUpdateCtx {
                delta: sim_dt,
                camera_target: None,
                caster_yaw: None,
            },
            &|_| None,
            &resolve_actor,
        );
        // One-shot forced animation (Jumpkick): drain it and play it on the
        // previewed actor so the kick pose is visible in-tool (the viewer holds
        // the actor on a static frame, so this snaps it to the action's start).
        if let Some(ba) = self
            .effect_holder
            .take_body_action_for_entity(VIEWER_ACTOR_ID)
        {
            self.animation
                .play(ba.action_index, ba.duration_ms, ba.start_frame);
        }
        // Floating numbers: turn each one-shot request into a number on the
        // previewed actor, then age the manager. Direction is unused (no drift).
        for (entity_id, req) in self.effect_holder.drain_number_requests() {
            self.damage_numbers.add(DamageNumber::effect_number(
                entity_id, req.value, req.color, 0,
            ));
        }
        self.damage_numbers.update(sim_dt);
        let body_channels = self.effect_holder.body_channels_for_entity(VIEWER_ACTOR_ID);
        // Screen-shake from quake effects like Magiccrasher / Falconassault,
        // mirroring the in-game actor pass so the tool shows the shake too.
        let shake_offset = self.effect_holder.camera_shake_offset();

        let Some(renderer) = &mut self.renderer else {
            return;
        };
        renderer.camera.shake_offset = shake_offset.into();
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
            extra_sprite_particles: &[],
        });
        let effect_batches = frame.effect_batches;
        let effect_draws = frame.effect_draws;
        let sprite_particle_records = frame.sprite_particle_records;

        let (sprite_batches, silhouette_batches): (
            Vec<ragnarok_renderer::sprite::SpriteBatch<'_>>,
            Vec<ragnarok_renderer::sprite::SpriteBatch<'_>>,
        ) = match (&self.entity_sprite, &self.map_data) {
            (Some(entity), Some(map)) => {
                // Trail only while the walk action plays (the viewer's
                // stand-in for the in-game `Moving` state).
                let emitting = self.animation.action() == SpriteActionType::Walk as usize;
                build_character_batches(
                    entity,
                    map,
                    self.character_cell,
                    &self.animation,
                    &renderer.camera,
                    screen_w,
                    screen_h,
                    &body_channels,
                    &mut self.effect_holder,
                    VIEWER_ACTOR_ID,
                    emitting,
                )
            }
            _ => (Vec::new(), Vec::new()),
        };

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
        ui_calls.extend(overlay::build_status(
            &renderer.font_atlas,
            screen_w,
            &status,
        ));
        ui_calls.extend(overlay::build_legend(
            &renderer.font_atlas,
            screen_w,
            screen_h,
        ));
        ui_calls.extend(crate::viewer_common::build_fps(
            &renderer.font_atlas,
            self.fps.get(),
            self.effect_holder.len(),
        ));
        if let Some(browser) = &self.browser
            && browser.open
        {
            ui_calls.extend(browser.build_draw_calls(&renderer.font_atlas, screen_w, screen_h));
        }
        if let Some(browser) = &self.stress_browser
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

        // Floating numbers: project the previewed actor's head and lay the
        // recoloured numbers over it (same path as the in-game scene render).
        let mut number_inline_textures: Vec<&wgpu::BindGroup> = Vec::new();
        if let (Some(entity), Some(map), Some(num_tex), Some(num_act)) = (
            &self.entity_sprite,
            &self.map_data,
            &self.damage_number_textures,
            &self.damage_number_act,
        ) && !self.damage_numbers.numbers.is_empty()
            && let Some(coords) = map.coordinates.as_ref()
            && let Some((screen_anchor, depth, camera_dir, sprite_scale, _)) = project_entity_screen(
                self.character_cell,
                map.gat.as_ref(),
                coords,
                &renderer.camera,
                screen_w,
                screen_h,
            )
        {
            let head_dir = self.animation.direction() as u8;
            let head_offset = entity.compute_head_offset(
                &self.animation,
                Some(camera_dir),
                head_dir,
                screen_anchor,
                depth,
                sprite_scale,
            );
            let entries: Vec<DamageNumberRenderEntry> = self
                .damage_numbers
                .numbers
                .iter()
                .filter(|d| d.entity_id == VIEWER_ACTOR_ID)
                .filter_map(|dmg| {
                    Some(DamageNumberRenderEntry {
                        entity_id: dmg.entity_id,
                        screen_x: screen_anchor[0],
                        screen_y: screen_anchor[1] - head_offset,
                        scale: sprite_scale,
                        data: dmg.render_data()?,
                    })
                })
                .collect();
            let quads = build_damage_number_quads(
                &entries,
                num_act,
                &num_tex.sizes,
                num_tex.indexed_count,
                None,
            );
            ragnarok_renderer::render_damage_number_quads(
                &quads,
                num_tex,
                None,
                &mut ui_calls,
                &mut number_inline_textures,
            );
        }

        renderer.render(FrameInputs {
            ui_draw_calls: &ui_calls,
            effect_sprite_batches: &effect_batches,
            effect_draws: &effect_draws,
            sprite_particle_records,
            sprite_batches: &sprite_batches,
            silhouette_batches: &silhouette_batches,
            cursor_batches: &[],
            inline_textures: &number_inline_textures,
            elapsed: dt,
        });
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
    body_channels: &ragnarok_renderer::BodyChannels,
    effect_holder: &mut EffectHolder,
    entity_id: u32,
    emitting: bool,
) -> (
    Vec<ragnarok_renderer::sprite::SpriteBatch<'a>>,
    Vec<ragnarok_renderer::sprite::SpriteBatch<'a>>,
) {
    let coords: &MapCoordinates = match map.coordinates.as_ref() {
        Some(c) => c,
        None => return (Vec::new(), Vec::new()),
    };
    let Some((screen_anchor, depth, camera_dir, sprite_scale, _depth_gradient)) =
        project_entity_screen(cell, map.gat.as_ref(), coords, camera, screen_w, screen_h)
    else {
        return (Vec::new(), Vec::new());
    };

    // Flat feet-depth body silhouette (gradient [0,0]) stamped into depth after
    // the colour pass so effects occlude against the body — same as the game.
    let silhouette = entity.build_batches(
        animation,
        Some(camera_dir),
        animation.direction() as u8,
        screen_anchor,
        depth,
        sprite_scale,
        [0.0, 0.0],
    );

    // Movement afterimage (`CBlurPC`): snapshot the moving actor on the emit
    // interval and draw every fading copy *before* the live sprite. The trail
    // uses the un-yawed facing, so it stays outside the shared composer.
    let mut batches: Vec<ragnarok_renderer::sprite::SpriteBatch<'a>> = Vec::new();
    if let Some(ai) = effect_holder.afterimage_params_for_entity(entity_id) {
        let action = animation.action();
        let motion = animation.motion_index();
        let last = effect_holder
            .afterimages_for_entity(entity_id)
            .last()
            .map(|i| (i.anim.action(), i.anim.motion_index()));
        // Fill every frame the animation passed through this tick so a fast
        // swing's arc has no gaps (same as the game scene).
        let frames: Vec<usize> = match last {
            Some((a, m)) if a == action && motion > m => (m + 1..=motion).collect(),
            Some((a, m)) if a == action && motion == m => Vec::new(),
            _ => vec![motion],
        };
        if emitting {
            for frame in frames {
                let mut anim = animation.clone();
                anim.set_motion_index(frame);
                effect_holder.push_afterimage(AfterimageSnapshot::new(
                    entity_id,
                    anim,
                    Some(camera_dir),
                    animation.direction() as u8,
                    // The preview actor is stationary in screen space; no world
                    // projection, so the frozen anchor/depth/scale below are used.
                    (0.0, 0.0),
                    screen_anchor,
                    depth,
                    sprite_scale,
                    &ai,
                ));
            }
        }
        for img in effect_holder.afterimages_for_entity(entity_id) {
            let mut copy = entity.build_batches(
                &img.anim,
                img.camera_dir,
                img.head_dir,
                img.anchor,
                img.depth,
                img.scale,
                [0.0, 0.0],
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

    let mut live = ragnarok_renderer::compose_actor_batches(
        entity,
        animation,
        camera_dir,
        animation.direction() as u8,
        screen_anchor,
        depth,
        sprite_scale,
        [0.0, 0.0],
        body_channels,
    );
    batches.append(&mut live);
    (batches, silhouette)
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
                if self.stress_browser_is_open() {
                    if event.state == ElementState::Pressed {
                        self.handle_stress_browser_key(&event.logical_key);
                    }
                    return;
                }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Tab))
                {
                    self.open_browser();
                    return;
                }
                if event.state == ElementState::Pressed {
                    match event.logical_key.as_ref() {
                        Key::Character("g") | Key::Character("G") => {
                            self.open_stress_browser();
                            return;
                        }
                        Key::Character("k") | Key::Character("K") => {
                            self.stop_stress();
                            return;
                        }
                        _ => {}
                    }
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
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
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
