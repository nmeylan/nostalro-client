use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    None,
    Once,
    Repeat(u16),
}

const HALF_WIDTH_MAX: f32 = 250.0;
const OPEN_SPEED: f32 = 500.0;
const SCROLL_SPEED: f32 = 50.0;
const TEXT_START_OFFSET: f32 = 500.0;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    Opening { half_width: f32 },
    Scrolling { offset_x: f32 },
}

#[derive(Debug)]
pub struct BannerState {
    queue: VecDeque<String>,
    current: Option<String>,
    phase: Phase,
}

pub struct BannerRender<'a> {
    pub text: &'a str,
    pub half_width: f32,
    pub text_offset_x: f32,
}

impl Default for BannerState {
    fn default() -> Self {
        Self::new()
    }
}

impl BannerState {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            phase: Phase::Idle,
        }
    }

    pub fn enqueue(&mut self, text: String, times: u16) {
        for _ in 0..times.max(1) {
            self.queue.push_back(text.clone());
        }
        if self.current.is_none() {
            self.start_next();
        }
    }

    fn start_next(&mut self) {
        match self.queue.pop_front() {
            Some(text) => {
                self.current = Some(text);
                self.phase = Phase::Opening { half_width: 0.0 };
            }
            None => {
                self.current = None;
                self.phase = Phase::Idle;
            }
        }
    }

    pub fn visible(&self) -> bool {
        self.current.is_some()
    }

    /// Advances the width-independent animation (open expansion, scroll offset).
    /// Scroll completion depends on text width, which only the caller can
    /// measure, so it is resolved separately via [`Self::current_scrolled_off`]
    /// and [`Self::advance`].
    pub fn tick(&mut self, dt: f32) {
        match &mut self.phase {
            Phase::Opening { half_width } => {
                *half_width += OPEN_SPEED * dt;
                if *half_width >= HALF_WIDTH_MAX {
                    self.phase = Phase::Scrolling {
                        offset_x: TEXT_START_OFFSET,
                    };
                }
            }
            Phase::Scrolling { offset_x } => {
                *offset_x -= SCROLL_SPEED * dt;
            }
            Phase::Idle => {}
        }
    }

    pub fn current_scrolled_off(&self, text_width: f32) -> bool {
        matches!(self.phase, Phase::Scrolling { offset_x } if offset_x <= -text_width)
    }

    pub fn advance(&mut self) {
        self.start_next();
    }

    pub fn render(&self) -> Option<BannerRender<'_>> {
        let text = self.current.as_deref()?;
        let (half_width, text_offset_x) = match self.phase {
            Phase::Opening { half_width } => (half_width, TEXT_START_OFFSET),
            Phase::Scrolling { offset_x } => (HALF_WIDTH_MAX, offset_x),
            Phase::Idle => return None,
        };
        Some(BannerRender {
            text,
            half_width,
            text_offset_x,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_advances_through_phases_and_drains() {
        let mut banner = BannerState::new();
        assert!(!banner.visible());

        banner.enqueue("Server maintenance".to_string(), 1);
        assert!(banner.visible());
        assert!(matches!(banner.phase, Phase::Opening { .. }));

        for _ in 0..10 {
            banner.tick(0.1);
        }
        assert!(matches!(banner.phase, Phase::Scrolling { .. }));

        let text_width = 200.0;
        for _ in 0..1000 {
            banner.tick(0.1);
            if banner.current_scrolled_off(text_width) {
                banner.advance();
                break;
            }
        }
        assert!(!banner.visible());
        assert!(matches!(banner.phase, Phase::Idle));
    }
}
