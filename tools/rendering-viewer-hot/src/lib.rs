#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_formats::act::ActFile;
use ragnarok_game::damage_number::{
    DamageNumber, DamageNumberManager, DamageNumberQuad, DamageNumberRenderEntry, DamageNumberType,
    build_damage_number_quads,
};
use ragnarok_game::entity::facing_degrees_for;
use ragnarok_game::scheduled_hit::{DOUBLE_ATTACK_TERM, ScheduledHit as GameHit, Swing};

struct ScheduledHit {
    delay: f32,
    entity_id: u32,
    hit: GameHit,
    is_player_target: bool,
}

// Fixed screen positions for each scenario entity_id (1-9)
const GRID_COLS: usize = 5;
const GRID_CELL_W: f32 = 160.0;
const GRID_CELL_H: f32 = 120.0;
const GRID_OFFSET_X: f32 = 230.0;
const GRID_OFFSET_Y: f32 = 80.0;

fn entity_screen_pos(entity_id: u32) -> (f32, f32) {
    let idx = (entity_id.saturating_sub(1)) as usize;
    let col = idx % GRID_COLS;
    let row = idx / GRID_COLS;
    let x = GRID_OFFSET_X + col as f32 * GRID_CELL_W + GRID_CELL_W / 2.0;
    let y = GRID_OFFSET_Y + row as f32 * GRID_CELL_H + GRID_CELL_H / 2.0;
    (x, y)
}

struct SpriteMetadata {
    num_act: ActFile,
    num_sizes: Vec<(u32, u32)>,
    num_indexed_count: usize,
    msg_sizes: Vec<(u32, u32)>,
}

struct State {
    damage_numbers: DamageNumberManager,
    scheduled_hits: Vec<ScheduledHit>,
    damage_value: i32,
    direction: u8,
    sprites: Option<SpriteMetadata>,
}

impl State {
    fn trigger_scenario(&mut self, scenario: u8) {
        match scenario {
            1 => {
                self.damage_numbers.emit(
                    1,
                    facing_degrees_for(self.direction),
                    &GameHit::single(self.damage_value, 0, false),
                    false,
                    false,
                );
            }
            2 => {
                self.damage_numbers.emit(
                    2,
                    facing_degrees_for(self.direction),
                    &GameHit::single(self.damage_value, 1, false),
                    false,
                    false,
                );
            }
            3 => {
                self.damage_numbers.emit(
                    3,
                    facing_degrees_for(self.direction),
                    &GameHit::single(self.damage_value, 0, true),
                    false,
                    false,
                );
            }
            4 => {
                self.damage_numbers.emit(
                    4,
                    facing_degrees_for(self.direction),
                    &GameHit::single(self.damage_value, 0, false),
                    true,
                    false,
                );
            }
            5 => {
                let per_hit = self.damage_value / 3;
                let delay = 0.2;
                for i in 0..3u16 {
                    self.scheduled_hits.push(ScheduledHit {
                        delay: delay * i as f32,
                        entity_id: 5,
                        hit: GameHit::multi_hit(per_hit, self.damage_value, 1, i, i == 2),
                        is_player_target: false,
                    });
                }
            }
            6 => {
                let per_hit = self.damage_value / 3;
                let delay = 0.2;
                for i in 0..3u16 {
                    self.scheduled_hits.push(ScheduledHit {
                        delay: delay * i as f32,
                        entity_id: 6,
                        hit: GameHit::multi_hit(per_hit, self.damage_value, 0, i, i == 2),
                        is_player_target: false,
                    });
                }
            }
            7 => {
                // A recovery number is not a hit: `emit` drops anything with a
                // negative damage, so it never reaches the manager that way.
                self.damage_numbers.add(DamageNumber::new(
                    7,
                    self.damage_value,
                    DamageNumberType::Heal,
                    facing_degrees_for(self.direction),
                ));
            }
            8 => {
                self.damage_numbers.emit(
                    8,
                    facing_degrees_for(self.direction),
                    &GameHit::single(0, 0, false),
                    false,
                    true,
                );
            }
            9 => {
                self.damage_numbers.add(DamageNumber::new(
                    9,
                    0,
                    DamageNumberType::Lucky,
                    facing_degrees_for(self.direction),
                ));
            }
            10 => {
                let per_hit = self.damage_value / 3;
                for i in 0..3u16 {
                    let mut hit = GameHit::multi_hit(per_hit, self.damage_value, 0, i, i == 2);
                    hit.is_critical = true;
                    self.scheduled_hits.push(ScheduledHit {
                        delay: DOUBLE_ATTACK_TERM * i as f32,
                        entity_id: 10,
                        hit,
                        is_player_target: false,
                    });
                }
            }
            11 => {
                let swing = Swing {
                    damage: self.damage_value,
                    left_damage: self.damage_value / 3,
                    count: 2,
                    is_endure: false,
                    is_critical: false,
                    attacker_gid: 0,
                    attacked_mt_secs: 0.288,
                    fire_at: 0.0,
                };
                for hit in swing.schedule() {
                    self.scheduled_hits.push(ScheduledHit {
                        delay: hit.fire_at,
                        entity_id: 11,
                        hit,
                        is_player_target: false,
                    });
                }
            }
            0 => {
                for s in 1..=11u8 {
                    self.trigger_scenario(s);
                }
            }
            _ => {}
        }
    }

    fn process_scheduled_hits(&mut self, dt: f32) {
        let mut ready = Vec::new();
        self.scheduled_hits.retain_mut(|scheduled| {
            scheduled.delay -= dt;
            if scheduled.delay <= 0.0 {
                ready.push((
                    scheduled.entity_id,
                    scheduled.hit,
                    scheduled.is_player_target,
                ));
                false
            } else {
                true
            }
        });
        for (entity_id, hit, is_player_target) in ready {
            self.damage_numbers.emit(
                entity_id,
                facing_degrees_for(self.direction),
                &hit,
                is_player_target,
                true,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hot_create() -> *mut () {
    let state = State {
        damage_numbers: DamageNumberManager::new(),
        scheduled_hits: Vec::new(),
        damage_value: 1234,
        direction: 0,
        sprites: None,
    };
    Box::into_raw(Box::new(state)) as *mut ()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_destroy(state_ptr: *mut ()) {
    if !state_ptr.is_null() {
        unsafe { drop(Box::from_raw(state_ptr as *mut State)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_trigger(
    state_ptr: *mut (),
    scenario: u8,
    damage_value: i32,
    direction: u8,
) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.damage_value = damage_value;
    state.direction = direction;
    state.trigger_scenario(scenario);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_update(state_ptr: *mut (), dt: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.damage_numbers.update(dt);
    state.process_scheduled_hits(dt);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_init_sprites(
    state_ptr: *mut (),
    act_bytes_ptr: *const u8,
    act_bytes_len: usize,
    num_sizes_ptr: *const (u32, u32),
    num_sizes_len: usize,
    num_indexed_count: usize,
    msg_sizes_ptr: *const (u32, u32),
    msg_sizes_len: usize,
) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let act_data = unsafe { std::slice::from_raw_parts(act_bytes_ptr, act_bytes_len) };
    let num_act = match ActFile::parse(act_data) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hot_init_sprites: failed to parse ACT: {e}");
            return;
        }
    };
    let num_sizes = unsafe { std::slice::from_raw_parts(num_sizes_ptr, num_sizes_len) };
    let msg_sizes = if msg_sizes_ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(msg_sizes_ptr, msg_sizes_len) }
    };
    state.sprites = Some(SpriteMetadata {
        num_act,
        num_sizes: num_sizes.to_vec(),
        num_indexed_count,
        msg_sizes: msg_sizes.to_vec(),
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_build(state_ptr: *mut (), out_quads: *mut Vec<DamageNumberQuad>) {
    let state = unsafe { &*(state_ptr as *const State) };
    let out = unsafe { &mut *out_quads };

    let sprites = match &state.sprites {
        Some(s) => s,
        None => return,
    };

    let entries: Vec<DamageNumberRenderEntry> = state
        .damage_numbers
        .numbers
        .iter()
        .filter_map(|dmg| {
            use ragnarok_game::damage_number::{
                STANDARD_MAP_ZOOM, flat_screen_offset, pixels_per_world_unit,
            };
            let anchor = entity_screen_pos(dmg.entity_id);
            let ppu = pixels_per_world_unit(1.0, STANDARD_MAP_ZOOM);
            let (screen_x, screen_y) = flat_screen_offset(anchor, dmg.world_offset(), ppu);
            let backdrop_screen = dmg.backdrop_world_offset().map(|o| {
                let (bx, by) = flat_screen_offset(anchor, o, ppu);
                (bx, by, 1.0)
            });
            let data = dmg.render_data()?;
            Some(DamageNumberRenderEntry {
                entity_id: dmg.entity_id,
                screen_x,
                screen_y,
                scale: 1.0,
                backdrop_screen,
                data,
            })
        })
        .collect();

    let msg_sizes = if sprites.msg_sizes.is_empty() {
        None
    } else {
        Some(sprites.msg_sizes.as_slice())
    };

    let num_act = &sprites.num_act;
    let quads = build_damage_number_quads(
        &entries,
        num_act,
        &sprites.num_sizes,
        sprites.num_indexed_count,
        msg_sizes,
    );
    out.extend(quads);
}
