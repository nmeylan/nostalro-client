# Network Latency Compensation

How the original RO client avoids desync with the server, what we currently do, ideas for improvement, and how to test under simulated latency.

## How original client Does It

### Server Time Extrapolation

The client maintains an estimate of the server's current tick by remembering the last known
server tick and the local time when it was received:

```
GetServerTime() = last_server_tick + (local_now - local_when_received)
```

This is updated every 5 seconds when `ZC_NOTIFY_TIME` arrives in response to
`CZ_REQUEST_TIME`. The server tick is adjusted by half the measured round-trip time:

```
server_tick = packet.time + ping / 2
```

The assumption is that latency is symmetric (one-way delay = RTT/2).

### Where Server Time Is Used

- **Movement start**: `ZC_NOTIFY_PLAYERMOVE` carries `moveStartTime`. The client uses
  `GetServerTime()` to figure out how far along the path the entity should already be.
- **Entity moves**: `ZC_NOTIFY_MOVE` carries `moveStartTime` and `moveEndTime`. The client
  distributes waypoint timing to arrive at `moveEndTime`.
- **Actions**: `ZC_NOTIFY_ACT` carries `startTime` for attack/damage animations.

### Movement Corrections

When the server confirms a player move (`ZC_NOTIFY_PLAYERMOVE`):

1. Start position is taken from the player's **current rendered position** (not the packet's
   start), preventing visual teleporting.
2. If the local server time estimate is behind `moveStartTime`, it's snapped forward.
3. Path waypoint times are distributed linearly via `FixPathTime()` to match the server's
   expected arrival time.

### Summary of Mechanisms

| Mechanism | What it does |
|---|---|
| 5s time sync + half-RTT | Keeps client clock within ~ping/2 of server |
| Forward time snap on move | Local clock never falls behind server |
| Start from rendered position | No visual teleport on move confirmation |
| `FixPathTime()` | Distributes timing error across all waypoints |
| 500ms + ack-flag throttle | Prevents move spam; waits for server confirmation |

## What We Currently Do

Full server-time synchronization is implemented:

- **Time sync**: `CZ_REQUEST_TIME` is sent every 10s; `ZC_NOTIFY_TIME` is handled and feeds
  `ServerTimeClock` (`lib/game/src/server_time.rs`), which tracks the server-tick offset, last RTT,
  and an RTT EMA, adjusting by half-RTT. `enhanced_lag_compensation` switches last-RTT vs EMA.
- **Server timestamps drive timing**: every move (`ZC_NOTIFY_PLAYERMOVE` / `ZC_NOTIFY_MOVE`) and
  action/skill (`ZC_NOTIFY_ACT` / `ZC_NOTIFY_SKILL`) is anchored to its server `start_time` via
  `ServerTimeClock::server_to_local_secs_clamped`, so entities don't start late by a round-trip and
  scheduled hits/animations align to when the server says things happened.
- **Forward-snap guard**: `observe_server_tick` advances the local estimate so it never trails the
  server; the clamped conversion prevents a start time landing in the future.
- **Convergence blending**: a position correction snaps the *logical* position to the grid while the
  *rendered* sprite eases in over a short window (`MovementState::correct_to_cell` + `decay_correction`),
  so re-paths don't visibly teleport.
- **Move throttling**: continuous walk gates on a 0.5s cooldown plus a server-ack flag.
- **Diagnostics**: F10 (or `debug_overlay` in config) shows sync state / RTT / offset.

### Not implemented (intentionally)

- Velocity-based dead reckoning between updates: RO moves are fully specified (start→dest + start
  time), so interpolation already covers the gap; only convergence blending was needed.

## Possible Improvements

### 1. Server Time Offset Tracking (present in original client)

Track the offset between local time and server time:

```
server_time_offset = server_tick - (local_elapsed_ms + ping/2)
```

Update this every time `ZC_NOTIFY_TIME` arrives. To get the estimated server time at any
moment:

```
estimated_server_tick = local_elapsed_ms + server_time_offset
```

This is the minimum viable fix -- it lets us use server timestamps from move/action packets
to position entities correctly relative to when the server says things happened.

**Where to store it**: game state (not network layer), since it's needed by movement and
action systems.

### 2. Use Server Timestamps for Movement (present in original client)

When receiving `ZC_NOTIFY_PLAYERMOVE` or `ZC_NOTIFY_MOVE`:

- Convert `moveStartTime` to local time via the offset.
- If the move started in the past (which it always does due to latency), the entity is already
  partway along the path. Start interpolation from the correct position.
- For `ZC_NOTIFY_MOVE` (other entities), also use `moveEndTime` to distribute waypoint timing
  like original client's `FixPathTime()`.

This matters most for other entities: without it, every entity appears to start moving late by
one round-trip.

### 3. RTT Tracking for Adaptive Compensation (not in original client)

Keep a running average of recent RTT values (e.g. exponential moving average). This can:

- Improve the half-RTT estimate used in time sync (using average rather than last-sample).
- Inform move request throttling (longer RTT = allow more time before resending).
- Be displayed in a debug overlay for diagnostics.

### 4. Dead Reckoning for Other Entities (not in original client)

Between server updates, extrapolate entity positions based on their last known velocity and
direction. When a new update arrives, blend toward the corrected position instead of snapping.
This is more complex but handles high-latency scenarios better than pure interpolation.

### 5. Move Request Throttling (present in original client)

Like original client's 500ms + ack gate: don't send a new `CZ_REQUEST_MOVE` until either the server
confirms the previous one or 500ms have elapsed. This prevents flooding the server and reduces
the chance of desynced paths.

## Simulating Latency

To test lag compensation without a remote server, set `debug_network_delay_ms` in `config.json`.
This is **implemented** in `lib/network/src/lib.rs`: it delays both received events and outbound
packets, so measured RTT reflects a full round trip (~2 × the configured delay). Zero-cost when 0.

### Implementation: Configurable Network Delay

Add a delay parameter to the network loop that holds packets (both send and receive) for a
configurable duration. This simulates real-world latency including its effects on time sync.

**Config** (`config.json`):
```json
{
    "debug_network_delay_ms": 0
}
```

**Network loop change** (`lib/network/src/lib.rs`):

Add a delay queue for received packets. Instead of dispatching immediately, push packets into
a `VecDeque<(Instant, Vec<Box<dyn Packet>>)>` with a release time of `now + delay`. Each
loop iteration, drain packets whose release time has passed.

For send delay, use `tokio::time::sleep` before actually writing to the socket.

```rust
// Receive side: delay before dispatching
struct DelayedPackets {
    queue: VecDeque<(Instant, Vec<Box<dyn Packet>>)>,
    delay: Duration,
}

impl DelayedPackets {
    fn push(&mut self, packets: Vec<Box<dyn Packet>>) {
        self.queue.push_back((Instant::now() + self.delay, packets));
    }

    fn drain_ready(&mut self) -> Vec<Box<dyn Packet>> {
        let now = Instant::now();
        let mut ready = Vec::new();
        while self.queue.front().is_some_and(|(t, _)| *t <= now) {
            ready.extend(self.queue.pop_front().unwrap().1);
        }
        ready
    }
}
```

This approach:
- Simulates both directions of latency (send + receive delay = full RTT).
- Affects keepalive timing realistically (server sees delayed pings).
- Exercises the time sync code path under stress.
- Is zero-cost when `debug_network_delay_ms = 0`.

### Alternative: OS-Level Simulation

On Linux, `tc netem` can add delay to loopback traffic:

```bash
# Add 100ms delay to loopback
sudo tc qdisc add dev lo root netem delay 100ms

# Add 100ms delay with 20ms jitter (normal distribution)
sudo tc qdisc add dev lo root netem delay 100ms 20ms distribution normal

# Remove
sudo tc qdisc del dev lo root
```

This is more realistic (affects TCP behavior, retransmits, etc.) but requires root and affects
all loopback traffic. Best used for final validation after the in-app simulation works.

### Test Scenarios

| Scenario | Delay | Jitter | What to verify |
|---|---|---|---|
| Baseline | 0ms | 0ms | Everything works as before |
| LAN | 5ms | 1ms | No visible difference from baseline |
| Broadband | 50ms | 10ms | Entities start moving slightly late but catch up |
| High latency | 150ms | 30ms | Player movement feels delayed but stays in sync |
| Extreme | 300ms | 50ms | Stress test; entity positions should converge |
| Asymmetric | 200ms send, 50ms recv | - | Half-RTT assumption breaks down |

For each scenario, verify:
1. Player position matches server position after movement completes.
2. Other entities don't visually teleport on path updates.
3. Time sync offset converges and stays stable.
4. Keepalive packets maintain the connection.
5. Actions (sit, attack) happen at roughly the right time relative to movement.
