# Roadmap

Current state: login → server select → char select → load map → player (body + head + weapon + shield + headgear + shadow) → walk, sit/stand, doridori.

---

## Phase 1 — Complete player character ✓
Finish all sprite layers so the player looks correct.

- ✓ Weapon sprite rendering (body attachment)
- ✓ Shield sprite rendering (direction-based render order)
- ✓ Headgear/accessories (head attachment chain)
- ✓ Doridori (head_dir packet)
- ✓ Sit/stand toggle (Insert key, PacketZcNotifyAct)
- ✓ Shadow under player

**Milestone: player character renders fully equipped**

---

## Phase 2 — Audio

- BGM playback (map-specific)
- Basic SFX (UI clicks)

**Milestone: maps have background music**

---

## Phase 3 — Populate the world
Entity management so the world isn't empty.

- Entity manager (spawn, despawn, update multiple entities)
- Other players (full sprite layers)
- NPCs (sprite + name plate)
- Monsters (sprite + name/HP bar)
- Ground items (sprite + pickup)

**Milestone: see other players, NPCs, and monsters on the map**

---

## Phase 4 — Chat & basic interaction
Can communicate and talk to NPCs.

- Chat box UI (send/receive messages, whisper, channels)
- Chat packets
- NPC dialog box (text, menu choices, number input, close)
- NPC shop (buy/sell)
- Emotion/emotes
- Chat bubbles above entities

**Milestone: can chat with players and interact with NPCs/shops**

---

## Phase 5 — Core UI
Essential windows for playing the game.

- Must support multi version of the UI
- Status window (stats display + stat point allocation)
- Inventory window
- Equipment window (equip/unequip, visual update on character)
- Item tooltips
- Hotkey/shortcut bar
- Minimap
- Escape/system menu

**Milestone: all core UI windows functional**

---

## Phase 6 — Combat

- Attack action + animation
- Skill casting (cast bar, targeting)
- Skill execution + visual feedback
- Damage/heal numbers
- HP/SP bars above entities
- Status effects (icons + visuals: poison, freeze, stun, etc.)
- Death + respawn
- Skill window (skill tree, level up)
- Weapon SFX (hit sounds per weapon type)

**Milestone: can fight monsters and use skills**

---

## Phase 7 — Items
Complete the loot loop.

- Item pickup / drop / use
- Item packets
- Equipment system (card slots, refine level display)
- Card slotting
- Cart system
- Storage / warehouse

**Milestone: full item lifecycle from drop to equip**

---

## Phase 8 — Social
Multiplayer interactions.

- Party (create, join, leave, exp/item share, member HP)
- Trade (player-to-player)
- Friend list + whisper management
- Chat rooms
- Vending (setup shop + buy from vendors)

**Milestone: can party up, trade, and vend**

---

## Phase 9 — Guild & advanced systems

- Guild (create, manage members, positions, emblem, notices)
- Guild skills
- War of Emperium basics
- Quest window + quest tracking
- Pet companion system
- Homunculus
- Mercenary
- Mount/Peco (costume job swap)
- Mail/Rodex

**Milestone: guild system and companion systems working**

---

## Phase 10 — Visual effects & polish

- STR effect rendering (skill/buff visuals)
- Particle system (skills, weather, environmental)
- Weather (rain, snow, sakura per map)
- Fog
- Day/night cycle
- Skybox
- Granny models (emperium, guardian stones)
- Portal/warp transition effects

**Milestone: visually complete client**

---

## Phase 11 — Settings & QoL

- Graphics settings
- Sound settings
- Hotkey configuration
- Battle mode (keyboard skill activation)
- Slash commands (/sit, /where, /emotion, etc.)
- Context menus (right-click on player/NPC)
- Effect viewer tool

**Milestone: configurable, polished client**
