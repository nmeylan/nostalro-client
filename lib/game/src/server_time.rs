pub struct ServerTimeClock {
    server_tick_at_sync: u32,
    local_ms_at_sync: u32,
    last_rtt: u32,
    synced: bool,

    rtt_average: f32,
    rtt_sample_count: u32,
}

impl Default for ServerTimeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerTimeClock {
    pub fn new() -> Self {
        Self {
            server_tick_at_sync: 0,
            local_ms_at_sync: 0,
            last_rtt: 0,
            synced: false,
            rtt_average: 0.0,
            rtt_sample_count: 0,
        }
    }

    pub fn on_server_tick(&mut self, server_tick: u32, local_now_ms: u32, local_send_time_ms: u32) {
        let rtt = local_now_ms.saturating_sub(local_send_time_ms);
        if rtt < 1000 {
            self.last_rtt = rtt;
        }

        if self.rtt_sample_count == 0 {
            self.rtt_average = rtt as f32;
        } else {
            self.rtt_average = self.rtt_average * 0.8 + rtt as f32 * 0.2;
        }
        self.rtt_sample_count += 1;

        let half_rtt = self.last_rtt / 2;
        self.server_tick_at_sync = server_tick.wrapping_add(half_rtt);
        self.local_ms_at_sync = local_now_ms;
        self.synced = true;
    }

    pub fn on_server_tick_enhanced(
        &mut self,
        server_tick: u32,
        local_now_ms: u32,
        local_send_time_ms: u32,
    ) {
        let rtt = local_now_ms.saturating_sub(local_send_time_ms);
        if rtt < 1000 {
            self.last_rtt = rtt;
        }

        if self.rtt_sample_count == 0 {
            self.rtt_average = rtt as f32;
        } else {
            self.rtt_average = self.rtt_average * 0.8 + rtt as f32 * 0.2;
        }
        self.rtt_sample_count += 1;

        let half_rtt = (self.rtt_average / 2.0) as u32;
        self.server_tick_at_sync = server_tick.wrapping_add(half_rtt);
        self.local_ms_at_sync = local_now_ms;
        self.synced = true;
    }

    pub fn observe_server_tick(&mut self, server_tick: u32, local_now_ms: u32) {
        if !self.synced {
            return;
        }
        let estimated = self.estimated_server_tick(local_now_ms);
        let ahead = server_tick.wrapping_sub(estimated);
        if ahead > 0 && ahead < 1000 {
            self.server_tick_at_sync = self.server_tick_at_sync.wrapping_add(ahead);
        }
    }

    pub fn estimated_server_tick(&self, local_elapsed_ms: u32) -> u32 {
        if !self.synced {
            return local_elapsed_ms;
        }
        let passed = local_elapsed_ms.wrapping_sub(self.local_ms_at_sync);
        self.server_tick_at_sync.wrapping_add(passed)
    }

    pub fn server_to_local_secs(&self, server_tick: u32, local_elapsed_ms: u32) -> f32 {
        if !self.synced {
            return local_elapsed_ms as f32 / 1000.0;
        }
        let estimated = self.estimated_server_tick(local_elapsed_ms);
        let diff = estimated.wrapping_sub(server_tick) as i32;
        let local_now_secs = local_elapsed_ms as f32 / 1000.0;
        local_now_secs - diff as f32 / 1000.0
    }

    pub fn server_to_local_secs_clamped(&self, server_tick: u32, local_elapsed_ms: u32) -> f32 {
        let converted = self.server_to_local_secs(server_tick, local_elapsed_ms);
        let local_now_secs = local_elapsed_ms as f32 / 1000.0;
        converted.min(local_now_secs)
    }

    pub fn is_synced(&self) -> bool {
        self.synced
    }

    pub fn rtt(&self) -> u32 {
        self.last_rtt
    }

    pub fn rtt_avg(&self) -> f32 {
        self.rtt_average
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_not_synced() {
        let clock = ServerTimeClock::new();
        assert!(!clock.is_synced());
        assert_eq!(clock.rtt(), 0);
    }

    #[test]
    fn sync_computes_rtt_and_adjusts_server_tick() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        assert!(clock.is_synced());
        assert_eq!(clock.rtt(), 100);
        assert_eq!(clock.server_tick_at_sync, 5050);
    }

    #[test]
    fn estimated_server_tick_extrapolates() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        let est = clock.estimated_server_tick(1300);
        assert_eq!(est, 5050 + 200);
    }

    #[test]
    fn server_to_local_converts_past_tick() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        let local_secs = clock.server_to_local_secs(5100, 1200);
        assert!((local_secs - 1.15).abs() < 0.001, "got {local_secs}");
    }

    #[test]
    fn unsyced_falls_back_to_local_time() {
        let clock = ServerTimeClock::new();
        let est = clock.estimated_server_tick(5000);
        assert_eq!(est, 5000);
        let secs = clock.server_to_local_secs(3000, 5000);
        assert!((secs - 5.0).abs() < 0.001);
    }

    #[test]
    fn rtt_above_1000_is_ignored() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(1000, 200, 100);
        assert_eq!(clock.rtt(), 100);
        clock.on_server_tick(2000, 2300, 300);
        assert_eq!(clock.rtt(), 100);
    }

    #[test]
    fn ema_converges_toward_recent_values() {
        let mut clock = ServerTimeClock::new();
        for i in 0..5u32 {
            let send = i * 1000;
            let recv = send + 100;
            clock.on_server_tick(i * 1000, recv, send);
        }
        assert!((clock.rtt_avg() - 100.0).abs() < 1.0);

        for i in 5..15u32 {
            let send = i * 1000;
            let recv = send + 200;
            clock.on_server_tick(i * 1000, recv, send);
        }
        assert!(clock.rtt_avg() > 180.0, "got {}", clock.rtt_avg());
    }

    #[test]
    fn enhanced_sync_uses_ema_for_half_rtt() {
        let mut clock = ServerTimeClock::new();
        for i in 0..5u32 {
            clock.on_server_tick_enhanced(i * 1000, i * 1000 + 100, i * 1000);
        }
        let half_ema = (clock.rtt_avg() / 2.0) as u32;
        assert_eq!(clock.server_tick_at_sync, 4000 + half_ema);
    }

    #[test]
    fn observe_forward_snaps_estimate_never_trails() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        clock.observe_server_tick(5250, 1200);
        assert!(clock.estimated_server_tick(1200) >= 5250);
        let before = clock.estimated_server_tick(1200);
        clock.observe_server_tick(5000, 1200);
        assert_eq!(clock.estimated_server_tick(1200), before);
    }

    #[test]
    fn clamped_conversion_never_returns_future() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        let local = clock.server_to_local_secs(5350, 1200);
        assert!(
            local > 1.2,
            "raw conversion should be in the future, got {local}"
        );
        let clamped = clock.server_to_local_secs_clamped(5350, 1200);
        assert!(
            (clamped - 1.2).abs() < 0.001,
            "clamped to now, got {clamped}"
        );
    }

    #[test]
    fn reset_clears_all_state() {
        let mut clock = ServerTimeClock::new();
        clock.on_server_tick(5000, 1100, 1000);
        assert!(clock.is_synced());
        clock.reset();
        assert!(!clock.is_synced());
        assert_eq!(clock.rtt(), 0);
        assert_eq!(clock.rtt_sample_count, 0);
    }
}
