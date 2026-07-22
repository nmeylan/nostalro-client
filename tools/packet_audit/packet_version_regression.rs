//! Receive-side packet-version regression tests. One assertion set per packet
//! fixed by the ZC remediation plan; guards the version-gated lengths (and the
//! 20120307 anchor) against accidental packets_db edits.
#![cfg(test)]

use crate::handler::dispatch_packet;
use packets::packets::*;
use packets::packets_parser::parse;
use ragnarok_game::event::GameEvent;


#[test]
fn req_wear_equip_ack_view_id_gated() {
    assert_eq!(PacketZcReqWearEquipAck::base_len(20101122), 7);
    assert_eq!(PacketZcReqWearEquipAck::base_len(20101123), 9);
    assert_eq!(PacketZcReqWearEquipAck::base_len(20120307), 9);
}

#[test]
fn property_pet_job_gated() {
    assert_eq!(PacketZcPropertyPet::base_len(20081125), 35);
    assert_eq!(PacketZcPropertyPet::base_len(20081126), 37);
    assert_eq!(PacketZcPropertyPet::base_len(20120307), 37);
}

// ---- Tier B ----
// Flagged by the length audit but verified already-correct (fixed-count nested
// arrays; the audit's fill_raw undercounted empty Vecs). Guard the strides.

#[test]
fn quest_and_shortcut_nested_array_lengths() {
    // 17 header + 3 * 30 (MISSION_HUNT)
    assert_eq!(PacketZcAddQuest::base_len(20120307), 107);
    // 2 header + 27/38 * 7 (ShortCutKey)
    assert_eq!(PacketZcShortcutKeyList::base_len(20120307), 191);
    assert_eq!(PacketZcShortcutKeyListV2::base_len(20120307), 268);
}

// ---- Tier D: MoveEntry family ----
// MoveData is uint8[6] (packed from/to position), not u16[6]; objecttype only
// exists from 20071106 (matters inside the 0x22c era).

#[test]
fn moveentry_movedata_and_objecttype_lengths() {
    assert_eq!(PacketZcNotifyMoveentry::base_len(20120307), 60);
    assert_eq!(PacketZcNotifyMoveentry2::base_len(20120307), 60);
    assert_eq!(PacketZcNotifyMoveentry3::base_len(20070212), 64);
    assert_eq!(PacketZcNotifyMoveentry3::base_len(20071106), 65);
    assert_eq!(PacketZcNotifyMoveentry4::base_len(20080827), 67);
}

// ---- [20101124, 20120221) entry variants (0x856/0x857/0x858) ----
// These ids were previously dropped (no parser arm); now handled via id-ladder on
// the 0x7f7-era structs + robe gated at 20101124.
#[test]
fn entry_variants_20101124_dispatch_and_robe() {
    assert_eq!(
        PacketZcNotifyMoveentry7::base_len(20101123) + 2,
        PacketZcNotifyMoveentry7::base_len(20101124),
        "robe must appear at 20101124"
    );
    let ver = 20111102;
    for (lo, hi) in [(0x56u8, 0x08u8), (0x57, 0x08), (0x58, 0x08)] {
        let mut buf = vec![0u8; 256];
        buf[0] = lo;
        buf[1] = hi;
        let p = parse(&buf, ver);
        assert!(
            p.as_any().downcast_ref::<PacketUnknown>().is_none(),
            "id 0x{:02x}{:02x} still dropped at {}",
            hi,
            lo,
            ver
        );
    }
}

// ---- pre-2008 cleanup ----
// MER_INIT gained call-num + kill-counter (8 bytes) at 20071106.
#[test]
fn mer_init_call_kill_counters_gated() {
    assert_eq!(PacketZcMerInit::base_len(20070212), 72);
    assert_eq!(PacketZcMerInit::base_len(20071106), 80);
    assert_eq!(PacketZcMerInit::base_len(20120307), 80);
}

// Verified already-correct (audit false positives): 0x00fd shares one rathena/
// Hercules struct with 0x02c5 but the old wire has a 1-byte result (clif=27);
// 0x0245's "7" belongs to a different pre-mail packet reusing the id at 20050718.
#[test]
fn ack_req_join_group_and_mail_get_item_lengths() {
    assert_eq!(PacketZcAckReqJoinGroup::base_len(20120307), 27);
    assert_eq!(PacketZcMailReqGetItem::base_len(20120307), 3);
}

// Every version-gated ZC family the server emits inside the classic band must
// dispatch (not fall through to PacketUnknown). These ids are the send-path truth
// at 20120307 (clif.cpp headers / active symbolic-enum values), verified against
// the actual send sites -- skill-unit uses 0x011f here, NOT the enum's 0x8c7.
#[test]
fn version_gated_zc_ids_dispatch_at_anchor() {
    let ver = 20120307;
    let ids: &[u16] = &[
        0x011f, // skill unit on ground (send path hardcodes 0x011f <= 20120702)
        0x009e, // item fall entry (dropflooritemType)
        0x01d7, // sprite change (sendLookType)
        0x043f, // status change (status_changeType)
        0x02b1, // all quest list (questListType)
        0x02b3, // add quest (questAddType)
        0x02b5, // update mission hunt (questUpdateType)
        0x02d0, // equipment itemlist (inventorylistequipType)
        0x02d1, // storage equipment itemlist (storageListEquipType)
        0x011c, // warplist (skilWarpPointType)
        0x00ac, // takeoff equip ack (unequipitemackType)
        // clif.cpp #if-PACKETVER header switches, both in-band branches:
        0x013e, 0x07fb, // skill-cast ack (switch @ 20091124)
        0x0101, 0x07d8, // party groupinfo change (switch @ 20090603)
        0x0199, // map property (else branch < 20121010)
        0x02e0, // battlefield notify hp (< 20140613)
        0x07f6, // notify exp (else branch < 20170830)
    ];
    for &id in ids {
        let mut buf = vec![0u8; 512];
        buf[0] = (id & 0xff) as u8;
        buf[1] = (id >> 8) as u8;
        let p = parse(&buf, ver);
        assert!(
            p.as_any().downcast_ref::<PacketUnknown>().is_none(),
            "server-emitted id 0x{:04x} dropped at {}",
            id,
            ver
        );
    }
}

#[test]
fn req_wear_equip_ack_parses_pre_20101123_layout() {
    let ver = 20080827;
    // 0x00aa, index=5, wearLocation=2, result=1 -- 7 bytes, no viewId
    let raw = vec![0xaa, 0x00, 5, 0, 2, 0, 1];
    let parsed = parse(&raw, ver);
    let events = dispatch_packet(parsed.as_ref(), ver);
    assert!(matches!(
        events.as_slice(),
        [GameEvent::InventoryEquipResult {
            index: 5,
            wear_location: 2,
            view_id: 0,
            success: true
        }]
    ));
}
