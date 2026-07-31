//! `showdigit`: a large digit clock across the top of the screen, drawn from
//! `timefont` where actions 0..=9 are the digits and action 10 is the separator.

/// Action index of the separator glyph in `timefont`.
pub const SEPARATOR_ACTION: usize = 10;

const DIGIT_STEP: f32 = 40.0;
const TOP_MARGIN: f32 = 40.0;
/// A plain value stays up this long; the counters run until they stop.
const STATIC_DURATION: f32 = 5.0;
/// Type 3 steps its two digits down twice a second.
const FAST_TICK: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowDigitMode {
    /// Value held for five seconds.
    Static,
    /// Counts up from the value, one per second.
    CountUp,
    /// Counts down from the value, one per second, wrapping past zero.
    CountDown,
    /// Two digits counting down twice a second, stopping at zero.
    FastCountDown,
}

impl ShowDigitMode {
    pub fn from_packet(atype: u8) -> Option<Self> {
        match atype {
            0 => Some(ShowDigitMode::Static),
            1 => Some(ShowDigitMode::CountUp),
            2 => Some(ShowDigitMode::CountDown),
            3 => Some(ShowDigitMode::FastCountDown),
            _ => None,
        }
    }
}

pub struct DigitQuad {
    pub action: usize,
    pub x: f32,
    pub y: f32,
}

pub struct ShowDigitClock {
    mode: ShowDigitMode,
    start: i64,
    elapsed: f32,
}

impl ShowDigitClock {
    pub fn new(mode: ShowDigitMode, value: i32) -> Self {
        Self {
            mode,
            start: value.max(0) as i64,
            elapsed: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    pub fn is_finished(&self) -> bool {
        match self.mode {
            ShowDigitMode::Static => self.elapsed >= STATIC_DURATION,
            ShowDigitMode::CountUp | ShowDigitMode::CountDown => false,
            ShowDigitMode::FastCountDown => self.value() == 0,
        }
    }

    /// The number on screen: seconds for every mode but [`ShowDigitMode::FastCountDown`],
    /// which counts bare units.
    fn value(&self) -> i64 {
        match self.mode {
            ShowDigitMode::Static => self.start,
            ShowDigitMode::CountUp => self.start + self.elapsed as i64,
            // The original keeps subtracting into unsigned wraparound rather
            // than stopping at zero.
            ShowDigitMode::CountDown => {
                let remaining = self.start - self.elapsed as i64;
                if remaining >= 0 {
                    remaining
                } else {
                    (remaining as u32) as i64
                }
            }
            ShowDigitMode::FastCountDown => (self.start - (self.elapsed / FAST_TICK) as i64).max(0),
        }
    }

    pub fn quads(&self, screen_w: f32) -> Vec<DigitQuad> {
        let groups = match self.mode {
            ShowDigitMode::FastCountDown => vec![self.value().clamp(0, 99)],
            _ => time_groups(self.value()),
        };

        // Two glyphs per group, with a separator between neighbouring groups.
        let glyph_count = groups.len() * 2 + groups.len().saturating_sub(1);
        let mut quads = Vec::with_capacity(glyph_count);
        let mut x = screen_w / 2.0 - (glyph_count as f32 * DIGIT_STEP) / 2.0;
        for (i, group) in groups.iter().rev().enumerate() {
            if i > 0 {
                quads.push(DigitQuad {
                    action: SEPARATOR_ACTION,
                    x,
                    y: TOP_MARGIN,
                });
                x += DIGIT_STEP;
            }
            for digit in [group / 10, group % 10] {
                quads.push(DigitQuad {
                    action: digit as usize,
                    x,
                    y: TOP_MARGIN,
                });
                x += DIGIT_STEP;
            }
        }
        quads
    }
}

/// Splits a second count into the two-digit groups the clock shows, least
/// significant first: seconds, then minutes, hours and days as each becomes
/// non-zero.
fn time_groups(seconds: i64) -> Vec<i64> {
    let seconds = seconds.max(0);
    let mut groups = vec![seconds % 60];
    let minutes = seconds / 60;
    if minutes > 0 {
        groups.push(minutes % 60);
        let hours = minutes / 60;
        if hours > 0 {
            groups.push(hours % 24);
            let days = hours / 24;
            if days > 0 {
                groups.push(days % 100);
            }
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_static_value_reads_as_a_clock_and_expires_after_five_seconds() {
        let mut clock = ShowDigitClock::new(ShowDigitMode::Static, 3661);
        let laid_out: Vec<(usize, f32)> =
            clock.quads(800.0).iter().map(|q| (q.action, q.x)).collect();

        assert_eq!(
            laid_out,
            vec![
                (0, 240.0),
                (1, 280.0),
                (SEPARATOR_ACTION, 320.0),
                (0, 360.0),
                (1, 400.0),
                (SEPARATOR_ACTION, 440.0),
                (0, 480.0),
                (1, 520.0),
            ],
            "3661s must read 01:01:01, centred"
        );

        clock.update(4.9);
        assert!(!clock.is_finished());
        clock.update(0.2);
        assert!(clock.is_finished());
    }

    #[test]
    fn the_fast_counter_steps_twice_a_second_and_stops_at_zero() {
        let mut clock = ShowDigitClock::new(ShowDigitMode::FastCountDown, 3);
        let digits = |c: &ShowDigitClock| -> Vec<usize> {
            c.quads(800.0).iter().map(|q| q.action).collect()
        };

        assert_eq!(digits(&clock), vec![0, 3]);
        clock.update(0.5);
        assert_eq!(digits(&clock), vec![0, 2]);
        clock.update(1.0);
        assert_eq!(digits(&clock), vec![0, 0]);
        assert!(clock.is_finished());
    }

    #[test]
    fn the_counters_run_in_both_directions() {
        let mut up = ShowDigitClock::new(ShowDigitMode::CountUp, 58);
        up.update(3.0);
        assert_eq!(up.quads(800.0).len(), 5, "61s spills into a minutes group");

        let mut down = ShowDigitClock::new(ShowDigitMode::CountDown, 10);
        down.update(4.0);
        assert_eq!(
            down.quads(800.0)
                .iter()
                .map(|q| q.action)
                .collect::<Vec<_>>(),
            vec![0, 6]
        );
        assert!(!down.is_finished(), "a countdown never stops on its own");
    }
}
