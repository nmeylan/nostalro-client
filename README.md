The goal is to have a playable client to retrieve the classic ro experience (2004-2008) for nostalgic

It reuses many part of [rust-ro](https://github.com/nmeylan/rust-ro): packets, data structure, proc-macro.

**This repository does not and will not provide any game assets**.

# Progress
see [todo](docs/TODO.md)

# Why yet another client?
I wanted to be able to run the game as it was in 2005~2008, but original client from this time does not handle well high dpi screen. It is also very painful to find right game resources and right exe diff to make it works with a server.

Other implementations are not focusing on this version of the client or do not aim for an exact match


# Principles
- Support any game resources until EP 12 (included)
- Support any packet version until 20120307 (reason is that rust-ro was implemented with this packet version support first)
- Support ALL visual effects accurately
- Do not alter original game resources: render actual resources with high dpi support
- Runnable on windows and linux
- We use a "mini framework" for the UI, in immediate mode, inspired by `egui`

# Divergence from original client

## AI
Instead of using lua file from player directory to implement AI, I made the decision to implement and highly configured AI in the client directly, inspired by the famous: https://github.com/SpenceKonde/AzzyAI

Reason are:
- I don't want to add LUA support, as sources are available there is no need to allow an external system in order to customize the client: change can happen directly in the rust code, a plugin system will be added later
- Based on my experience many lua script for homunculus where buggy and suboptimal, newbie player had to use the default AI scripts which where bad, while experienced player had advanced script for their homunculus

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

Features:

- Render bmp,tga,spr(act),str,rsw
- Grid mode (view all elements in grid to ease finding of resources)
- Export element
- Add element
- Play sound

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
# Homunculus/Mercenary/Pet
lib/ui-component/dev.sh companion
# Guild
lib/ui-component/dev.sh guild
# Social: chat, chat room, guild, party
lib/ui-component/dev.sh social
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

## GR 2 viewer

```bash
# guild flag with emblem
cargo run -p ragnarok-tools --bin gr2-viewer --   --grf data/data.grf guildflag90_1.gr2 --emblem /home/nmeylan/dev/ragnarok/classic-client/data/emblem/emblem_0013.bmp

# Archer guardian
cargo run -p ragnarok-tools --bin gr2-viewer --   --grf data/data.grf aguardian90_8.gr2

# Knight guardian
cargo run -p ragnarok-tools --bin gr2-viewer --   --grf data/data.grf data/model/3dmob/kguardian90_7.gr2

# Emperium
cargo run -p ragnarok-tools --bin gr2-viewer --   --grf data/data.grf data/model/3dmob/empelium90_0.gr2
```

# AI usage
This project leverage AI to allow a faster development, as my time is very limited. AI is being used for:
- Fix network packet handling: investigate raw packet trace
- Helping to implement rendering (wgpu and wsgl api): when i started this project my knowledge on wgpu and wsgl was almost 0
- Game resource format handling
- Refactoring tasks
- Write tools
- Effects analysis (from gif) and implementation


# Resources
Without below resources, my memories alone where not enough to implement visual parity with original game

ALL effects have been implemented this would not have been possible without following resources:
- **Waken** youtube channel https://www.youtube.com/@wakenragnadev6265
- https://casual-ragnarok.github.io/ro-effects/

Various gameplay/effect/ui rendering
- https://www.youtube.com/@lordknightnecri1603 <- effects
- https://www.youtube.com/watch?v=-XCxB3hem-A&list=PLbEyWK1BqG7oYWWnTg9ENpIB_XESvml8P&index=28 <- effects
- https://www.youtube.com/watch?v=P__GwtWu6pQ <- marionette dolls
- http://guidesragnarok.free.fr/guides/guildes.php <- guild
- https://www.youtube.com/watch?v=BuEU4GeoUPQ <- chat room
