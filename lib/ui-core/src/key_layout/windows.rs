use std::collections::HashMap;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::scancode::PhysicalKeyExtScancode;

const MAPVK_VK_TO_CHAR: u32 = 2;
const MAPVK_VSC_TO_VK_EX: u32 = 3;
/// `MapVirtualKeyExW` sets the top bit for a dead key; the character below it is
/// still the one on the keycap.
const DEAD_KEY_FLAG: u32 = 0x8000_0000;

type Hkl = *mut std::ffi::c_void;

unsafe extern "system" {
    fn GetKeyboardLayout(thread_id: u32) -> Hkl;
    fn MapVirtualKeyExW(code: u32, map_type: u32, layout: Hkl) -> u32;
}

pub fn labels() -> HashMap<KeyCode, String> {
    let layout = unsafe { GetKeyboardLayout(0) };
    let mut labels = HashMap::new();
    for scancode in 0x00u32..=0xff {
        let PhysicalKey::Code(code) = PhysicalKey::from_scancode(scancode) else {
            continue;
        };
        let virtual_key = unsafe { MapVirtualKeyExW(scancode, MAPVK_VSC_TO_VK_EX, layout) };
        if virtual_key == 0 {
            continue;
        }
        let mapped = unsafe { MapVirtualKeyExW(virtual_key, MAPVK_VK_TO_CHAR, layout) };
        let Some(character) = char::from_u32(mapped & !DEAD_KEY_FLAG) else {
            continue;
        };
        if character.is_control() {
            continue;
        }
        labels.insert(code, character.to_string());
    }
    labels
}
