use std::collections::HashMap;
use std::ffi::c_char;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::scancode::PhysicalKeyExtScancode;
use xkbcommon_dl::{
    XkbCommon, xkb_context_flags, xkb_keymap, xkb_keymap_compile_flags, xkb_keymap_format,
    xkb_rule_names, xkbcommon_option,
};

/// X11/Wayland keycodes are evdev scancodes offset by 8.
const KEYCODE_OFFSET: u32 = 8;

pub fn labels_from_keymap_string(keymap: &str) -> Option<HashMap<KeyCode, String>> {
    let source = std::ffi::CString::new(keymap).ok()?;
    with_context(|xkb, context| unsafe {
        (xkb.xkb_keymap_new_from_string)(
            context,
            source.as_ptr(),
            xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
            xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
        )
    })
}

pub fn labels_from_rule_names(names: &[String; 5]) -> Option<HashMap<KeyCode, String>> {
    let fields: Vec<std::ffi::CString> = names
        .iter()
        .map(|name| std::ffi::CString::new(name.as_str()).unwrap_or_default())
        .collect();
    let names = xkb_rule_names {
        rules: fields[0].as_ptr(),
        model: fields[1].as_ptr(),
        layout: fields[2].as_ptr(),
        variant: fields[3].as_ptr(),
        options: fields[4].as_ptr(),
    };
    with_context(|xkb, context| unsafe {
        (xkb.xkb_keymap_new_from_names)(
            context,
            &names,
            xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
        )
    })
}

fn with_context(
    compile: impl FnOnce(&XkbCommon, *mut xkbcommon_dl::xkb_context) -> *mut xkb_keymap,
) -> Option<HashMap<KeyCode, String>> {
    let xkb = xkbcommon_option()?;
    unsafe {
        let context = (xkb.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS);
        if context.is_null() {
            return None;
        }
        let keymap = compile(xkb, context);
        let labels = if keymap.is_null() {
            None
        } else {
            let labels = labels_from_keymap(xkb, keymap);
            (xkb.xkb_keymap_unref)(keymap);
            labels
        };
        (xkb.xkb_context_unref)(context);
        labels
    }
}

/// Reads every key of the keymap with no modifier applied, so the label is what
/// the keycap shows unshifted.
unsafe fn labels_from_keymap(
    xkb: &XkbCommon,
    keymap: *mut xkb_keymap,
) -> Option<HashMap<KeyCode, String>> {
    unsafe {
        let state = (xkb.xkb_state_new)(keymap);
        if state.is_null() {
            return None;
        }
        let mut labels = HashMap::new();
        let mut buf = [0 as c_char; 16];
        for keycode in (xkb.xkb_keymap_min_keycode)(keymap)..=(xkb.xkb_keymap_max_keycode)(keymap) {
            let PhysicalKey::Code(code) =
                PhysicalKey::from_scancode(keycode.saturating_sub(KEYCODE_OFFSET))
            else {
                continue;
            };
            let written =
                (xkb.xkb_state_key_get_utf8)(state, keycode, buf.as_mut_ptr(), buf.len());
            if written <= 0 {
                continue;
            }
            let bytes: Vec<u8> = buf[..written as usize].iter().map(|b| *b as u8).collect();
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            if text.chars().any(|c| c.is_control()) {
                continue;
            }
            labels.insert(code, text);
        }
        (xkb.xkb_state_unref)(state);
        Some(labels)
    }
}
