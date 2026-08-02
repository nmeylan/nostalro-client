pub mod colors;
pub mod dialog_container;
pub mod dropdown;
pub mod fallback;
pub mod format;
pub mod head_board;
pub mod scrollbar;
pub mod window_chrome;

use ragnarok_ui::frame::CheckboxTextures;

pub const CHECKBOX: CheckboxTextures = CheckboxTextures {
    off: ragnarok_resources::ui::CHECKBOX_0,
    on: ragnarok_resources::ui::CHECKBOX_1,
};
