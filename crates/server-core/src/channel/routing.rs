use protocol::types::{MapId, PlayerId};
use std::collections::HashMap;

pub struct Router {
    player_to_map: HashMap<PlayerId, MapId>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            player_to_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, player_id: PlayerId, map_id: MapId) {
        self.player_to_map.insert(player_id, map_id);
    }

    pub fn unregister(&mut self, player_id: PlayerId) -> Option<MapId> {
        self.player_to_map.remove(&player_id)
    }

    pub fn lookup(&self, player_id: PlayerId) -> Option<MapId> {
        self.player_to_map.get(&player_id).copied()
    }

    pub fn clear(&mut self) {
        self.player_to_map.clear();
    }
}
