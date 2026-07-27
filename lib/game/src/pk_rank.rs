//! Bottom-right PvP rank fraction, drawn from `rankfont` where actions 0..=9
//! are the digits and action 10 is the slash.

/// Action index of the slash glyph in `rankfont`.
pub const SLASH_ACTION: usize = 10;

pub struct RankHudQuad {
    pub action: usize,
    pub x: f32,
    pub y: f32,
}

const DIGIT_STEP: f32 = 25.0;
const RIGHT_MARGIN: f32 = 80.0;

fn digits(mut value: i32) -> (Vec<usize>, usize) {
    let count = match value {
        v if v > 999 => 4,
        v if v > 99 => 3,
        v if v > 9 => 2,
        _ => 1,
    };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push((value % 10) as usize);
        value /= 10;
    }
    (out, count)
}

pub fn pk_rank_hud_quads(rank: i32, total: i32, screen_w: f32, screen_h: f32) -> Vec<RankHudQuad> {
    if rank <= 0 || total <= 0 {
        return Vec::new();
    }
    let x0 = screen_w - RIGHT_MARGIN;
    let mut quads = Vec::new();

    let (total_digits, total_count) = digits(total.min(9999));
    for (i, &d) in total_digits.iter().enumerate() {
        quads.push(RankHudQuad {
            action: d,
            x: x0 + DIGIT_STEP * total_count as f32 - DIGIT_STEP * i as f32,
            y: screen_h - 30.0,
        });
    }

    quads.push(RankHudQuad {
        action: SLASH_ACTION,
        x: x0,
        y: screen_h - 40.0,
    });

    let (rank_digits, _) = digits(rank);
    for (i, &d) in rank_digits.iter().enumerate() {
        quads.push(RankHudQuad {
            action: d,
            x: x0 - DIGIT_STEP - DIGIT_STEP * i as f32,
            y: screen_h - 50.0,
        });
    }

    quads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_38_of_130_stacks_a_fraction_in_the_bottom_right() {
        let quads = pk_rank_hud_quads(38, 130, 800.0, 600.0);
        let laid_out: Vec<(usize, f32, f32)> = quads.iter().map(|q| (q.action, q.x, q.y)).collect();
        assert_eq!(
            laid_out,
            vec![
                (0, 795.0, 570.0),
                (3, 770.0, 570.0),
                (1, 745.0, 570.0),
                (SLASH_ACTION, 720.0, 560.0),
                (8, 695.0, 550.0),
                (3, 670.0, 550.0),
            ]
        );
    }

    #[test]
    fn unranked_or_empty_map_draws_nothing() {
        assert!(pk_rank_hud_quads(0, 130, 800.0, 600.0).is_empty());
        assert!(pk_rank_hud_quads(38, 0, 800.0, 600.0).is_empty());
        assert!(pk_rank_hud_quads(-1, 130, 800.0, 600.0).is_empty());
    }
}
