# Changelog

## Unreleased
- Add weapon sprite rendering on player character (attached to body, synced animation)
- Add composite mode to sprite viewer: preview body+head+weapon together with `--job`, `--head`, `--weapon` flags
- Add weapon cycling in sprite viewer with `[`/`]` keys
- Add shadow under player character, grounding the entity visually on the map
- Fix character appearing below the ground surface on maps with terrain elevation
- Move sprite format conversion, map loading, and sprite loading out of client into game and formats crates
- Move walkability check, pathfinding, and cursor logic into game crate for reusability
- Refactor client main loop into smaller, focused methods for better readability
- Extract map coordinate system into reusable MapCoordinates module
- Add player character body sprite rendering with idle and walk animations
- Unify UI widget responses into a single type with hover, click, and focus tracking
- Add 3D model rendering for map objects (buildings, trees, props) with texture batching
- Add animated water surfaces with wave displacement and texture cycling
- Add camera positioning at player spawn point on map entry
- Add map change support (warp to new map loads terrain, models, and water)
- Add character selection screen with character list, stats display, and navigation to game map
- Add draggable windows — UI windows can now be moved by clicking and dragging their title bar
- Add server selection screen with GRF textures (win_service.bmp, btn_ok/btn_cancel) and fallback rendering
- Add support for opening older GRF archives (version 1.x)
- Add login screen with username/password fields, connect and exit buttons
- Add 2D UI rendering pipeline with font atlas and orthographic projection
- Add network connection from login screen (connect to login server, display errors)
- Add ground terrain rendering with textures and lightmap shading
- Add orbit camera with mouse controls (right-drag rotate, scroll zoom)
- Add wgpu-based rendering window
