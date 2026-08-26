pub mod ailment;
pub mod animation;
pub mod app_state;
pub mod arrow;
pub mod autocounter;
pub mod banner;
pub mod book;
pub mod boss_info;
pub mod cast_scope;
pub mod char_name;
pub mod character;
pub mod chat_command;
pub mod chat_room;
pub mod companion;
pub mod cooldown;
pub mod cursor;
pub mod damage_number;
pub mod day_night;
pub mod display;
pub mod display_name;
pub mod doridori;
pub use ragnarok_effects as effect;
pub mod effects;
pub mod emotion;
pub mod gm;
pub mod gr2_model;
pub mod graffiti;
pub mod inventory;
pub mod item;
pub use cursor::RenderEntry;
pub mod data_table;
pub mod entity;
pub mod entity_collection;
pub mod event;
pub mod floor_item;
pub mod friends;
pub mod guild;
pub mod hotkey;
pub mod job_class;
pub mod keybinding;
pub mod level_aura;
pub mod lightmap;
pub mod mail;
pub mod map_cloud;
pub mod map_loader;
pub mod minimap_mark;
pub mod mob_info;
pub mod monster_info;
pub mod movement;
pub mod npc_dialog;
pub mod npc_shop;
pub mod party;
pub mod path;
pub mod pet;
pub mod pet_tables;
pub mod pk_rank;
pub mod poptip;
pub mod progress_bar;
pub mod quest;
pub mod scheduled_hit;
pub mod server_time;
pub mod shadow;
pub mod show_digit;
pub mod skill;
pub mod skill_action;
pub mod skill_msg;
pub mod sound;
pub mod sprite_loader;
pub mod sprite_path;
pub mod star_gladiator;
pub mod status_icon;
pub mod targeting;
pub mod trade;

/// Lookup key for a map: the bare name, lowercased. Map names reach us in three
/// forms — `prontera` from the world map tables, `prontera.rsw` from the data
/// tables, `prontera.gat` from the party and guild packets — and all three must
/// resolve to the same map.
pub fn map_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    match lower.strip_suffix(".gat").or(lower.strip_suffix(".rsw")) {
        Some(bare) => bare.to_string(),
        None => lower,
    }
}
