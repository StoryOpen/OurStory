use crate::map::handle::SessionHandle;
use crate::map::PlayerSnapshot as Snapshot;
use protocol::packets::MoveAction;
use protocol::types::{MapId, PlayerId, Position};
use std::collections::HashMap;

pub struct Map {
    pub id: MapId,
    players: HashMap<PlayerId, PlayerEntry>,
    mobs: Vec<MobEntry>,
    drops: Vec<DropEntry>,
}

struct PlayerEntry {
    snapshot: Snapshot,
    session: SessionHandle,
    position: Position,
}

struct MobEntry {
    mob_id: i32,
    template_id: i32,
    position: Position,
    hp: i32,
    max_hp: i32,
}

struct DropEntry {
    item_id: i32,
    position: Position,
    owner: Option<PlayerId>,
}

impl Map {
    pub fn new(id: MapId) -> Self {
        Self {
            id,
            players: HashMap::new(),
            mobs: Vec::new(),
            drops: Vec::new(),
        }
    }

    pub fn add_player(&mut self, snapshot: Snapshot, session: SessionHandle) {
        let pos = snapshot.position;
        self.players.insert(
            snapshot.id,
            PlayerEntry {
                snapshot,
                session,
                position: pos,
            },
        );
    }

    pub fn remove_player(&mut self, player_id: PlayerId) -> Option<(Snapshot, SessionHandle)> {
        let entry = self.players.remove(&player_id)?;
        Some((entry.snapshot, entry.session))
    }

    pub fn move_player(
        &mut self,
        player_id: PlayerId,
        position: Position,
        _moves: Vec<MoveAction>,
    ) {
        if let Some(entry) = self.players.get_mut(&player_id) {
            entry.position = position;
            entry.snapshot.position = position;
        }
    }

    pub fn broadcast(&self, packet: &[u8], except: Option<PlayerId>) {
        for (id, entry) in &self.players {
            if Some(*id) != except {
                entry.session.send(packet);
            }
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }
}
