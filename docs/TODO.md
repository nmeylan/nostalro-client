TODO:

# Companion
- ~~homonculus~~
- ~~mercenary~~
- ~~pet~~

# Sprite rendering
- ~~Weapon sprite~~ (done — full rendering with ACT animation in sprite.rs)
- ~~Shield sprite~~ (done — render order fix for direction-based layering)
- ~~Headgear/accessories~~ (done — three-layer system: top/mid/bottom)
~~- Mount/Peco (costume job swap, loads mounted body sprite)~~
~~- cart~~
- ~~Other players / NPCs / monsters entity sprites~~ (done — entity collection manages Player/NPC/Monster types)
- ~~Doridori head animation~~ (done — head_dir from server selects head/headgear motion)
- ~~Divide attachment offset by clip zoom for weapons/accessories (original compensates when sprClip zoom != 1.0)~~
- ~~Damage numbers display~~
- ~~Emotion/emote bubbles~~ (done — EmotionState with 2.5s display)
- ~~Chat bubbles above entities~~ (done — text bubble with background above entity, 5s duration)
- ~~Name above entities~~ (done — text with outline rendered above sprites)
- ~~HP bar above entities~~ (done — green/yellow/red bar below name, on hover + always for player)
- ~~Shadow under entities~~ (done — shadow size table for 200+ jobs)
- GM sprite based on account id + yellow text color + yellow name

# Rendering
- ~~STR effects (skill/buff visuals, 200 effect types)~~
- ~~Particle system (2D/3D particles for skills, weather, buffs)~~
- ~~Fog~~
- ~~Weather (rain, snow, sakura)~~
- ~~Day/night cycle (lighting changes)~~
- ~~Granny models (emperium, guardian)~~
- ~~ALL 1050 effects https://casual-ragnarok.github.io/ro-effects/~~
# UI
- ~~Character selection~~
- ~~Chat box (normal, whisper, party, guild channels)~~
- ~~Status window (stats, stat allocation)~~
- ~~Inventory window~~
- ~~Equipment window~~
- ~~Skill window (skill tree, skill levels)~~
- ~~Hotkey/shortcut bar (F1-F9 skills/items)~~
- ~~Minimap~~
- ~~basic info window~~
- ~~NPC dialog box (text, menu choices, number input)~~
- ~~NPC shop (buy/sell)~~
~~- Vending (player shop setup + buying)~~
- ~~Party window~~
- ~~Guild window (members, positions, skills, emblem, notices)~~
- ~~Friend/messenger list~~
- ~~Game menu: graphic/audio options~~
- ~~Emotion selector~~
- ~~Char delete~~
- ~~Quest window~~
~~- Cart window~~
- ~~Storage/warehouse window~~
- ~~Item tooltips (description, stats, cards, refine level)~~
- Context menu (right-click on player/NPC)[ui-component](../lib/ui-component)
- ~~Escape/system menu~~
- ~~Card illustration display~~
~~- Refining UI~~
- ~~Mail/Rodex~~
- Loading screen
- Login background
- 
# Entities
- ~~Multiple entity management (spawn, despawn, update)~~ (done — entity_collection.rs)
- ~~Other players (full sprite layers)~~
- ~~cart~~
- ~~Trap~~
- ~~Falcon~~
- ~~NPCs~~
- ~~Monsters~~
- ~~Ground items (dropped items with pickup)~~
- ~~Skill ground units (AoE, bottom song)~~
- ~~Pet companion rendering~~
- ~~Homunculus rendering~~
- ~~Mercenary rendering~~

# Combat
- ~~Attack action + animation~~
- ~~Skill casting (cast bar, cast animation)~~
~~- Skill execution + effects~~
- ~~Damage/heal numbers~~
- ~~Status effects (buff/debuff icons + visuals: poison, freeze, stun, etc.)~~
- ~~HP/SP bars~~
- ~~Death~~ + respawn
- ~~Sit/stand actions~~ 
- ~~hide/cloak~~

# Items & equipment
- ~~Inventory management (pickup, drop, use, equip)~~
- ~~Equipment slots + visual update on character~~
- ~~Card slotting~~
- ~~Item info window~~
~~- Item crafting (arrows, cooking)~~
~~- Item refining~~
~~- Cart system~~
- ~~Storage/warehouse~~
- ~~Bank~~

# Social
- ~~Party (create, join, leave, exp/item share settings)~~
- ~~Guild (create, manage, skills, emblem)~~
- ~~War of emperium: no name, no damage number, emblem above head~~
- ~~Friend list + whisper~~
- ~~Chat rooms~~
- ~~Trade~~
~~- Vending/merchant shop~~
- ~~Marriage/couples~~

# Network
- ~~Chat packets~~ (done — multi-channel chat)
- ~~Guild chat~~
- ~~Whisper~~
- ~~Team chat~~ 
- ~~Combat packets (attack, skill use, damage)~~
- ~~Item packets (pickup, drop, use, equip, unequip)~~
- ~~Skill packets (cast, execute, ground target)~~
- ~~NPC packets (dialog, menu, shop, close)~~
- ~~Entity spawn/despawn/update packets~~ (done — full spawn/move/vanish/act handling)
- ~~Status change packets~~
- ~~Party/guild packets~~
- ~~Trade/vending packets~~
- ~~Quest packets~~
- ~~Pet/homunculus/mercenary packets~~
- ~~Mail packets~~
- ~~Effect wiring~~

# Audio
- ~~BGM playback (map-specific music)~~
- ~~SFX (skill sounds, weapon hit sounds, UI sounds, ambient)~~
- ~~Positional audio (3D sound based on entity distance)~~

# World
- ~~Portal/warp transitions~~ (done — MapChanged event with full map reload)
~~- Map type properties (PvP zones, no-teleport, no-skill zones)~~
- ~~Weather per map~~

# Input
- ~~Slash commands~~ (/sit, /emotion, /where, etc.) https://irowiki.org/classic/Basic_Game_Control
- ~~Keyboard shortcuts~~
- ~~Hotkey configuration~~
- ~~Battle mode (keyboard skill activation)~~

# Tools
~~- Effect viewer tool~~

# Performance
- Profile memory
- Profile CPU

# To valide
- THQ: don't rembmer if there is packet specific