use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PacketTrace {
    All,
    Unhandled,
    #[default]
    None,
}

impl PacketTrace {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => PacketTrace::All,
            1 => PacketTrace::Unhandled,
            _ => PacketTrace::None,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            PacketTrace::All => 0,
            PacketTrace::Unhandled => 1,
            PacketTrace::None => 2,
        }
    }
}

static PACKET_TRACE: AtomicU8 = AtomicU8::new(2);
static TRACE_EFFECTS: AtomicBool = AtomicBool::new(false);
static TRACE_INPUT: AtomicBool = AtomicBool::new(false);
static TRACE_TEXTURE_LOAD: AtomicBool = AtomicBool::new(false);

pub fn init(packet: PacketTrace, effects: bool, input: bool, texture_load: bool) {
    PACKET_TRACE.store(packet.as_u8(), Ordering::Relaxed);
    TRACE_EFFECTS.store(effects, Ordering::Relaxed);
    TRACE_INPUT.store(input, Ordering::Relaxed);
    TRACE_TEXTURE_LOAD.store(texture_load, Ordering::Relaxed);
}

pub fn packet_trace() -> PacketTrace {
    PacketTrace::from_u8(PACKET_TRACE.load(Ordering::Relaxed))
}

pub fn trace_effects() -> bool {
    TRACE_EFFECTS.load(Ordering::Relaxed)
}

pub fn trace_input() -> bool {
    TRACE_INPUT.load(Ordering::Relaxed)
}

pub fn trace_texture_load() -> bool {
    TRACE_TEXTURE_LOAD.load(Ordering::Relaxed)
}
