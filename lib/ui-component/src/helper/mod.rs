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
    off: "data/texture/유저인터페이스/checkbox_0.bmp",
    on: "data/texture/유저인터페이스/checkbox_1.bmp",
};
