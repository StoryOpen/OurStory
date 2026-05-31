use crate::channel::Channel;
use crate::player::{Buff, Player};
use protocol::types::{ChannelId, PlayerId, WorldId};
use std::collections::HashMap;

pub struct World {
    pub id: WorldId,
    pub name: String,
    channels: HashMap<ChannelId, Channel>,
    players: HashMap<PlayerId, Player>,
    buff_storage: BuffStorage,
}

impl World {
    pub fn new(id: WorldId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            channels: HashMap::new(),
            players: HashMap::new(),
            buff_storage: BuffStorage::new(),
        }
    }

    pub fn add_channel(&mut self, id: ChannelId, channel: Channel) {
        self.channels.insert(id, channel);
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.get_mut(&id)
    }

    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.get(&id)
    }

    pub fn channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.values()
    }

    pub fn register_player(&mut self, player: Player) {
        self.players.insert(player.id, player);
    }

    pub fn unregister_player(&mut self, player_id: PlayerId) -> Option<Player> {
        self.players.remove(&player_id)
    }

    pub fn get_player(&self, player_id: PlayerId) -> Option<&Player> {
        self.players.get(&player_id)
    }

    pub fn get_player_mut(&mut self, player_id: PlayerId) -> Option<&mut Player> {
        self.players.get_mut(&player_id)
    }

    pub fn store_buffs(&self, player_id: PlayerId, buffs: Vec<Buff>) {
        self.buff_storage.store(player_id, buffs);
    }

    pub fn take_buffs(&self, player_id: PlayerId) -> Option<Vec<Buff>> {
        self.buff_storage.take(player_id)
    }

    pub fn broadcast(&self, packet: &[u8]) {
        for channel in self.channels.values() {
            channel.broadcast(packet);
        }
    }
}

struct BuffStorage {
    inner: std::sync::Mutex<HashMap<PlayerId, Vec<Buff>>>,
}

impl BuffStorage {
    fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn store(&self, player_id: PlayerId, buffs: Vec<Buff>) {
        let mut map = self.inner.lock().unwrap();
        map.insert(player_id, buffs);
    }

    fn take(&self, player_id: PlayerId) -> Option<Vec<Buff>> {
        let mut map = self.inner.lock().unwrap();
        map.remove(&player_id)
    }
}
