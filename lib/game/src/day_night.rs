pub use ragnarok_effects::EFST_SKE;

const FADE_RATE_PER_SEC: f32 = 0.3;
const NIGHT_FLOOR: f32 = 0.5;

pub struct DayNightState {
    night: bool,
    diffuse_rg: [f32; 2],
    base_diffuse: [f32; 3],
    ambient: [f32; 3],
    dirty: bool,
}

impl Default for DayNightState {
    fn default() -> Self {
        Self {
            night: false,
            diffuse_rg: [1.0, 1.0],
            base_diffuse: [1.0, 1.0, 1.0],
            ambient: [0.3, 0.3, 0.3],
            dirty: false,
        }
    }
}

impl DayNightState {
    pub fn set_night(&mut self, night: bool) {
        self.night = night;
    }

    pub fn on_map_loaded(&mut self, diffuse: [f32; 3], ambient: [f32; 3]) {
        self.base_diffuse = diffuse;
        self.diffuse_rg = [diffuse[0], diffuse[1]];
        self.ambient = ambient;
        self.dirty = true;
    }

    pub fn reset(&mut self) {
        *self = Self {
            dirty: true,
            ..Self::default()
        };
    }

    pub fn tick(&mut self, delta: f32) {
        let rate = FADE_RATE_PER_SEC * delta;
        for c in 0..2 {
            let before = self.diffuse_rg[c];
            if self.night {
                if self.base_diffuse[c] > NIGHT_FLOOR {
                    self.diffuse_rg[c] = (self.diffuse_rg[c] - rate).max(NIGHT_FLOOR);
                }
            } else {
                self.diffuse_rg[c] = (self.diffuse_rg[c] + rate).min(self.base_diffuse[c]);
            }
            if self.diffuse_rg[c] != before {
                self.dirty = true;
            }
        }
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn world_diffuse(&self) -> [f32; 3] {
        [self.diffuse_rg[0], self.diffuse_rg[1], self.base_diffuse[2]]
    }

    pub fn sprite_light(&self) -> [f32; 3] {
        let now = self.world_diffuse();
        let mut factor = [1.0; 3];
        for c in 0..3 {
            let env_now = 1.0 - (1.0 - now[c]) * (1.0 - self.ambient[c]);
            let env_day = 1.0 - (1.0 - self.base_diffuse[c]) * (1.0 - self.ambient[c]);
            let denom = env_day + self.ambient[c];
            if denom > 1e-4 {
                factor[c] = (env_now + self.ambient[c]) / denom;
            }
        }
        factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_decays_rg_recovers_and_survives_map_change() {
        let mut s = DayNightState::default();
        s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
        assert!(s.take_dirty());

        s.set_night(true);
        for _ in 0..200 {
            s.tick(0.1);
        }
        let d = s.world_diffuse();
        assert_eq!(d[0], NIGHT_FLOOR);
        assert_eq!(d[1], NIGHT_FLOOR);
        assert_eq!(d[2], 0.9);
        assert!(s.sprite_light()[0] < 1.0);

        s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
        assert_eq!(s.world_diffuse()[0], 1.0);
        for _ in 0..200 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse()[0], NIGHT_FLOOR);

        s.set_night(false);
        for _ in 0..200 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), [1.0, 1.0, 0.9]);
        assert_eq!(s.sprite_light(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn dark_map_never_brightens_or_oscillates() {
        let mut s = DayNightState::default();
        s.on_map_loaded([0.4, 0.5, 0.4], [0.2, 0.2, 0.2]);
        let day = s.world_diffuse();

        s.set_night(true);
        for _ in 0..50 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), day);

        s.set_night(false);
        for _ in 0..50 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), day);
    }
}
