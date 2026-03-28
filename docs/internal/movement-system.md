# Movement System Analysis

Analysis of how movement is implemented in the original Ragnarok Online client and robrowser,
compared with our current Rust client.

## Packets

### Client to Server

| Packet | ID | Fields | Description |
|---|---|---|---|
| `CZ_REQUEST_MOVE` | 0x0085 | `dest[3]` (encoded x, y, dir) | Client requests to walk to a cell |

### Server to Client

| Packet | ID | Fields | Description |
|---|---|---|---|
| `ZC_NOTIFY_PLAYERMOVE` | 0x0087 | `moveStartTime`, `MoveData[6]` (src+dest encoded) | Server confirms the player's own movement with authoritative timing |
| `ZC_NOTIFY_MOVE` | 0x0086 | `GID`, `moveStartTime`, `moveServerEndTime`, `speed`, `MoveData[6]` | Server notifies that another entity started moving |
| `ZC_STOPMOVE` | 0x0088 | `AID`, `xPos`, `yPos` | Server forces an entity to stop at a given cell |
| `ZC_NOTIFY_ACT` | 0x008a | `GID`, `targetGID`, `startTime`, `attackMT`, `attackedMT`, `damage`, `count`, `action` | Action notification (attack, damage, etc.) |

Position encoding: 3 bytes for single position (`x[10] y[10] dir[4]`), 6 bytes for source+destination pair.

## Mouse-Down Continuous Walking

In both reference clients, holding the left mouse button down causes the client to repeatedly
send walk packets to the server, allowing the player to steer their character by dragging the
cursor. This is a core UX pattern in Ragnarok Online.

### Original game

**Files:** `GameMode.cpp:6415-6455`, `GameMode2.cpp:3336-3365`

The input system polls `g_mouse.GetLBtn()` each frame and distinguishes three states:
`BTN_DOWN` (initial press), `BTN_PRESSED` (held), and `BTN_DBLCLK` (double-click).

Two independent throttles gate packet sending:

1. **Auto-click throttle (150ms):** When the mouse is held (`BTN_PRESSED`), the client only
   processes a new click if at least 150ms have elapsed since the last one:
   ```cpp
   bool autoClickEnoughDelayed = timeGetTime() > m_leftBtnClickTick + 150;
   bool autoClick = (!m_world->m_player->m_proceedType
                     && g_mouse.GetLBtn() == BTN_PRESSED
                     && autoClickEnoughDelayed
                     && m_isAutoMoveClickOn);
   ```

2. **Packet send throttle (500ms) with server ack gating:** Before sending `CZ_REQUEST_MOVE`,
   the client checks whether the server has acknowledged the previous move AND whether 500ms
   have elapsed:
   ```cpp
   static unsigned s_last_send_move_time = 0;
   unsigned int delay = 500;
   unsigned cur = ::GetTickCount();

   if (g_PlayerRecvMoveFlag == false && cur - s_last_send_move_time < delay) {
       // Suppress: server hasn't acked AND not enough time elapsed
       return;
   }
   s_last_send_move_time = cur;
   g_PlayerRecvMoveFlag = false;  // Reset flag, wait for next ack
   ```

   `g_PlayerRecvMoveFlag` is set back to `true` when `ZC_NOTIFY_PLAYERMOVE` is received from the
   server. This means:
   - If the server acks quickly (< 500ms), the next packet can be sent immediately on the next
     auto-click (subject to the 150ms throttle).
   - If the server hasn't acked, the client waits up to 500ms before allowing the next packet
     regardless.

Each auto-click goes through the full click pipeline: raycast mouse position to map cell,
run A* pathfinding, send one `CZ_REQUEST_MOVE` packet.

### robrowser

**File:** `src/Engine/MapEngine.js:710-779`

Uses a `setTimeout`-based loop at 500ms interval:

```javascript
function walkIntervalProcess() {
    if (_walkLastTick + 450 > Renderer.tick) {
        return;  // Debounce: skip if < 450ms since last send
    }

    var isWalkable   = (Mouse.world.x > -1 && Mouse.world.y > -1);
    var isCurrentPos = (Math.round(Session.Entity.position[0]) === Mouse.world.x
                     && Math.round(Session.Entity.position[1]) === Mouse.world.y);

    if (isWalkable && !isCurrentPos) {
        var pkt = new PACKET.CZ.REQUEST_MOVE();
        pkt.dest[0] = Mouse.world.x;
        pkt.dest[1] = Mouse.world.y;
        Network.sendPacket(pkt);
    }

    Events.clearTimeout(_walkTimer);
    _walkTimer = Events.setTimeout(walkIntervalProcess, 500);
    _walkLastTick = +Renderer.tick;
}
```

- `onMouseDown` clears any existing timer and calls `walkIntervalProcess()` immediately (sends first packet with no delay).
- The function then schedules itself again after 500ms.
- `onMouseUp` clears `_walkTimer`, ending the loop.
- No server ack gating; only a 450ms tick-based debounce.
- Won't send if cursor is on a non-walkable cell or on the player's current cell.

## Movement Flow & Client-Side Prediction

Both reference clients use client-side prediction: the character starts walking locally
the moment the player clicks, before the server responds.

### Original game flow

1. Player clicks (or auto-click fires) on a map cell.
2. Client runs A* pathfinding from current cell to destination.
3. Client sends `CZ_REQUEST_MOVE` to server with the destination.
4. Client immediately enters `ST_MOVE` state and begins walk animation along the computed path.
5. Server validates the move and responds with `ZC_NOTIFY_PLAYERMOVE`, containing:
   - `moveStartTime`: server timestamp when the move began
   - `MoveData`: encoded source and destination cells
6. Client adjusts animation timing based on server's authoritative timestamps.
7. If the server disagrees with the position (e.g. obstacle, speed hack detection), it sends
   `ZC_STOPMOVE` to force-correct the entity.

For other entities, the server sends `ZC_NOTIFY_MOVE` which includes `GID`, `speed`,
`moveStartTime`, and `moveServerEndTime`. The client runs local pathfinding for these entities
too and animates them with the server-provided speed.

### robrowser flow

Same pattern: send `CZ_REQUEST_MOVE`, start local walk animation immediately, server confirms
with `ZC_NOTIFY_PLAYERMOVE`. Other entities receive `ZC_NOTIFY_MOVE` and are animated locally.

### Interpolation

Both clients perform linear interpolation between path nodes each frame:

- **Speed per cell:** `speed` milliseconds for orthogonal movement (default 150ms).
- **Diagonal penalty:** Diagonal cells take `speed / 0.7` ms (~214ms at default speed).
- **Position each frame:** `current_cell + (next_cell - current_cell) * (elapsed / step_duration)`.
- **Direction:** Computed from delta to next path node. 8 directions (S, SW, W, NW, N, NE, E, SE).

## Stop Move (ZC_STOPMOVE)

The server sends `ZC_STOPMOVE` to force an entity to a specific cell. This happens when the
server needs to correct a desynchronized position (e.g. the client moved through an obstacle,
or the entity was stopped by a skill/stun).

### Original game

**Files:** `GameModePacket.cpp:3935-3950`, `GameActor.cpp:6340-6392`

The handler dispatches `AM_STOPMOVE` to the actor:

```cpp
void CGameMode::Zc_Stopmove(const char *buf) {
    PACKET_ZC_STOPMOVE *p = (PACKET_ZC_STOPMOVE *)buf;
    if (p->AID == g_session.GetAid()) {
        m_world->m_player->SendMsg(NULL, AM_STOPMOVE, p->xPos, p->yPos);
    } else {
        CGameActor *theActor = GetWorld()->GetGameActorByAID(p->AID);
        if (theActor)
            theActor->SendMsg(NULL, AM_STOPMOVE, p->xPos, p->yPos);
    }
}
```

The actor's `AM_STOPMOVE` handler does **not** teleport. Instead it:

1. Converts the current visual position back to cell coordinates.
2. Runs A* pathfinding from the current cell to the stop-move destination cell.
3. If a path is found: enters `ST_MOVE` and walks to the corrected position smoothly.
4. If no path is found: enters `ST_STAND` at the current position.

```cpp
case AM_STOPMOVE: {
    int dx = arg1, dy = arg2;
    int sx, sy, cellX, cellY;
    m_moveStartTime = g_session.GetServerTime();
    gameMode->GetServerCoor(m_pos.x, m_pos.z, sx, sy, cellX, cellY);

    bool result = FindPath(sx, sy, cellX, cellY, dx, dy, 2);
    if (result) {
        m_moveDestX = dx;
        m_moveDestY = dy;
        SetState(ST_MOVE);
        m_pathStartCell = 0;
    } else {
        SetState(ST_STAND);
    }
}
```

This approach avoids visual teleporting and provides smooth correction.

### robrowser

**File:** `src/Engine/MapEngine/Entity.js:244-263`

Simpler approach: directly snaps position and truncates path:

```javascript
function onEntityStopMove(pkt) {
    var entity = EntityManager.get(pkt.AID);
    if (entity) {
        entity.position[0] = pkt.xPos;
        entity.position[1] = pkt.yPos;
        entity.position[2] = Altitude.getCellHeight(pkt.xPos, pkt.yPos);
        entity.walk.index = entity.walk.total;  // Truncate remaining path
        if (entity.action === entity.ACTION.WALK) {
            entity.setAction({
                action: entity.ACTION.IDLE,
                frame: 0, repeat: true, play: true
            });
        }
    }
}
```

This is a hard snap/teleport to the corrected position, followed by state transition to IDLE.

## Actor State Machine

### Original game states

**File:** `GameActor.h:103-128`

```
ST_STAND(0)       - Standing idle
ST_MOVE(1)        - Walking
ST_ATTACK(2)      - Attacking
ST_DEAD(3)        - Dead (terminal, no transitions out)
ST_DAMAGE(4)      - Taking damage / hit stun
ST_PICKUP(5)      - Picking up item
ST_SIT(6)         - Sitting
ST_SKILL(7)       - Casting skill
ST_BEGIN_SKILL(8) - Skill preparation
ST_ATTACK2(9)     - Secondary attack
```

**State transition rules** (`GameActor.cpp:3999-4244`):

The `SetState()` method enforces transition validity:

- **Dead or trick-dead**: No transitions allowed. This is a terminal state.
  ```cpp
  if (ST_DEAD == m_stateId || m_isTrickDead) return;
  ```
- **ST_MOVE**: Unregisters position from cell registry (entity is between cells). Sets looping
  walk animation.
- **ST_STAND**: Registers position in cell registry (entity occupies a cell). Sets looping
  idle animation.
- **ST_DAMAGE**: Plays one-shot damage animation. Duration controlled by `attackedSpeedFactor`.
- State changes also track `m_oldstateId` for stand/sit states, allowing the system to know
  what state to return to after transient states like damage.

### robrowser states

**File:** `src/Renderer/Entity/EntityAction.js:118-154`

Player/PC actions:
```
IDLE(0), WALK(1), SIT(2), PICKUP(3), READYFIGHT(4),
ATTACK1(5), HURT(6), FREEZE(7), DIE(8), FREEZE2(9),
ATTACK2(10), ATTACK3(11), SKILL(12)
```

Monster actions:
```
IDLE(0), WALK(1), ATTACK(2), HURT(3), DIE(4)
```

State transitions are handled through `setAction()` which supports:
- Immediate transitions
- Delayed transitions (`option.delay`)
- Chained transitions (`option.next` — queue the next state after current completes)

## Damage and Movement Interaction

### Original game: damage cancels movement

**File:** `GameActor.cpp:5906-5934`

When `AM_ATTACKED` is received (entity takes damage):

```cpp
if (!recoverDamage) {
    if (IsThisPC(GetJob())) {  // Player character
        if (m_stateId != ST_DEAD && m_stateId != ST_ATTACK
            && message != AM_ATTACKEDNOMOTION) {
            if (wba->is_damage_act == true) {
                if (attackedSpeedFactor > 1) {
                    SetState(ST_DAMAGE);  // INTERRUPTS walking
                    m_loopCountOfmotionFinish = attackedSpeedFactor;
                    m_motionSpeed *= attackedSpeedFactor;
                }
            }
        }
    } else {  // Monster/NPC
        if (m_stateId != ST_DEAD && message != AM_ATTACKEDNOMOTION) {
            if (attackedSpeedFactor > 1) {
                SetState(ST_DAMAGE);
                m_loopCountOfmotionFinish = attackedSpeedFactor;
                m_motionSpeed *= attackedSpeedFactor;
            }
        }
    }
}
```

Key behaviors:
- **Damage interrupts walking**: Entering `ST_DAMAGE` cancels `ST_MOVE`. The entity plays the
  hit reaction animation.
- **Attacking prevents knockback**: If the entity is in `ST_ATTACK`, damage does NOT force
  `ST_DAMAGE`. The entity continues attacking without interruption.
- **Dead prevents all transitions**: `ST_DEAD` blocks any state change.
- **No automatic walk resume**: After the damage animation finishes, the entity returns to
  `ST_STAND`. The player must click again to resume walking.
- **`AM_ATTACKEDNOMOTION`**: Some damage types skip the hit animation entirely (e.g. damage
  over time, reflected damage).
- **`attackedSpeedFactor`**: Controls the duration and intensity of the hit stun animation.
  Higher values = longer stun.

### robrowser: damage pauses then resumes movement

**File:** `src/Engine/MapEngine/Entity.js:1947-1994`

```javascript
function onEntityWillBeHitSub(pkt, dstEntity) {
    if ((pkt.damage > 0 || pkt.leftDamage > 0)
        && pkt.action !== 4 && pkt.action !== 9 && pkt.action !== 11) {

        var count = pkt.count || 1;

        function impendingAttack() {
            if (dstEntity.action !== dstEntity.ACTION.DIE) {
                dstEntity.setAction({
                    action: dstEntity.ACTION.HURT,
                    frame: 0, repeat: false, play: true,
                    next: {
                        action: dstEntity.ACTION.READYFIGHT,
                        delay: pkt.attackedMT,
                        frame: 0, repeat: true, play: true,
                    }
                });
            }
        }

        function resumeWalk() {
            if (dstEntity.action !== dstEntity.ACTION.DIE
                && EntityManager.getFocusEntity()
                && dstEntity.walk.index < dstEntity.walk.total) {
                dstEntity.setAction({
                    action: dstEntity.ACTION.WALK,
                    frame: 0, repeat: false, play: true
                });
            }
        }

        for (var i = 0; i < count; i++) {
            Events.setTimeout(impendingAttack,
                pkt.attackMT + (C_MULTIHIT_DELAY * i));
        }

        // Resume walking after all hits complete
        Events.setTimeout(resumeWalk,
            pkt.attackMT + (C_MULTIHIT_DELAY * count) + pkt.attackedMT);
    }
}
```

Timeline of a damage hit in robrowser:
1. **At `attackMT` ms**: Attacker's motion reaches the target. Set action to `HURT`.
2. **For `attackedMT` ms**: Entity plays hurt animation.
3. **Then**: Transition to `READYFIGHT` (combat-ready idle stance).
4. **Then**: If path is not complete and entity has a focus target, auto-resume `WALK`.

Multi-hit attacks: `C_MULTIHIT_DELAY = 200ms` spacing between hits.

**Key difference from original**: robrowser auto-resumes walking after the damage animation if the
entity's path wasn't fully traversed. The original game does NOT auto-resume; the player must
re-click.

## Pathfinding

All implementations use **A* (A-star)** search:

- **Heuristic**: Manhattan distance (`|dx| + |dy|`) scaled by 10.
- **8-directional**: Supports orthogonal and diagonal movement.
- **Client-local**: The client computes the full path. The server only sends destination (and
  timing for other entities), not intermediate waypoints.
- **Walkability data**: Read from GAT file cells. Each cell has a type flag indicating walkable,
  non-walkable, water, etc.

The original game additionally has a cross-map pathfinding module (`FindPath_Module.cpp`) for
automatic navigation waypoints across map transitions, which is not relevant to standard movement.

## Comparison with Current Implementation

### What works correctly
- Client-side prediction: path is computed and animation starts on click, before server response.
- A* pathfinding with diagonal cost adjustment.
- Linear interpolation between path nodes with correct timing.
- `PlayerMoved` event checks if already moving to same destination to avoid redundant path reset.

### Gaps

| Feature | Original Game | robrowser | Current Client |
|---|---|---|---|
| Mouse-hold walking | 150ms auto-click + 500ms packet throttle + server ack gate | 500ms setTimeout loop + 450ms debounce | Single click only |
| Packet throttle | 500ms + ack flag | 450ms debounce | None |
| `ZC_STOPMOVE` handling | Smooth walk to corrected position | Snap + IDLE | Teleport, doesn't clear moving flag |
| Entity states | ~10 states (stand, move, attack, dead, damage, sit, skill...) | ~13 actions | 3 states (Standing, Moving, Sitting) |
| Damage interrupts walk | Yes, cancels movement | Yes, pauses then auto-resumes | Not implemented |
| Attack prevents knockback | Yes (`ST_ATTACK` blocks `ST_DAMAGE`) | Not explicitly | Not implemented |
| Dead is terminal | Yes, no transitions out | Yes | Not implemented |
| Damage animation | One-shot with configurable duration | Chained HURT -> READYFIGHT with timers | Not implemented |
| Walk resume after damage | Manual (player must re-click) | Automatic if path remains | N/A |
