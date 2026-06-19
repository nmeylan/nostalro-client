The goal is to have a playable client to retrieve the classic ro experience (2004-2008)

It reuses many part of [rust-ro](https://github.com/nmeylan/rust-ro): packets, data structure, proc-macro.

If you seek for the best/most promising client implementation check-out [korangar](https://github.com/vE5li/korangar)

This repository does not provide any game assets

# Why
I wanted to be able to run the game as it was in 2005~2008, but original client from this time does not handle well high dpi screen. It is also very painful to find right game resources and right exe diff to make it works with a server.

# Principles
- Support any game resources until EP 12 (included)
- Support any packet version until 20120307 (reason is that rust-ro was implemented with this packet version support first)
- Support ALL visual effects accurately
- Do not alter original game resources: render actual resources with high dpi support
- Runnable on windows and linux
- We use a "mini framework" for the UI, in immediate mode, inspired by `egui`

# Run
Place a single game resource file at `data/data.grf`

```
run --package ragnarok-client --bin ragnarok-client
```

# Development tools

For faster feedback loop following tools are available

## Sprite Viewer

Standalone tool to browse and preview SPR/ACT sprites from GRF archives.

```bash
# Open with GRF file picker (scans current directory for .grf files)
cargo run --bin sprite-viewer

# Open a specific GRF
cargo run --bin sprite-viewer -- --grf data.grf
```

## Grf editor

```bash
cargo run --bin ragnarok-grf-editor
```

## Ui component hot reload
Creation of UI is something that can takes lot of iteration, for this reason it was designed from scratch to be hot reloadable
```bash
# In game UI
lib/ui-component/dev.sh game
# Login/char select UI
lib/ui-component/dev.sh account
```

## Effect viewer hot reload
Effect implementation also requires huge amount of iteration to get them right, effect viewer support hot reload so effect rendering can be tune and feedback is almost immediate
```bash
tools/effect-viewer-dev.sh
```

## (Game) Viewer tool
This tool provide rendering of scene + sprite + effect in same tool, this allows to validate effect rendering with actual entity rendering: allow to validate effect size (beginspell), validate entity alteration (body tint, body size change), effect alpha and additive properties
```bash
tools/viewer-dev.sh
```

# AI usage
This project leverage AI to allow a faster development, as my time is very limited. AI is being used for:
- Fix network packet handling: investigate raw packet trace
- Helping to implement rendering (wgpu and wsgl api): when i started this project my knowledge on wgpu and wsgl was almost 0, although I have implemented few effects for robrowser 4 years ago.
- Game resource format handling
- Refactoring tasks
- Write tools

# Progress
see [todo](docs/TODO.md)

# Effect
ALL effects have been implemented this would not have been possible without following resources:
- **Waken** youtube channel https://www.youtube.com/@wakenragnadev6265
- https://casual-ragnarok.github.io/ro-effects/