use crate::helper::dialog_container::DialogContainer;
use crate::helper::head_board;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_ui::draw::DrawCall;

pub use crate::helper::head_board::board_rect;

pub const VENDING_ICON_TEX: &str = "data/texture/유저인터페이스/shop.bmp";

/// Textures needed to render the board with GRF art (frame + bag icon).
pub fn grf_texture_paths() -> Vec<&'static str> {
    head_board::grf_texture_paths(&[VENDING_ICON_TEX])
}

/// Draw the vendor's shop board (frame + bag icon + name) into `draw_calls`.
pub fn draw_board(
    draw_calls: &mut Vec<DrawCall>,
    container: &DialogContainer,
    atlas: &FontAtlas,
    anchor_x: f32,
    anchor_y: f32,
    head_offset: f32,
    name: &str,
) {
    let label = head_board::truncate_to_width(name, head_board::label_max_width(), atlas);
    head_board::draw_board(
        draw_calls,
        container,
        atlas,
        anchor_x,
        anchor_y,
        head_offset,
        VENDING_ICON_TEX,
        &label,
    );
}
