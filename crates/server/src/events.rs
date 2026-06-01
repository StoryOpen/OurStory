use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

impl SessionId {
    pub fn new() -> Self {
        Self(NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub enum GameEvent {
    Packet {
        session: SessionId,
        opcode: u16,
        payload: Vec<u8>,
    },
    Disconnected {
        session: SessionId,
    },
    PlayerLoaded {
        session: SessionId,
        char_id: i32,
    },
}
