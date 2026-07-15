use crate::helper::dialog_container::DialogContainer;
use crate::helper::head_board;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_ui::draw::DrawCall;

pub use crate::helper::head_board::board_rect;

pub const CHAT_OPEN_TEX: &str = "data/texture/유저인터페이스/chat_open.bmp";
pub const CHAT_CLOSE_TEX: &str = "data/texture/유저인터페이스/chat_close.bmp";

/// Textures needed to render the board with GRF art (frame + open/close icon).
pub fn grf_texture_paths() -> Vec<&'static str> {
    head_board::grf_texture_paths(&[CHAT_OPEN_TEX, CHAT_CLOSE_TEX])
}

fn icon_texture(atype: u8) -> &'static str {
    if atype == 0 {
        CHAT_CLOSE_TEX
    } else {
        CHAT_OPEN_TEX
    }
}

fn count_suffix(atype: u8, cur: i16, max: i16) -> String {
    if atype == 3 {
        String::new()
    } else {
        format!(" ({cur}/{max})")
    }
}

/// Draw the room board (frame + open/close icon + title/count) into `draw_calls`.
/// The title is pixel-truncated to the board width, keeping the count suffix
/// fully visible.
#[allow(clippy::too_many_arguments)]
pub fn draw_board(
    draw_calls: &mut Vec<DrawCall>,
    container: &DialogContainer,
    atlas: &FontAtlas,
    anchor_x: f32,
    anchor_y: f32,
    head_offset: f32,
    atype: u8,
    title: &str,
    cur: i16,
    max: i16,
) {
    let suffix = count_suffix(atype, cur, max);
    let title_width = (head_board::label_max_width() - atlas.measure_text(&suffix)).max(0.0);
    let title = head_board::truncate_to_width(title, title_width, atlas);
    head_board::draw_board(
        draw_calls,
        container,
        atlas,
        anchor_x,
        anchor_y,
        head_offset,
        icon_texture(atype),
        &format!("{title}{suffix}"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_and_count_suffix_follow_room_type() {
        assert_eq!(icon_texture(2), CHAT_OPEN_TEX);
        assert_eq!(icon_texture(0), CHAT_CLOSE_TEX);
        assert_eq!(count_suffix(2, 3, 20), " (3/20)");
        assert_eq!(count_suffix(3, 3, 20), "");
    }
}
