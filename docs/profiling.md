# Profiling and stress testing

## What the numbers mean

The frame loop is capped: `about_to_wait` re-arms a redraw every 16667 us
(`FRAME_INTERVAL` in [`client/src/main.rs`](../client/src/main.rs)), and the
surface presents with `Mailbox` when the adapter offers it, `Fifo` otherwise
([`lib/renderer/src/device.rs`](../lib/renderer/src/device.rs)). A healthy client
therefore reads 60 fps and stays there until it cannot keep up any more, so fps
tells us only whether we already fell off. 

**The number to watch is the duration of the `frame` scope against the 16.7 ms budget.**

## Puffin

### Build with the feature

```
cargo run --release --bin ragnarok-client --features profiling
```

`--features profiling` on the workspace root resolves through
`ragnarok-client/profiling` to `ragnarok-profiling/{profiling,puffin}`, and
feature unification turns the macros on for every crate in the build, not just
the binary. Without the feature `profile_scope!` and `profile_function!` expand
to nothing and `Profiler` is an empty stub, so Ctrl+P does nothing at all: a
client built without it cannot be profiled at runtime.

`--release` matters. The debug build spends its time in places release never
goes, and the shape of the timeline is wrong rather than merely slower.

### Install the viewer

```
cargo install puffin_viewer
```

`puffin_viewer` 0.23 is the version that speaks to `puffin_http` 0.17 and
`puffin` 0.20, which is what the workspace pins in the root `Cargo.toml`. A
viewer built against a different puffin will connect and then show nothing.

### Connect

Press **Ctrl+P** (chat must be closed!!) in the running client. That calls `Profiler::start`
([`lib/profiling/src/server.rs`](../lib/profiling/src/server.rs)), which turns
scopes on, binds a `puffin_http` server on `0.0.0.0:8585` and spawns
`puffin_viewer --url 127.0.0.1:8585`. 


### What is instrumented

`Profiler::new_frame` runs at the top of `RedrawRequested`, so one puffin frame
is one client frame. The scopes that exist today:

| Scope | Where |
| --- | --- |
| `frame` | the whole redraw, `client/src/main.rs` |
| `run_game_updates`, `ambient-effects`, `effects-update` | `client/src/game_updates/mod.rs` |
| `build_ui` | `client/src/main.rs` |
| `handle_ui_events` | `client/src/events/mod.rs` |
| `compute_render_list` | `client/src/scene/render_list.rs` |
| `compose_and_render` | `client/src/scene/mod.rs` |
| `render`, `render_into` | `lib/renderer/src/lib.rs` |
| `scene-opaque`, `ground`, `model`, `animated-models`, `skill-unit-models`, `gr2-models` | `lib/renderer/src/lib.rs` |
| `effect-behind`, `effect-sprite`, `effect-build`, `effect-dispatch` | `lib/renderer/src/lib.rs` |
| `sprite`, `silhouette`, `water`, `ui`, `cursor` | `lib/renderer/src/lib.rs` |
| `submit-scene`, `submit-ui` | `lib/renderer/src/lib.rs` |
| `effect_update`, `effect_collect` | per live effect, tagged with its id, `lib/renderer/src/effect/holder.rs` |
| `update` | model animation upload, `lib/renderer/src/model.rs` |
| `dispatch_packet` | `lib/network/src/handler.rs` |

`dispatch_packet` runs on the network thread, a dedicated thread holding a
current-thread tokio runtime (`client/src/main.rs`), and puffin lanes it
separately. A frame that looks cheap on the main lane can still be waiting on a
burst there. `effect_update` and `effect_collect` carry the effect id as scope
data, which is how we find the one effect that costs more than the other two
hundred.

Adding a scope is one line, and costs nothing in a build without the feature:

```rust
ragnarok_profiling::profile_scope!("name");
ragnarok_profiling::profile_function!();
```

### When scopes are not enough

Named scopes only show what we thought to name. For a more complete profiling, use `perf`.
The release profile carries no debug info, so ask for it on the build:

```
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release --bin ragnarok-client
perf record -g --call-graph dwarf target/release/ragnarok-client
perf report
```

## Cheap in-client measurements

Neither needs the profiling feature.

- `/show_fps` draws the smoothed frame rate.
- `/show_ping` draws network sync state, RTT and its average, the estimated
  server tick and our offset from it.

The trace toggles in `config.json` (`debug.trace_packet`, `debug.trace_effects`,
`debug.trace_input`, `debug.trace_texture_load`, documented in
[configuration.md](configuration.md)) print from hot paths. They answer what
happened, not how long it took, and they change the timing enough that a puffin
capture taken with them on is not comparable to one taken with them off.

- `@spawn poring 10000`
- `@killmonster`
- `@spawn poring 10000`


## Stress: effects

[`tools/src/stress.rs`](../tools/src/stress.rs) spawns a named set of effects at
random on-screen ground positions and re-seeds it every 0.5 s so the population
stays up. Both `viewer` and `effect-viewer` drive it:

```
cargo run --release --bin viewer -- --map prontera
cargo run --release --bin effect-viewer
```

Key **G** opens the set browser, Enter launches the highlighted set, **K** stops it.
The sets are plain data in `stress_sets()`: every non-noop effect once, 600 level
99 auras, and a caster AoE mix. A new set is a new `StressSet` entry there.

For now, the viewers hold no puffin server.

## Stress: a populated map

**WIP**: to do document how to use https://github.com/nmeylan/ragnarok-player-bots