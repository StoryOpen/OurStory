use crate::events::{GameEvent, SessionId};
use crate::handlers;
use crate::net::session::OutboundTx;
use server_core::db::Database;
use server_core::world::World;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct SessionRegistry {
    map: HashMap<SessionId, OutboundTx>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, id: SessionId, tx: OutboundTx) {
        self.map.insert(id, tx);
    }

    pub fn remove(&mut self, id: SessionId) -> Option<OutboundTx> {
        self.map.remove(&id)
    }

    #[allow(dead_code)]
    pub fn send(&self, id: SessionId, packet: Vec<u8>) {
        if let Some(tx) = self.map.get(&id) {
            let _ = tx.try_send(packet);
        }
    }
}

pub async fn run(
    world: Arc<RwLock<World>>,
    db: Arc<dyn Database>,
    mut events: mpsc::Receiver<GameEvent>,
) {
    let mut sessions = SessionRegistry::new();

    while let Some(event) = events.recv().await {
        match event {
            GameEvent::Packet { session, opcode, payload } => {
                let mut w = world.write().await;
                handlers::handle_packet(&mut w, &db, session, opcode, &payload).await;
            }
            GameEvent::Disconnected { session } => {
                sessions.remove(session);
                let mut w = world.write().await;
                handlers::handle_disconnect(&mut w, session);
            }
            GameEvent::PlayerLoaded { session, char_id } => {
                let mut w = world.write().await;
                if let Some(player) = db.load_player(char_id).await {
                    w.register_player(player);
                }
                let _ = session;
            }
        }
    }
}
