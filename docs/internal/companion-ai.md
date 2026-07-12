# Companion AI

A configurable AI for the Homunculus and Mercenary systems. It decides how the
companion targets, chases, attacks, casts skills, buffs, heals, and moves
relative to its owner. This document describes what the code actually does. Where
a configuration field exists but the engine does not yet act on it, that is
stated explicitly.

## Contents

- Overview
- Architecture
- Setup and configuration file
- The in-game configuration window
- Commanding a companion
- Friending other players
- Behavior by state
- Skills the AI uses
- Configuration options that drive behavior
- Configuration fields present but not consumed
- Tactics
- What is not implemented

## Overview

The AI runs a small state machine per companion. Once per engine tick it takes a
snapshot of the world (the companion, the owner, and nearby actors) and returns a
list of intents (move, attack, cast a skill). The client turns those intents into
the same network packets a player would send.

The default configuration works without any changes. Every option is stored in a
JSON file and can be edited in-game.

Supported homunculus types are the four base classes and their evolutions:

- 1 Lif
- 2 Amistr
- 3 Filir
- 4 Vanilmirth
- 5 to 8 the evolved forms, which resolve to the same behavior as their base
  class using the type number modulo 4

Homunculus S classes are not supported. Their combo, grapple, minion, and mob
skills are not modelled.

## Architecture

The AI is a standalone crate, `ragnarok-ai`, that depends only on the shared data
models plus serde and tracing. It has no dependency on the network, renderer,
game, or UI crates. This keeps it usable from tools and testable in isolation.

The interface is a snapshot in and intents out:

- `AiContext` carries the per-tick world snapshot: the companion position, HP, SP,
  motion, attack range and attack speed, the companion type, the owner position,
  motion and HP, the list of nearby actors, the companion skill list, the active
  configuration values (`AiParams`), the tactics table, the PVP tactics table, and
  a function that classifies another actor as friend, enemy, and so on.
- `CompanionAi::tick` advances the state machine and returns `Vec<AiIntent>`.
- `AiIntent` is one of `MoveTo`, `MoveToOwner`, `Attack`, `SkillObject`,
  `SkillGround`, or `EmergencyDisconnect`.

The engine tick interval is 140 ms. The client feeds elapsed time each frame and
the engine steps the state machine in fixed 140 ms increments, capped so a long
stall cannot run many steps at once.

## Setup and configuration file

Configuration lives in `companion_ai.json` at the repository root, next to
`config.json`. It is loaded at startup and created with defaults if absent.

The file has these sections:

- `homunculus` object of homunculus options
- `mercenary` object of mercenary options
- `homunculus_tactics` array of per-mob tactic rows for the homunculus
- `mercenary_tactics` array of per-mob tactic rows for the mercenary
- `homunculus_pvp_tactics` array of PVP tactic rows for the homunculus
- `mercenary_pvp_tactics` array of PVP tactic rows for the mercenary
- `friends` map from account id to a friend class

Every field is optional. Missing fields self-heal to their default value when the
file is read, so a partial file is valid. This also means that once a file has
been saved, fields already present keep their saved value; new default tactics are
not merged into an existing saved tactics array. Use the Reset control in the
configuration window to adopt the current default tables.

Enum-valued options are stored as their integer value, so the JSON matches the
documented option numbers below.

## The in-game configuration window

Window id 3200. Open it by right-clicking the companion and choosing AI Settings,
or from the AI button on the homunculus window.

It has seven tabs:

- Homun homunculus options
- Merc mercenary options
- H.Tactics homunculus per-mob tactics editor
- M.Tactics mercenary per-mob tactics editor
- H.PVP homunculus PVP tactics
- M.PVP mercenary PVP tactics
- Extra shared controls

Option rows are checkboxes, integer steppers, or cycling enum selectors, grouped
under category headers. The tactics tabs show a row selector with add and delete
controls and one editable row of the thirteen tactic columns.

Save writes the current values to `companion_ai.json`. Revert reloads the file.
Reset restores the built-in defaults, including the full default tactic tables.

## Commanding a companion

Direct commands override the AI until they complete, then the AI resumes.

- Alt and left-click, or Alt and right-click, on a target or cell issues a
  command. An attackable target under the cursor becomes an attack order.
  Anything else is a move order to that cell.
- If a mercenary is out, the command is sent to the mercenary. Otherwise it is
  sent to the homunculus.
- Holding Shift while issuing a command queues it as a reserved command. Reserved
  commands run after the current action finishes, in order, up to ten queued.
- Right-clicking the companion without Alt opens a context menu: Show Info, Feed
  (homunculus only), Stand By, and AI Settings.
- Stand By puts the companion in a follow-and-hold state. It stops seeking
  targets and stays near the owner until the next command. Any command clears it.
- Ctrl and T toggles Stand By for the mercenary.

Casting a specific skill:

- Open the homunculus or mercenary skill window, select a skill, and press Use, or
  double-click the skill row.
- For a self or support skill the cast is issued immediately.
- For a target skill the cursor arms, and the next left-click on a target fires it.
- For a ground skill the next click on a cell fires it.

The command set understood by the engine is Move, Stop, Hold, Follow, Patrol,
Attack a target, Attack an area, cast a skill on a target, and cast a skill on the
ground.

## Friending other players

The `friends` section maps an account id to a friend class. The friend classifier
combines this map with the current party and the owner. Friend classes are:

- 1 Friend
- 2 Retainer
- 3 PK Friend
- 10 Neutral
- 11 Enemy
- 12 KoS
- 13 Ally

The owner, friends, and retainers are protected. A monster that a friend is
already fighting is picked up as a target through the friend-target scan, and a
monster that is fighting a non-friend player is left alone by kill-steal
protection. Friend classes also select which PVP tactic applies to a player when
PVP mode is on.

## Behavior by state

Idle, in this order:

1. Drain one reserved command if any is queued.
2. Clear berserk mode.
3. If avoidance is on and a dangerous monster is within aggro distance, retreat
   toward the owner.
4. Upkeep, gated by the global skill delay: heal if enabled and needed, then
   Amistr Castling if enabled and the owner is mobbed, then one self-buff whose
   configured timing matches the idle context.
5. If the owner is sitting and rest is allowed, stop seeking targets.
6. Otherwise select a target: a monster a friend is attacking, then an aggro or
   reaction target, then a tank target.
7. If no target and the owner is out of follow range, follow.
8. Otherwise, if idle-walk is on and the companion is healthy, wander near the
   owner.

Chase: move toward the target, drop it if it goes out of sight or becomes a
kill-steal, give up after repeated failure to close and mark it unreachable, and
optionally switch to a better opportunistic target. Enter attack when in range.

Attack: verify the target is alive and in range. Fire the offensive attack skill
if one is selected and its gates pass, otherwise melee. When in berserk mode, fire
a berserk-timed buff first if one is due. When dance-attack is enabled, circle one
cell around the target after the melee.

Tank and tank-chase: approach a tank-tactic monster and hit it periodically to
hold its attention when it is not already attacking the companion.

Follow: move to the owner until within follow range, then return to idle.

Emergency layer: before any state runs each tick, buffs configured for the ASAP
timing are applied in any state.

## Skills the AI uses

Offensive attack skills, cast automatically while attacking:

- Filir casts Moonlight
- Vanilmirth casts Caprice
- Lif and Amistr have no offensive attack skill and only melee
- A mercenary casts the first attack skill it has learned from this priority
  order: Double Strafe, Sharp Shooting, Pierce, Spiral Pierce, Bash. A mercenary
  that has learned none of these only melees.

Selection gates, all of which must pass:

- Use Attack Skill is on.
- The global auto skill delay since the last cast has elapsed.
- The tactic Skill Use column allows another cast on this target.
- The per-skill reuse cooldown has elapsed. Moonlight is 2000 ms. Caprice is
  2000 ms plus 200 ms per level.
- SP minus the reserve is at least the skill cost.
- The skill range reaches the target.

The cast level and SP cost come from the live server skill list, so a skill must
be learned to be used.

Cast-fail detection: after an auto-cast the engine watches the result. SP being
consumed or a visible cast motion counts as success. No sign within 1500 ms counts
as a failure, which clears the cooldown so the cast is retried, capped at two
retries per engagement and reset when the target changes. The wider timeout and
the hard cap exist because SP and motion updates arrive over the network with lag.

Buffs, cast while idle by default. Each is re-cast when its duration lapses:

- Offensive: Lif Mental Charge, Amistr Bloodlust, Filir Flitting
- Defensive: Lif Urgent Escape, Amistr Bulwark, Filir Accelerated Flight

Healing:

- Lif casts its heal on the owner when the owner drops below the owner heal
  percent
- Vanilmirth casts Chaotic Blessing on itself when it drops below the self heal
  percent

Castling: an Amistr with Castle Defend enabled casts Castling on the owner to swap
places and tank when the number of monsters attacking the owner reaches the
threshold.

## Configuration options that drive behavior

These options are read by the engine. Names match the JSON field names.

Engaging and targeting:

- AggroHP the companion only picks aggressive targets while its HP percent is
  above this
- AggroSP the companion only picks aggressive targets while its SP percent is
  above this
- SuperPassive when set, the companion never seeks targets on its own
- OpportunisticTargeting while chasing, switch to a closer or higher priority
  target if one appears
- DoNotAttackMoving do not aggro or chase monsters that are moving
- AttackLastFullSP for the attack-last basic tactic, only engage at full SP
- TankMonsterLimit the most tank-tactic monsters the companion will pick up
- AutoMobCount the aggro count at which the tank-when-mobbed basic tactic switches
  to attacking
- StationaryAggroDist and MobileAggroDist the aggro search radius from the owner,
  chosen by whether the owner is moving
- StationaryMoveBounds and MobileMoveBounds how far from the owner the companion
  will roam, chosen by whether the owner is moving

Attack skills:

- UseAttackSkill enable offensive attack-skill casting
- AutoSkillDelay minimum milliseconds between auto skill casts
- AttackSkillReserveSP SP kept in reserve when a tactic SP column is set to use it

Buffs and heal:

- UseOffensiveBuff and UseDefensiveBuff when to cast each buff group. The value is
  a timing: 0 never, 1 while idle, 2 while berserk, 3 ASAP. The idle context fires
  timing 1, the berserk combat context fires timing 2, and the emergency layer
  fires timing 3.
- UseAutoHeal 0 disables healing, any other value enables it
- HealOwnerHP heal the owner below this HP percent
- HealSelfHP self-heal below this HP percent

Movement and rest:

- DoNotChase do not close on targets whose chase tactic defers to this option
- DoNotUseRest do not stop and rest when the owner sits
- UseIdleWalk 0 disables idle wandering, any other value enables it
- IdleWalkSP only idle-walk while SP percent is above this

Dance, berserk, castling, avoid, PVP:

- UseDanceAttack enable the circle-strafe dance, only for Vanilmirth and Filir
  with DoNotChase set
- DanceMinSP only dance while SP is at least this
- UseBerserkMobbed enter berserk mode when the weighted aggro count exceeds this,
  which enables berserk-timed buffs during combat
- UseAvoid retreat from monsters on the dangerous-mob list
- UseCastleDefend enable Amistr Castling defense
- CastleDefendThreshold cast Castling when at least this many monsters attack the
  owner
- PVPmode consult the PVP tactics against players

## Configuration fields present but not consumed

The configuration file and window contain many more fields than the list above.
They are stored and preserved but the engine does not yet act on them. This
includes the Homunculus S skill options, the fine-grained kiting fields such as
KiteMonsters and KiteStep, the chase SP pause fields, RescueOwnerLowHP,
UseCastleRoute, the mob-count and combo skill options, pushback options, and the
various level overrides for buffs. Buff levels are currently taken from the live
learned skill rather than from the per-buff level fields.

Do not rely on these fields having an effect. They are retained for compatibility
with the tactics and configuration layout and as room for future behavior.

## Full configuration reference

Every field in the `homunculus` and `mercenary` sections, with its default value
and purpose. Fields marked active are read by the engine. Fields marked reserved
are stored and editable but currently have no effect. Defaults shown are the
homunculus defaults; where the mercenary section lacks a field, the difference is
noted at the end.

### Basic

- AggroHP (60) active. Only pick aggressive targets while the companion HP percent
  is above this.
- AggroSP (0) active. Only pick aggressive targets while the companion SP percent
  is above this.
- OldHomunType (3) reserved. Pre-mutation type for a Homunculus S, used to pick
  its old-style skills. Not used since Homunculus S is unsupported.
- UseSkillOnly (-1) reserved. Reference mode to cast only, never melee. The engine
  always allows melee.
- UseAttackSkill (1) active. Enable offensive attack-skill casting. 1 on, 0 off.
- OpportunisticTargeting (0) active. While chasing, switch to a closer or higher
  priority target if one appears. 1 on.
- DoNotChase (0) active. Do not close on targets whose chase tactic defers to this
  option, and required for the dance attack. 1 on.
- UseDanceAttack (0) active. Circle-strafe the target between melee hits. Only
  Vanilmirth and Filir with DoNotChase set. 1 on.
- SuperPassive (0) active. The companion never seeks targets on its own. 1 on.
- RescueOwnerLowHP (0) reserved. Intended to drop the current target to rescue the
  owner below this HP.
- AssumeHomun (1) reserved. Startup hint used by the reference before the companion
  type is known.
- AttackLastFullSP (0) active. For the attack-last basic tactic, only engage at
  full SP. 1 on.
- DanceMinSP (0) active. Only dance while SP is at least this.
- TankMonsterLimit (4) active. The most tank-tactic monsters the companion will
  pick up at once.
- StationaryAggroDist (12) active. Aggro search radius from the owner while the
  owner is not moving.
- MobileAggroDist (7) active. Aggro search radius from the owner while the owner
  is moving.
- UseAvoid (0) active. Retreat from monsters on the dangerous-mob list. 1 on.
- DoNotAttackMoving (0) active. Do not aggro or chase moving monsters. 1 on.
- LagReduction (0) reserved. Reference option to reduce action frequency on laggy
  connections.
- LiveMobID (0) reserved. Reference toggle for reading mob ids from a live source.
- PainkillerFriends (0) reserved. Cast Painkiller on friends. Homunculus S only.
- PainkillerFriendsSave (0) reserved. Companion of the Painkiller-on-friends
  option.

### Attack skills

- AttackSkillReserveSP (0) active. SP kept in reserve when a tactic SP column is
  set to -1.
- AutoMobMode (2) reserved. Reference anti-mob AoE targeting mode.
- AutoMobCount (2) active. Aggro count at which the tank-when-mobbed basic tactic
  switches to attacking.
- AutoComboMode (1) reserved. Homunculus S combo skill mode.
- AutoComboSkill (0) reserved. Homunculus S combo skill selection.
- AutoComboSpheres (10) reserved. Spirit sphere threshold for combos.
- UseHomunSSkillChase (1) reserved. Use Homunculus S skills while chasing.
- UseHomunSSkillAttack (1) reserved. Use Homunculus S skills while attacking.
- AutoSkillDelay (400) active. Minimum milliseconds between auto skill casts.
- AutoSkillLimit (100) reserved. Reference cap on skill casts.
- AoEMaximizeTargets (0) reserved. Aim AoE skills to hit the most targets.
- AoEReserveSP (1) reserved. SP reserve for AoE skills.
- AoEFixedLevel (0) reserved. Cast AoE skills at a fixed level.
- CastTimeRatio (0.80) reserved. Multiplier applied to variable cast time when
  estimating cast duration.
- UseAutoPushback (0) reserved. Use a pushback skill to knock enemies away.
- AutoPushbackThreshold (2) reserved. Aggro count that triggers pushback.
- AttackTimeLimit (10000) reserved. Milliseconds after which a stuck attack is
  abandoned.

### Homunculus S skills

All fields in this group are reserved. Homunculus S is unsupported, so none take
effect. They cover the Eira, Bayeri, Sera, Eleanor, and Dieter skill selections
and levels: UseEiraSilentBreeze (0), EiraSilentBreezeLevel (5),
UseEiraXenoSlasher (0), EiraXenoSlasherLevel (0), UseEiraEraseCutter (0),
EiraEraseCutterLevel (0), UseEiraOveredBoost (0), UseBayeriStahlHorn (1),
BayeriStahlHornLevel (5), UseBayeriHailegeStar (1), BayeriHailegeStarLevel (5),
UseBayeriAngriffModus (0), UseBayeriGoldenPherze (0), UseBayeriSteinWand (0),
BayeriSteinWandLevel (5), UseSteinWandSelfMob (2), UseSteinWandOwnerMob (2),
UseSteinWandTele (0), StienWandTelePause (3000), UseSeraParalyze (0),
SeraParalyzeLevel (5), UseSeraPoisonMist (0), SeraPoisonMistLevel (5),
UseSeraCallLegion (1), SeraCallLegionLevel (5), UseSeraPainkiller (0),
UseEleanorSonicClaw (1), EleanorSonicClawLevel (5), EleanorSilverveinLevel (5),
EleanorMidnightLevel (5), EleanorDoNotSwitchMode (0), UseDieterLavaSlide (1),
DieterLavaSlideLevel (5), UseDieterMagmaFlow (0), UseDieterGraniticArmor (0),
UseDieterPyroclastic (0).

### Movement, follow, and rest

- FollowStayBack (2) reserved. Preferred trailing distance while following.
- RestXOff (2) reserved. X offset from the owner used when resting.
- RestYOff (0) reserved. Y offset from the owner used when resting.
- DoNotUseRest (0) active. Do not stop and rest when the owner sits. 1 on.
- SpawnDelay (1000) reserved. Delay before acting after the companion spawns.
- MoveSticky (0) reserved. Reference sticky-position behavior.
- MoveStickyFight (0) reserved. Sticky position while fighting.
- UseIdleWalk (0) active. Wander near the owner when idle. 0 off, any other value
  on.
- IdleWalkSP (80) active. Only idle-walk while SP percent is above this.
- IdleWalkDistance (4) reserved. Intended idle-walk radius. The engine uses a
  fixed radius.
- UseCastleRoute (0) reserved. Use Amistr Castling to keep up with the owner.
- RelativeRoute (1) reserved. Reference route-following behavior.
- ChaseSPPause (0) reserved. Pause chasing when SP is low.
- ChaseSPPauseSP (-60) reserved. SP level for the chase pause.
- ChaseSPPauseTime (3000) reserved. Duration of the chase pause.
- StationaryMoveBounds (14) active. How far from the owner the companion roams
  while the owner is not moving.
- MobileMoveBounds (9) active. How far from the owner the companion roams while
  the owner is moving.

### Buffs and heal

- UseOffensiveBuff (1) active. When to cast the offensive self-buff. 0 never, 1
  idle, 2 berserk, 3 ASAP.
- UseDefensiveBuff (1) active. When to cast the defensive self-buff. Same values
  as above.
- DefensiveBuffOwnerHP (0) reserved. Cast a defensive buff when the owner is below
  this HP.
- DefensiveBuffOwnerMobbed (0) reserved. Cast a defensive buff when the owner is
  mobbed.
- UseProvokeOwner (0) reserved. Cast Provoke on the owner. Not applicable to base
  homunculi.
- ProvokeOwnerMobbed (3) reserved. Owner mob count that triggers Provoke.
- LifEscapeLevel (5) reserved. Level for the Lif defensive buff. The engine uses
  the learned level.
- FilirFlitLevel (1) reserved. Level for the Filir offensive buff. The engine uses
  the learned level.
- FilirAccelLevel (1) reserved. Level for the Filir defensive buff. The engine
  uses the learned level.
- AmiBulwarkLevel (5) reserved. Level for the Amistr defensive buff. The engine
  uses the learned level.
- HealOwnerHP (50) active. Heal the owner below this HP percent.
- HealSelfHP (50) active. Self-heal below this HP percent.
- HealOwnerBreeze (0) reserved. Use a Homunculus S heal on the owner.
- UseAutoHeal (0) active. Enable healing. 0 off, any other value on.
- LavaSlideMode (0) reserved. Dieter Lava Slide placement mode.
- PoisonMistMode (0) reserved. Sera Poison Mist placement mode.
- UseCastleDefend (0) active. Amistr swaps with the owner to tank when the owner
  is mobbed. 1 on.
- CastleDefendThreshold (4) active. Owner mob count that triggers Castling.
- UseSmartBulwark (0) reserved. Reserve extra SP before casting Bulwark so a
  following buff still fits.

### Kiting

All fields in this group are reserved. The reference declares a fine-grained kite
system that it never wired. The circle-strafe dance is driven by UseDanceAttack
instead. Fields: KiteMonsters (0), KiteStep (5), KiteParanoidStep (2),
KiteThreshold (3), KiteParanoidThreshold (2), KiteBounds (10), KiteParanoid (0),
ForceKite (0), FleeHP (0).

### Friending and standby

- StandbyFriending (1) reserved. Auto-friend behavior while on standby.
- MirAIFriending (1) reserved. Cross and circle friending gesture.
- DefendStandby (0) reserved. Defend while on standby.
- StickyStandby (1) reserved. Keep standby across relogs.

### Berserk

- UseBerserkMobbed (0) active. Enter berserk mode when the weighted aggro count
  exceeds this, enabling berserk-timed buffs in combat. 0 disables.
- UseBerserkSkill (0) reserved. Force skills while berserk.
- UseBerserkAttack (0) reserved. Force attacks while berserk.
- Berserk_SkillAlways (0) reserved. Always cast skills while berserk.
- Berserk_Dance (0) reserved. Dance while berserk.
- Berserk_IgnoreMinSP (0) reserved. Ignore SP reserves while berserk.
- Berserk_ComboAlways (0) reserved. Always combo while berserk. Homunculus S only.

### Other

- PVPmode (0) active. Consult the PVP tactics against players. 1 on.

### Mercenary section differences

The mercenary section shares the same field names and defaults for the options
above, except that it omits the homunculus-only fields: the heal thresholds and
UseAutoHeal, the Castling fields, the per-buff level overrides, the Homunculus S
skill fields, and DanceMinSP. For the mercenary the engine treats healing as off
and Castling as off. The mercenary auto-casts an attack skill but does not
self-buff or heal.

## Tactics

A tactic row describes how to treat one monster class. Rows are keyed by the
monster class id. Lookup falls back in this order: the exact class row, then the
treasure-chest row for treasure classes, then the default row with id 0.

Each row has thirteen columns:

1. Basic the base engagement stance
2. Skill Use whether and how often to cast the attack skill
3. Kite
4. Cast React whether to react to the monster starting a cast
5. Pushback
6. Debuff
7. Skill Class
8. Rescue whether to rescue a target the monster is attacking
9. SP Reserve SP to keep when casting on this monster, or -1 to use the global
   attack skill reserve
10. Snipe
11. KS kill-steal policy
12. Weight the monster weight used in the aggro count
13. Chase how far to chase this monster

The engine currently reads columns 1, 2, 4, 8, 9, 11, 12, and 13. Columns 3
Kite, 5 Pushback, 6 Debuff, 7 Skill Class, and 10 Snipe are stored and editable
but not acted on.

Basic tactic values:

- -2 tank when mobbed
- -1 tank
- 0 ignore, never attack this monster
- 2 attack low, engage while HP is above the aggro threshold
- 3 attack medium
- 4 attack high
- 5 react low, only defend when attacked
- 7 react medium
- 8 react high
- 9 react self, defend only when the monster attacks the companion, not the owner
- 10 to 12 snipe variants
- 13 attack low, react medium
- 14 attack last
- 15 attack top

Skill Use values:

- 0 never
- 100 always
- a positive number N, cast up to N times on each target
- a negative number -N, cast once at level N

KS values are -1 polite, 0 never, and 1 always. Chase values are -1 normal, 0
always, 1 never, and 2 clever.

Default tactic table: the shipped homunculus table treats plants and mushrooms as
react-only, snipes orc archers, kites drainliar and orc skeletons, ignores
treasure boxes, and attacks everything else with the default stance. The mercenary
table has a default row, a default summon row, and an auto-detect plant row.

### PVP tactics

When PVP mode is on, hostile players enter the target list and resolve their basic
and cast stance from the PVP tactic table by friend class rather than the per-mob
table. The shipped rows are: KoS attacked aggressively, Enemy and Neutral defended
when attacked, and Friend, Ally, and Retainer ignored. When PVP mode is off,
players are never targeted and monster targeting is unchanged.

## What is not implemented

- Homunculus S behavior, including combo, grapple, minion, and mob skills.
- Mercenary buff, heal, pushback, and trap skills. A mercenary auto-casts its
  learned attack skill but does not yet self-buff, heal, or lay traps.
- The provoke and targeted-buff-into-range substate. For the four base homunculus
  classes there is no skill that needs it, since the only targeted cast is the Lif
  heal on the owner, and the server walks the caster into range for that.
- The reference AI declares an avoid monster list, a per-mob kite tactic column,
  and an idle-walk state but never wires any of them. Avoidance and idle-walk here
  implement the evident intent. Idle-walk uses a single circle wander rather than
  distinct pattern modes. The kite tactic column remains unread; the circle-strafe
  dance is driven by UseDanceAttack instead.
