mod routing;

use crate::map::{LocalMapHandle, Map, MapHandle, PlayerSnapshot, SessionHandle};
use protocol::types::{ChannelId, MapId, PlayerId, WorldId};
use std::collections::HashMap;

pub struct Channel {
    pub id: ChannelId,
    pub world_id: WorldId,
    maps: HashMap<MapId, Box<dyn MapHandle>>,
    routing: routing::Router,
}

impl Channel {
    pub fn new(id: ChannelId, world_id: WorldId) -> Self {
        Self {
            id,
            world_id,
            maps: HashMap::new(),
            routing: routing::Router::new(),
        }
    }

    pub fn add_map(&mut self, map_id: MapId, handle: Box<dyn MapHandle>) {
        self.maps.insert(map_id, handle);
    }

    pub fn get_or_create_map(&mut self, map_id: MapId) -> &mut Box<dyn MapHandle> {
        self.maps
            .entry(map_id)
            .or_insert_with(|| Box::new(LocalMapHandle::new(Map::new(map_id))))
    }

    pub fn add_player(
        &mut self,
        player_id: PlayerId,
        map_id: MapId,
        snapshot: PlayerSnapshot,
        session: SessionHandle,
    ) {
        self.routing.register(player_id, map_id);
        if let Some(map) = self.maps.get(&map_id) {
            map.add_player(snapshot, session);
        }
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        if let Some(map_id) = self.routing.unregister(player_id) {
            if let Some(map) = self.maps.get(&map_id) {
                map.remove_player(player_id);
            }
        }
    }

    pub fn switch_map(
        &mut self,
        player_id: PlayerId,
        target_map: MapId,
        snapshot: PlayerSnapshot,
        session: SessionHandle,
    ) {
        if let Some(old_map_id) = self.routing.unregister(player_id) {
            if let Some(old_map) = self.maps.get(&old_map_id) {
                old_map.remove_player(player_id);
            }
        }
        self.routing.register(player_id, target_map);
        self.get_or_create_map(target_map)
            .add_player(snapshot, session);
    }

    pub fn map_for_player(&self, player_id: PlayerId) -> Option<MapId> {
        self.routing.lookup(player_id)
    }

    pub fn broadcast(&self, packet: &[u8]) {
        for map in self.maps.values() {
            map.broadcast(packet, None);
        }
    }
}
