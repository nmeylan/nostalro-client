#[path = "shared/mod.rs"]
mod shared;

use libloading::{Library, Symbol};
use ragnarok_game::data_table::item_resource_table::ItemResourceTable;
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui_component::Window;
use ragnarok_ui_component::account::char_create_window::CharCreateWindow;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::account::login_server_list_window::LoginServerListWindow;
use ragnarok_ui_component::account::login_window::LoginWindow;
use ragnarok_ui_component::account::server_list_window::ServerListWindow;
use ragnarok_ui_component::game::basic_info_window::BasicInfoWindow;
use ragnarok_ui_component::game::cart_window::CartWindow;
use ragnarok_ui_component::game::chat_room_create_window::ChatRoomCreateWindow;
use ragnarok_ui_component::game::chat_room_member_window::ChatRoomMemberWindow;
use ragnarok_ui_component::game::chat_window::ChatWindow;
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::game::emotion_window::EmotionWindow;
use ragnarok_ui_component::game::equipment_window::EquipmentWindow;
use ragnarok_ui_component::game::guild_window::GuildWindow;
use ragnarok_ui_component::game::homun_window::HomunWindow;
use ragnarok_ui_component::game::hotkey_bar::HotkeyBarWindow;
use ragnarok_ui_component::game::inventory_window::InventoryWindow;
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::mercenary_skill_window::MercenarySkillWindow;
use ragnarok_ui_component::game::mercenary_window::MercenaryWindow;
use ragnarok_ui_component::game::my_shop_window::MyShopWindow;
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::party_friends_window::PartyFriendsWindow;
use ragnarok_ui_component::game::shortcut_list_window::ShortcutListWindow;
use ragnarok_ui_component::game::skill_tree_window::SkillTreeWindow;
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::game::vending_setup_window::VendingSetupWindow;
use ragnarok_ui_component::game::vending_shop_window::VendingShopWindow;
use ragnarok_ui_component::game::world_map_window::{WORLD_MAP_TEX, WorldMapWindow};
use ragnarok_ui_component::helper::dialog_container::DialogContainer;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// Force system allocator so host and dylib share the same heap.
// A cdylib gets its own Rust runtime; without this, Vec/String
// reallocations across the FFI boundary would mix allocators.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

type HotCreateFn = unsafe extern "C" fn(*const u8, usize) -> *mut ();
type HotGrfInitFn = unsafe extern "C" fn(
    *mut (),
    unsafe extern "C" fn(*const u8, usize, *mut u32, *mut u32) -> bool,
    *const ItemResourceTable,
);
type HotBuildFn = unsafe extern "C" fn(*mut (), *mut UiFrame);
type HotDestroyFn = unsafe extern "C" fn(*mut ());

struct HotLib {
    _lib: Library,
    state: *mut (),
    build_fn: HotBuildFn,
    destroy_fn: HotDestroyFn,
    grf_init_fn: HotGrfInitFn,
}

impl HotLib {
    fn load(dylib_path: &Path, example_name: &str) -> Option<Self> {
        let lib = match unsafe { Library::new(dylib_path) } {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Failed to load dylib: {e}");
                return None;
            }
        };

        let (create_fn, build_fn, destroy_fn, grf_init_fn) = unsafe {
            let create: Symbol<HotCreateFn> = lib.get(b"hot_create").ok()?;
            let build: Symbol<HotBuildFn> = lib.get(b"hot_build").ok()?;
            let destroy: Symbol<HotDestroyFn> = lib.get(b"hot_destroy").ok()?;
            let grf_init: Symbol<HotGrfInitFn> = lib.get(b"hot_grf_init").ok()?;
            (*create, *build, *destroy, *grf_init)
        };

        let state = unsafe { (create_fn)(example_name.as_ptr(), example_name.len()) };

        Some(Self {
            _lib: lib,
            state,
            build_fn,
            destroy_fn,
            grf_init_fn,
        })
    }

    /// Destroy state using current dylib's destroy function, then drop the Library.
    /// This ensures the dylib is fully unloaded before any new dylib is loaded.
    fn unload(mut self) {
        if !self.state.is_null() {
            unsafe { (self.destroy_fn)(self.state) };
            self.state = std::ptr::null_mut();
        }
        // _lib dropped here, calling dlclose
    }

    fn grf_init(
        &self,
        texture_size_fn: unsafe extern "C" fn(*const u8, usize, *mut u32, *mut u32) -> bool,
        item_resource_table: Option<&ItemResourceTable>,
    ) {
        let table_ptr = match item_resource_table {
            Some(t) => t as *const ItemResourceTable,
            None => std::ptr::null(),
        };
        unsafe { (self.grf_init_fn)(self.state, texture_size_fn, table_ptr) };
    }

    fn build(&self, ui: &mut UiFrame) {
        unsafe { (self.build_fn)(self.state, ui as *mut UiFrame) };
    }
}

const GAME_COMPONENTS: &[&str] = &[
    "inventory",
    "npc_shop_buy",
    "npc_shop_sell",
    "npc_dialog",
    "equipment",
    "system_menu",
    "confirm_dialog",
    "dialog_container",
    "item_info",
    "skill_tree",
    "card_insert",
    "hotkey_bar",
    "basic_info",
];
const SOCIAL_COMPONENTS: &[&str] = &[
    "inventory",
    "guild",
    "chat",
    "chat_room_create",
    "chat_room_member",
    "chat_room_board",
    "vending_board",
    "party",
    "emotion",
    "shortcut_list",
];
const ACCOUNT_COMPONENTS: &[&str] = &[
    "login",
    "login_server_list",
    "server_list",
    "char_select",
    "char_create",
];
const SHOP_COMPONENTS: &[&str] = &["cart", "vending_setup", "my_shop", "vending_buy"];
const COMPANION_COMPONENTS: &[&str] = &["mercenary", "mercenary_skill", "homun"];

fn grf_texture_paths_single(name: &str) -> Vec<&'static str> {
    match name {
        "inventory" => InventoryWindow::grf_texture_paths(),
        "cart" => CartWindow::grf_texture_paths(),
        "vending_setup" => VendingSetupWindow::grf_texture_paths(),
        "my_shop" => MyShopWindow::grf_texture_paths(),
        "vending_buy" => VendingShopWindow::grf_texture_paths(),
        "vending_board" => ragnarok_ui_component::game::vending_board::grf_texture_paths(),
        "chat_room_board" => ragnarok_ui_component::game::chat_room_board::grf_texture_paths(),
        "chat_room_create" => ChatRoomCreateWindow::grf_texture_paths(),
        "chat_room_member" => ChatRoomMemberWindow::grf_texture_paths(),
        "emotion" => EmotionWindow::grf_texture_paths(),
        "shortcut_list" => ShortcutListWindow::grf_texture_paths(),
        "party" => PartyFriendsWindow::grf_texture_paths(),
        "npc_shop_buy" => NpcShop::grf_texture_paths(),
        "npc_shop_sell" => NpcShop::grf_texture_paths(),
        "login" => LoginWindow::grf_texture_paths(),
        "chat" => ChatWindow::grf_texture_paths(),
        "npc_dialog" => NpcDialog::grf_texture_paths(),
        "confirm_dialog" => ConfirmDialog::grf_texture_paths(),
        "login_server_list" => LoginServerListWindow::grf_texture_paths(),
        "server_list" => ServerListWindow::grf_texture_paths(),
        "equipment" => EquipmentWindow::grf_texture_paths(),
        "system_menu" => SystemMenu::grf_texture_paths(),
        "char_select" => CharSelectWindow::grf_texture_paths(),
        "char_create" => CharCreateWindow::grf_texture_paths(),
        "dialog_container" => DialogContainer::grf_texture_paths(),
        "item_info" => ItemInfoWindow::grf_texture_paths(),
        "skill_tree" => SkillTreeWindow::grf_texture_paths(),
        "card_insert" => vec![],
        "world_map" => {
            let mut paths = WorldMapWindow::grf_texture_paths();
            paths.push(WORLD_MAP_TEX);
            paths.push("data/texture/유저인터페이스/map/prontera.bmp");
            paths
        }
        "mercenary" => MercenaryWindow::grf_texture_paths(),
        "mercenary_skill" => MercenarySkillWindow::grf_texture_paths(),
        "homun" => HomunWindow::grf_texture_paths(),
        "guild" => GuildWindow::grf_texture_paths(),
        "basic_info" => BasicInfoWindow::grf_texture_paths(),
        "hotkey_bar" => {
            let mut paths = HotkeyBarWindow::grf_texture_paths();
            paths.extend(InventoryWindow::grf_texture_paths());
            paths.extend(SkillTreeWindow::grf_texture_paths());
            paths.sort_unstable();
            paths.dedup();
            paths
        }
        _ => {
            eprintln!("Unknown example: {name}");
            vec![]
        }
    }
}

fn grf_texture_paths(example_name: &str) -> Vec<&'static str> {
    let names: &[&str] = match example_name {
        "game" => GAME_COMPONENTS,
        "social" => SOCIAL_COMPONENTS,
        "account" => ACCOUNT_COMPONENTS,
        "shop" => SHOP_COMPONENTS,
        "companion" => COMPANION_COMPONENTS,
        _ => return grf_texture_paths_single(example_name),
    };
    let mut paths: Vec<&'static str> = names
        .iter()
        .flat_map(|n| grf_texture_paths_single(n))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

fn find_dylib() -> PathBuf {
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("debug");

    #[cfg(target_os = "linux")]
    let name = "libragnarok_ui_component_hot.so";
    #[cfg(target_os = "macos")]
    let name = "libragnarok_ui_component_hot.dylib";
    #[cfg(target_os = "windows")]
    let name = "ragnarok_ui_component_hot.dll";

    target_dir.join(name)
}

fn dylib_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut example_name = "inventory".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--example" => {
                i += 1;
                if i < args.len() {
                    example_name = args[i].clone();
                }
            }
            _ => {}
        }
        i += 1;
    }

    let dylib_path = find_dylib();
    eprintln!("Dylib path: {}", dylib_path.display());
    eprintln!("Example: {example_name}");

    while !dylib_path.exists() {
        eprintln!("Waiting for dylib... Run: cargo build -p ragnarok-ui-component-hot");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let mut hot_lib =
        HotLib::load(&dylib_path, &example_name).expect("Failed to load initial dylib");
    let mut last_mtime = dylib_mtime(&dylib_path).unwrap_or(SystemTime::UNIX_EPOCH);
    let mut grf_initialized = false;
    let mut reload_counter = 0u64;

    let texture_paths = grf_texture_paths(&example_name);
    let example_name_for_closure = example_name.clone();
    let is_category = matches!(
        example_name.as_str(),
        "game" | "social" | "account" | "shop" | "companion" | "guild"
    );
    let (win_w, win_h) = if is_category {
        (1600, 1000)
    } else {
        (800, 600)
    };

    shared::UiExampleApp::new("Hot Reload", win_w, win_h, move |ctx| {
        // Poll for dylib changes (mtime check is cheap)
        if let Some(mtime) = dylib_mtime(&dylib_path) {
            if mtime > last_mtime {
                last_mtime = mtime;
                reload_counter += 1;
                let tmp_path = dylib_path.with_extension(format!("hot{reload_counter}.so"));
                if std::fs::copy(&dylib_path, &tmp_path).is_err() {
                    eprintln!("Failed to copy dylib to temp file");
                    return;
                }

                eprintln!("Reloading dylib...");

                // StateCache holds Box<dyn Any> with vtable pointers into the dylib.
                // Must drop these entries BEFORE dlclose, otherwise the vtables are
                // unmapped memory and Drop/downcast_mut segfaults.
                ctx.ui.state.clear();

                // Destroy old state and unload old dylib BEFORE loading new one.
                let old = unsafe { std::ptr::read(&hot_lib) };
                old.unload();

                // 2. Load new dylib and create fresh state
                match HotLib::load(&tmp_path, &example_name_for_closure) {
                    Some(new_lib) => {
                        unsafe { std::ptr::write(&mut hot_lib, new_lib) };
                        grf_initialized = false;
                        eprintln!("Reload complete.");
                    }
                    None => {
                        // Fallback: reload original dylib
                        eprintln!("Failed to load new dylib, falling back to original");
                        let fallback = HotLib::load(&dylib_path, &example_name_for_closure)
                            .expect("Failed to reload original dylib");
                        unsafe { std::ptr::write(&mut hot_lib, fallback) };
                    }
                }

                // Clean up previous temp file
                if reload_counter > 1 {
                    let prev = dylib_path.with_extension(format!("hot{}.so", reload_counter - 1));
                    let _ = std::fs::remove_file(prev);
                }
            }
        }

        if ctx.ui.has_grf_textures && !grf_initialized {
            let cache_ptr = &ctx.texture_size as *const &dyn Fn(&str) -> Option<(u32, u32)>;
            TEXTURE_SIZE_CACHE.set(cache_ptr as *const ());
            hot_lib.grf_init(texture_size_trampoline, ctx.item_resource_table);
            TEXTURE_SIZE_CACHE.set(std::ptr::null());
            grf_initialized = true;
        }

        hot_lib.build(&mut ctx.ui);
    })
    .with_grf_textures(texture_paths)
    .run();
}

std::thread_local! {
    static TEXTURE_SIZE_CACHE: std::cell::Cell<*const ()> = const { std::cell::Cell::new(std::ptr::null()) };
}

unsafe extern "C" fn texture_size_trampoline(
    name_ptr: *const u8,
    name_len: usize,
    out_w: *mut u32,
    out_h: *mut u32,
) -> bool {
    let name =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let cache_ptr = TEXTURE_SIZE_CACHE.get();
    if cache_ptr.is_null() {
        return false;
    }
    let closure_ref = unsafe { &**(cache_ptr as *const &dyn Fn(&str) -> Option<(u32, u32)>) };
    match closure_ref(name) {
        Some((w, h)) => {
            unsafe {
                *out_w = w;
                *out_h = h;
            }
            true
        }
        None => false,
    }
}
