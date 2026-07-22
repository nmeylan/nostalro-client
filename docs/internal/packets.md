# Packets: the junk-padding era and packetver bands

This document explains why some client-to-server packets carry meaningless
filler bytes, why two reference server implementations describe those bytes
differently, what changed around 2010 that removed the problem, and how the
client models the whole thing.

It reflects the code as it stands. When the code and this document disagree, the
code is right and this document is stale; fix it.

## What the junk is

Gravity clients from roughly 2004 to 2010 shipped a crude anti-third-party
measure. For each client-to-server packet the client did two things:

- Reassigned the packet id on almost every client patch. The same logical
  request (walk here, pick up that item) travels under different ids in
  different client builds.
- Inserted a run of garbage bytes between the real fields, and changed the
  length and placement of that garbage on almost every client patch.

The garbage carries no information. The server never reads it. It exists so that
a bot or unofficial client that hardcodes a fixed byte layout breaks the moment
the official client is patched.

A request therefore looks different on the wire depending on which client built
it. A walk request from one client build:

```
id      pad          dest
a7 00 | 00 00 00 | 00 26 cb      (8 bytes)
```

The `a7 00` is the id, the three `00` bytes are junk, the last three bytes are the
packed destination. In another client build the id differs and the junk is a
different count.

## Client lines share dates

Gravity shipped several client lines in parallel. The ones that matter for the
junk era are Sakexe (the Sakray test client, patched most frequently) and Ragexe
and RagexeRE (the main and renewal clients). Later there are also AD and Zero
lines.

The lines share calendar dates but carry different layouts. A single date can
have both a `2008-09-10aSakexe` build and a `2008-09-10aRagexeRE` build with
different junk. A bare date is therefore ambiguous until the line is named.

## How servers encode layouts

A server cannot parse these packets generically, so it hardcodes per client
build the id, total length, and byte offset of every real field.

rathena encodes this in `src/map/clif_packetdb.hpp`:

```
parseable_packet(0x00a7, 8, clif_parse_WalkToXY, 5);
```

Id `0x00a7` is a walk request, 8 bytes long, destination at offset 5; the bytes
between the 2-byte header and offset 5 are junk. Each registration sits in a
`#if PACKETVER >= <date>` block; later blocks override earlier ones.

rathena uses one flat `PACKETVER` integer. `clif_packetdb.hpp` lists the Sakexe
timeline first, then a Renewal section guarded by `#ifdef PACKETVER_RE_NUM`. A
build that defines only `PACKETVER` preprocesses the Renewal section out, so
every guard resolves against the Sakexe timeline:

```
// 2008-09-10aSakexe
#if PACKETVER >= 20080910
	parseable_packet(0x0437,7,clif_parse_ActionRequest,2,6);
#endif

// Renewal Clients
#ifdef PACKETVER_RE_NUM
// 2008-08-27aRagexeRE
#if PACKETVER_RE_NUM >= 20080827
	parseable_packet(0x00a7,9,clif_parse_WalkToXY,6);   // compiled out on a plain build
#endif
#endif
```

Conclusion: a plain rathena `PACKETVER` build follows the Sakexe line. The
RagexeRE line only exists when `PACKETVER_RE_NUM` is defined, which is not a
configuration rathena supports for dates this old.

Hercules encodes the same information in `src/map/packets.hpp` and
`packets_struct.h`, but keeps the lines apart with separate version numbers:
`PACKETVER_MAIN_NUM`, `PACKETVER_RE_NUM`, `PACKETVER_SAK_NUM`,
`PACKETVER_AD_NUM`, `PACKETVER_ZERO`. A guard reads `PACKETVER_RE_NUM >= 20080827`,
so Hercules attributes each boundary to a specific line.

## Why the two tables differ

Because Hercules tracks the lines with distinct version numbers and rathena
exposes only the Sakexe line under a flat `PACKETVER`, a table derived from
Hercules can carry RagexeRE boundaries that a plain rathena build never uses. The
walk request shows the divergence:

| client line | date     | walk id | length | junk bytes |
| ----------- | -------- | ------- | ------ | ---------- |
| Sakexe      | 20080910 | 0x00a7  | 8      | 3          |
| RagexeRE    | 20080827 | 0x00a7  | 9      | 4          |

There is no universal correct layout for a junk-era packet. The junk carries no
meaning, so the only requirements are that the real field values land at the
offsets the server reads and that the total length matches the server's table.
Correctness is defined by the server we connect to. For the junk era that server
is a plain-`PACKETVER` rathena, so its Sakexe-line `clif_packetdb.hpp` is the
authority, not a Hercules-derived table.

## What changed around 2010

From client date 20101124 onward the junk padding disappears. Field offsets
become canonical, ids stabilise, and the client lines converge. The walk request
becomes id `0x035f` at 20101124 with the destination at offset 2 and no filler,
then `0x0437` at 20120307, both 5 bytes.

Modern and junk-era packetvers therefore coexist in one table without conflict:
they occupy disjoint version bands. Editing a junk-era band does not affect a
post-2010 packetver.

## Feature gating by packetver

Independent of the junk, packetver gates client features, because features were
introduced at specific client dates and both client and server branch on
packetver for them. Two boundaries:

- `20100413`: the extended character-slot fields
  (`TotalSlotNum`, `PremiumStartSlot`, `PremiumEndSlot`) appear in
  `HC_ACCEPT_ENTER 0x006b`. Below this date the old fixed-slot layout applies.
- `20120307`: character creation switches to the stat-less packet `0x0970`.
  Below this date creation carries the six stats for the player to allocate.

## How the client models it

`packets_db` (in `../rust-ragnarok-server/tools/packets/`) describes each packet
as an id ladder plus versioned fields. The junk is expressed as `#V RANGE` filler
fields between the real ones:

```
0x0437, 0x0085:20040705, ..., 0x00a7:20041129, 0x035f:20101124, ...
struct PACKET_CZ_REQUEST_MOVE {
  short PacketId
  ...
  #V RANGE 20070108 20090408 unsigned byte pad14[3]
  #V RANGE 20090408 20101124 unsigned byte pad15[4]
  unsigned byte dest[3]
 }
```

The id ladder maps a packetver to an id. Each `#V RANGE A B` field is emitted
only when `A <= packetver < B`; the upper bound is exclusive.

The generator (`packet_struct_generator.rs`) turns each `#V RANGE` into a
conditional branch. The builder emits only the fields whose band contains the
packetver, so the wire length is correct for that date. This requires a concrete
packetver:

- `fill_raw_with_packetver(Some(pv))` emits only the fields whose band contains
  `pv`. Correct.
- `fill_raw()` (packetver `None`) emits every conditional field, producing an
  oversized packet on any struct with `#V` fields and desyncing the stream.
  Junk-era packets must use the `Some(pv)` form.

Regenerate after editing `packets_db`:

```
cd ../rust-ragnarok-server && cargo run --package tools --bin packets-tool
```

The client picks up the result through the `[patch]` in `Cargo.toml`.

## Packets that carry junk bands

Any packet with `#V RANGE` filler fields is a junk-era packet. The client-to-
server packets the client sends during login, map enter, and normal play, with
their rathena handlers:

| packet                          | rathena handler                |
| ------------------------------- | ------------------------------ |
| CZ_ENTER2                       | clif_parse_WantToConnection    |
| CZ_REQUEST_MOVE                 | clif_parse_WalkToXY            |
| CZ_REQUEST_TIME                 | clif_parse_TickSend            |
| CZ_CHANGE_DIRECTION             | clif_parse_ChangeDir           |
| CZ_REQUEST_ACT                  | clif_parse_ActionRequest       |
| CZ_REQNAME                      | clif_parse_GetCharNameRequest  |
| CZ_REQNAME_BYGID                | clif_parse_SolveCharName       |
| CZ_ITEM_PICKUP                  | clif_parse_TakeItem            |
| CZ_ITEM_THROW                   | clif_parse_DropItem            |
| CZ_USE_ITEM                     | clif_parse_UseItem             |
| CZ_MOVE_ITEM_FROM_STORE_TO_BODY | clif_parse_MoveFromKafra       |
| CZ_MOVE_ITEM_FROM_BODY_TO_STORE | clif_parse_MoveToKafra         |
| CZ_USE_SKILL                    | clif_parse_UseSkillToId        |
| CZ_USE_SKILL_TOGROUND           | clif_parse_UseSkillToPos       |

The server-to-client (ZC) packets have the same structure and are not fully
reconciled. A mismatch surfaces when the server sends a packet the client
mis-parses.

## Reconciling a band against the server

To verify or fix a junk-era packet, map its rathena handler registrations to
their `#if PACKETVER` guards and pick the one active at the target packetver:

```
active = none
for each parseable_packet(id, len, handler, off0, ...) in clif_packetdb.hpp, file order:
    guard = nearest enclosing "#if PACKETVER >= V"
    if V <= target_packetver:
        active = (id, len, off0)      # later blocks override earlier ones

pad_before_first_field = active.off0 - 2      # 2 is the id header
```

The `packets_db` band for the target packetver must reproduce `active.id`,
`active.len`, and the same field offsets. A one-line awk over `clif_packetdb.hpp`
lists the whole ladder for a handler:

```
awk '/#if PACKETVER >=/ {ver=$0} /clif_parse_WalkToXY/ {print ver"  "$0}' \
    src/map/clif_packetdb.hpp
```

## The rule

For any packet in the pre-2010 junk era, the target server's flat-`PACKETVER`
`clif_packetdb.hpp` (the Sakexe line) is the authority for id, length, and field
offsets. Verify with the offset-map method, fix the `#V RANGE` bands in
`packets_db`, regenerate, and confirm the sent length matches the server. Do not
seed junk-era bands from Hercules `packets.hpp`; it tracks the RagexeRE line.
Post-2010 bands are canonical and shared across implementations; leave them alone.
