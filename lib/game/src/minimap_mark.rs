use std::collections::HashMap;

/// `ZC_COMPASS` action: the server either shows a mark for 15 s, shows it until
/// it clears it, or clears it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkAction {
    ShowTimed,
    Show,
    Remove,
}

impl MarkAction {
    pub fn from_packet(atype: i32) -> Option<Self> {
        match atype {
            0 => Some(MarkAction::ShowTimed),
            1 => Some(MarkAction::Show),
            2 => Some(MarkAction::Remove),
            _ => None,
        }
    }
}

const TIMED_SECS: f32 = 15.0;

#[derive(Clone, Copy, Debug)]
pub struct MinimapMarkEntry {
    pub x: u16,
    pub y: u16,
    /// `0xRRGGBB`, as the server sends it.
    pub color: u32,
    expires_at: Option<f32>,
}

impl MinimapMarkEntry {
    pub fn rgb(&self) -> [f32; 3] {
        [
            ((self.color >> 16) & 0xff) as f32 / 255.0,
            ((self.color >> 8) & 0xff) as f32 / 255.0,
            (self.color & 0xff) as f32 / 255.0,
        ]
    }
}

/// Marks the server put on the minimap, keyed by the mark number it owns. The
/// town guide places one per facility and clears them by number, so several are
/// live at once and only the number identifies them.
#[derive(Default)]
pub struct MinimapMarks {
    marks: HashMap<u8, MinimapMarkEntry>,
}

impl MinimapMarks {
    pub fn apply(&mut self, id: u8, action: MarkAction, x: u16, y: u16, color: u32, now: f32) {
        match action {
            // A remove carries no meaningful position — the guide sends 1,1.
            MarkAction::Remove => {
                self.marks.remove(&id);
            }
            MarkAction::Show | MarkAction::ShowTimed => {
                let expires_at = matches!(action, MarkAction::ShowTimed).then(|| now + TIMED_SECS);
                self.marks.insert(
                    id,
                    MinimapMarkEntry {
                        x,
                        y,
                        color,
                        expires_at,
                    },
                );
            }
        }
    }

    pub fn prune(&mut self, now: f32) {
        self.marks
            .retain(|_, m| m.expires_at.is_none_or(|end| now < end));
    }

    pub fn iter(&self) -> impl Iterator<Item = &MinimapMarkEntry> {
        self.marks.values()
    }

    pub fn clear(&mut self) {
        self.marks.clear();
    }

    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_marks_expire_and_removals_are_by_id() {
        let mut marks = MinimapMarks::default();
        marks.apply(0, MarkAction::ShowTimed, 134, 221, 0xFF0000, 0.0);
        marks.apply(1, MarkAction::Show, 175, 220, 0x0A82FF, 0.0);
        assert_eq!(marks.len(), 2);

        marks.prune(14.9);
        assert_eq!(marks.len(), 2);
        marks.prune(15.1);
        assert_eq!(marks.len(), 1, "the timed mark should be gone");

        // The guide's clear form sends x = y = 1; only the id may be used.
        marks.apply(1, MarkAction::Remove, 1, 1, 0xFFFF00, 20.0);
        assert!(marks.is_empty());
    }

    #[test]
    fn reusing_an_id_replaces_the_mark_and_its_colour() {
        let mut marks = MinimapMarks::default();
        marks.apply(3, MarkAction::Show, 10, 10, 0xFF0000, 0.0);
        marks.apply(3, MarkAction::Show, 20, 30, 0x00FF00, 1.0);

        let mark = marks.iter().next().copied().expect("one mark");
        assert_eq!((mark.x, mark.y), (20, 30));
        assert_eq!(mark.rgb(), [0.0, 1.0, 0.0]);
        assert_eq!(marks.len(), 1);
    }
}
