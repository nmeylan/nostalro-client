The goal is to have a playable client to retrieve the classic ro experience (2004-2007)

It reuse many part of rust-ro.

If you seek for the best/most promising client implementation check-out [korangar](https://github.com/vE5li/korangar)

## Sprite Viewer

Standalone tool to browse and preview SPR/ACT sprites from GRF archives.

```bash
# Open with GRF file picker (scans current directory for .grf files)
cargo run --bin sprite-viewer

# Open a specific GRF
cargo run --bin sprite-viewer -- --grf data.grf

# Open a specific sprite directly
cargo run --bin sprite-viewer -- --grf data.grf --sprite "data/sprite/monsters/poring.spr"

# List all sprites in a GRF
cargo run --bin sprite-viewer -- --grf data.grf --list
```

**Controls:** arrows (direction/action), space (pause), `.`/`,` (step), scroll/+/- (zoom), B (background), Tab (sprite browser)

### Hot-reload

WGSL shader hot-reload works out of the box — edit `lib/renderer/src/shaders/sprite.wgsl` and changes apply instantly.

For Rust code hot-patching via [subsecond](https://crates.io/crates/subsecond), install the Dioxus CLI and `lld`, then run:

```bash
cargo install dioxus-cli@0.7.3
dx serve --hot-patch --features hot-reload --bin sprite-viewer -- --grf data/data.grf
```

Editing any workspace crate (e.g. `lib/renderer/src/sprite.rs`) hot-reloads in ~500ms without restarting.