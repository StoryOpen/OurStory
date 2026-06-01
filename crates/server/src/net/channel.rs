use crate::config::Config;
use crate::events::GameEvent;
use crate::net::{dispatcher, listener};
use protocol::types::{ChannelId, MapId, WorldId};
use server_core::channel::Channel;
use server_core::db::postgres::PostgresDb;
use server_core::map::{LocalMapHandle, Map};
use server_core::world::World;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tracing::info;
use tracing_subscriber::EnvFilter;

pub async fn run(config: Config) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let db = match PostgresDb::connect(&config.db_url).await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            tracing::error!(?e, "failed to connect to postgres");
            return;
        }
    };
    if let Err(e) = db.run_migrations().await {
        tracing::error!(?e, "migration failed");
        return;
    }

    let world_cfg = &config.worlds[0];
    let world = Arc::new(RwLock::new(World::new(
        WorldId(world_cfg.id),
        &world_cfg.name,
    )));

    {
        let mut w = world.write().await;
        for ch_cfg in &world_cfg.channels {
            let mut channel = Channel::new(ChannelId(ch_cfg.id), WorldId(world_cfg.id));
            for map_id in [10000, 10001, 20000] {
                channel.add_map(
                    MapId(map_id),
                    Box::new(LocalMapHandle::new(Map::new(MapId(map_id)))),
                );
            }
            w.add_channel(ChannelId(ch_cfg.id), channel);
        }
    }

    let (event_tx, event_rx) = mpsc::channel::<GameEvent>(1024);
    let db_for_dispatcher: Arc<dyn server_core::db::Database> = db.clone();
    let world_for_dispatcher = world.clone();
    tokio::spawn(async move {
        dispatcher::run(world_for_dispatcher, db_for_dispatcher, event_rx).await;
    });

    for ch_cfg in &world_cfg.channels {
        let listener = TcpListener::bind(("0.0.0.0", ch_cfg.port))
            .await
            .expect("bind failed");
        let event_tx = event_tx.clone();
        let port = ch_cfg.port;
        info!(port, "channel listening");
        tokio::spawn(listener::run(listener, event_tx));
    }

    info!(
        world = %world_cfg.name,
        "channel server ready"
    );

    std::future::pending::<()>().await;
}
