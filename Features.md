Below what has been implemented and validated.

# Account
- Login
- Server list selection
- Character select (renders each slot's sprite)
- Character creation
- Character deletion
- Join / enter game
- Change character / return to select
- Window layout persistence (restored once per login, saved on quit/logout/change-char)

# Environment
## Rendering
- Map ground (batched by texture, lightmap atlas)
- Water
- Day / night cycle (light tint fade, world lighting)
- Weather: fog, cloud, firework, snow, leaves
- Ambient RSW effects: e.g. torch, smoke, bubble, fog
- Mob / entity sprites
- GR2 3D models (emperium, guardians): pure-Rust Oodle0 + Bink texture decoders, skeletal animation
- Minimap (with entity / party / quest / NPC markers)

# Effects
**All effect IDs (from 1 to 1050) are handled** through two generic players plus custom-coded families.

## Generic players
- **SPR**: sprite-sheet / ACT animated particle effects
- **STR**: animated multi-layer texture effects (`.str`), including angle, additive/alpha blend, repeat/loop

## Custom effect families
### Casting & preparation animations
- Begin-spell rings (`begin_spell`, `begin_spell_6`, `begin_spell_8`)
- Cast circles / casting rings (`cast_circle`, `casting_ring`)
- Saint casting, color casting, flower cast
- Couple / heart casting (wedding & linked casts)
- Asura preparation, ready-portal preparation

### Melee & physical skill hits
- Bash, Bash 3D variants, Magnum Break, Bowling Bash
- Pierce, Spear Boomerang, Grimtooth
- Sonic Blow hit, Triple Attack, Chookgi (finger offensive)
- Storm Kick, Sphere Wind, Jump Kick / Jump Body
- Asura Strike body, Guillotine Fist (gumgang / gumgang2)
- Sonic / Soul strike melee (sma), generic slash
- Cart Revolution, Cart Termination, throw-item (Assassin)
- Curse attack, pressure

### Single-target magic & bolts
- Elemental bolts (fire / cold / lightning) via `magic_bolt`
- Soul Strike, Napalm Beat / Napalm Vulcan
- Fireball, Frost Diver, Grimtooth, Water Ball (2 variants)
- Kouenka (fire), electric bolts, cloud projectile
- Soul Breaker

### Area / ground magic
- Storm Gust, Heaven's Drive, Sightrasher
- Thunderstorm, Jupitel Thunder (yupitel / yufitel2)
- Meteor / Volcano, Sand Wind, Earth Spike, Waterfall
- Gravitation, Grand Cross (walls / caps, additive)

### Ground unit / placed skills (`bottom_*`)
- Sanctuary pillar, Magnus Exorcismus, song / dance
- Volcano / Deluge / Whirlwind, Land Protector, Hermode
- Safety Wall & Glass Wall, Ice Wall
- Fire Pillar, Basilica, box / light / vertical / out zones
- Warp portal, big portal, portal wind, warp zone

### Buffs & self auras
- Blessing, Increase AGI (agiup), Haste up
- Overthrust, Endure, Guard, Defender, Providence
- Aura Blade, Enhance, spirit orbit (rg_coin)
- Two-Hand Quicken, Energy Coat / body buff (body tint + afterimage)
- Status-up sparkles, peong / peong-up

### Support / heal
- Heal, Heal-SP, First Aid
- Turn Undead, Resurrection / revive
- Soul Link, line link (party links)

### Body-channel effects
- Body tint, body scale, quake body, multi-body afterimage
- Spined body, square body, 4-way body, land body

### Hit / impact markers
- Generic hit / hit2 / hit5-6, hit line, dark hit
- Cold hit, fire-splash hit, tei hit, spike burst

### Status ailments (visual)
- Poison, Curse, Silence, Stun, Sleep
- Freeze (ice-block overlay + shatter), Stone / Stone-wait
- Blind (fullscreen overlay), Bleeding
- Root, Blade Stop (rooted)

### Level & job auras
- Level-99 aura ring, transcendent sparkles, orbs (3-layer composite)

### Environmental / ambient
- Cloud, wind, sakura (cherry blossom), color paper, rainbow
- Forest light, twilight, dragon smoke, fire ivy
- Bubble drop, footprints, sparkle column

### Warp / teleport
- Warp, teleportation, portals, entry / exit

### Numbers & indicators
- Damage-number effect, colored heal / SP numbers

### Summon / special / misc
- Summon slave, super angel, saint wing, black devil, ghost
- Tarot card, wink, detecting, sight, lock-on, light sphere
- Barrier, dome ring, floor aura, map zone / pillar, call zone
- Party effect, energy drain, acid demonstration (chemical / aciddemon), venom dust
- Firecracker (pokjuk), potion effects (berserk / concentration / pillar), sparkle (banjjakii)

# Combat & gameplay
- Sound effects (SFX)
- Background music (BGM)
- Damage numbers (color by source: player red, skill, miss white; multi-hit)
- Entity animation: movement, attack, death, sit, doridori head turn
- Camera rotation locked indoors
- Server-time synced movement & actions
- Arrow / projectile flight (normal attacks + arrow skills)
- Camera shake (e.g. Meteor / earthquake skills)
- Skill casting, cooldowns, cast bars
- Emotions (emote balloons, /commands)
- Full server-time sync of hits & moves
- Marriage system

# Status appearance
- Status-icon bar (EFST icons, clock-wedge timer, tooltips)
- Ailment body tints, animation locks, movement blocks
- EFST-driven buff effects

# Stealth
- Hide / Cloaking / Chase Walk (movement & input gates, hidden-viewer render, picking filter)

# UI windows
## Character & info
- Basic info window
- Status window (stats)
- Equipment window with paperdoll
- Inventory window (3 tabs, drag & drop, floor-drop)
- Skill tree window
- Hotkey / shortcut bar
- Shortcut list window (Alt+M chat-command bindings)
- Level-up notification, item-pickup notification

## Items & interaction
- Item info window + card picture
- Card insert dialog / card slot selection
- Drop-quantity dialog
- Number input / item-list selection
- Book reader
- Confirmation dialog
- Context menu (right-click)
- Poptip / tooltips

## Social
- Chat box / chat window
- Chat room: creation, board, member window
- Party & Friends window (combined tabbed)
- Party helper window
- Guild window (context menu, invite, members with head sprites, emblem, alliance, antagonist, announcement, expel dialog with reason)
- Emblem picker window
- Emotion selector window
- Trade window (player exchange)
- Warp list window

## NPC & shops
- NPC dialog
- NPC shop
- Vending: setup, my-shop, board, shopping window
- Cart window & cart-select (change cart)

## Companions
- Pet window (info, taming, hatch, feed, performance, talk)
- Homunculus window + skill window
- Mercenary window + skill window
- Companion (Homunculus / Mercenary) AI configuration window (JSON-configurable native AI)

## Storage & mail
- Storage / Kafra warehouse window
- Mailbox window + read-mail window

## Crafting / production
- Make-item window (potion / weapon / arrow / refine)

## System
- Graphic options window (fog / effects / aura toggles, runtime)
- Sound options window
- System / game menu
- Map-missing window
- Shortcut configuration
- Slash command

# Game mechanics
## Items & equipment
- Slot / card insertion into equipment
- Equip / unequip with paperdoll rendering

## Crafting
- Potion creation (pharmacy)
- Weapon forging
- Weapon refining
- Arrow crafting

## Social systems
- Party (invite, members, context menu)
- Friends list
- Guild (invite, members, alliance, antagonist, announcement, exclusion)
- Chat rooms
- Trade between players
- Vending (open shop from cart, browse others)
- Mail (send / read)

## World & progression
- Quest log (list + detail, NPC markers, minimap dots)
- Marriage / couples (wedding costume, partner name, cut-ins, WE_ skill effects)
- Storage access

## Companions
- Pet system (taming, roulette, hatching, accessory, performance, talk)
- Homunculus (native AI, feed, skills)
- Mercenary (native AI, skills)
- Falcon companion
- Cart (visual + storage)
