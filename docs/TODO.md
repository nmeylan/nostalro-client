TODO:

# Sprite rendering
- Weapon sprite (attach to body attachInfo[0])
- ~~Shield sprite~~ (done — render order fix for direction-based layering)
- Headgear/accessories (chain: body→head→accessory via attachInfo[0])
- Mount/Peco (costume job swap, loads mounted body sprite)
- Other players / NPCs / monsters entity sprites
- ~~Doridori head animation~~ (done — head_dir from server selects head/headgear motion)
- Divide attachment offset by clip zoom for weapons/accessories (dhxj compensates when sprClip zoom != 1.0)
- Damage numbers display
- Emotion/emote bubbles
- Chat bubbles above entities
- Name / HP bar above entities
- Shadow under entities

# Rendering
- STR effects (skill/buff visuals, ~200 effect types)
- Particle system (2D/3D particles for skills, weather, buffs)
- Fog
- Weather (rain, snow, sakura)
- Day/night cycle (lighting changes)
- Granny models (emperium, guardian)
- Skybox

# UI
- ~~Chat box (normal, whisper, party, guild channels)~~
- Status window (base/job level, stats, stat allocation)
- Inventory window
- Equipment window
- Skill window (skill tree, skill levels)
- Hotkey/shortcut bar (F1-F9 skills/items)
- Minimap
- NPC dialog box (text, menu choices, number input)
- NPC shop (buy/sell)
- Trade window (player-to-player)
- Vending (player shop setup + buying)
- Party window
- Guild window (members, positions, skills, emblem, notices)
- Friend/messenger list
- Option/settings window
- Emotion selector
- Quest window
- Cart window
- Storage/warehouse window
- Item tooltips (description, stats, cards, refine level)
- Context menu (right-click on player/NPC)
- Escape/system menu
- Card illustration display
- Refining UI
- Mail/Rodex

# Entities
- Multiple entity management (spawn, despawn, update)
- Other players (full sprite layers)
- NPCs
- Monsters
- Ground items (dropped items with pickup)
- Skill ground units (AoE, traps)
- Pet companion rendering
- Homunculus rendering
- Mercenary rendering

# Combat
- Attack action + animation
- Skill casting (cast bar, cast animation)
- Skill execution + effects
- Damage/heal numbers
- Status effects (buff/debuff icons + visuals: poison, freeze, stun, etc.)
- HP/SP bars
- Death + respawn
- ~~Sit/stand actions~~ (done — Insert key toggle, PacketZcNotifyAct handling)

# Items & equipment
- Inventory management (pickup, drop, use, equip)
- Equipment slots + visual update on character
- Card slotting
- Item crafting (arrows, cooking)
- Item refining
- Cart system
- Storage/warehouse

# Social
- Party (create, join, leave, exp/item share settings)
- Guild (create, manage, skills, emblem, war of emperium)
- Friend list + whisper
- Chat rooms
- Trade
- Vending/merchant shop
- Marriage/couples

# Network
- Chat packets
- Combat packets (attack, skill use, damage)
- Item packets (pickup, drop, use, equip, unequip)
- Skill packets (cast, execute, ground target)
- NPC packets (dialog, menu, shop, close)
- Entity spawn/despawn/update packets
- Status change packets
- Party/guild packets
- Trade/vending packets
- Quest packets
- Pet/homunculus/mercenary packets
- Mail packets

# Audio
- BGM playback (map-specific music)
- SFX (skill sounds, weapon hit sounds, UI sounds, ambient)
- Positional audio (3D sound based on entity distance)

# World
- Portal/warp transitions
- Map type properties (PvP zones, no-teleport, no-skill zones)
- Weather per map

# Input
- Slash commands (/sit, /emotion, /where, etc.)
- Keyboard shortcuts
- Hotkey configuration
- Battle mode (keyboard skill activation)

# Tools
- Effect viewer tool
