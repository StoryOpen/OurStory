use crate::config::Config;
use server_core::channel::Channel;
use server_core::map::{LocalMapHandle, Map};
use server_core::world::World;
use protocol::types::{ChannelId, MapId, WorldId};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run(config: Config) {
    let world_cfg = &config.worlds[0];
    let world = Arc::new(RwLock::new(World::new(WorldId(world_cfg.id), &world_cfg.name)));

    // Create channels with maps
    {
        let mut w = world.write().await;
        for ch_cfg in &world_cfg.channels {
            let mut channel = Channel::new(ChannelId(ch_cfg.id), WorldId(world_cfg.id));
            // Pre-create some maps for each channel
            for map_id in [10000, 10001, 20000] {
                channel.add_map(
                    MapId(map_id),
                    Box::new(LocalMapHandle::new(Map::new(MapId(map_id)))),
                );
            }
            w.add_channel(ChannelId(ch_cfg.id), channel);
        }
    }

    tracing::info!(
        "channel server for world '{}' ready ({} channels)",
        world_cfg.name,
        world_cfg.channels.len()
    );

    // TODO: bind TCP acceptor, route packets
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
