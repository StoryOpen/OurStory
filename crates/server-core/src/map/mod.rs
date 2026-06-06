mod instance;
mod snapshot;

pub use handle::{LocalMapHandle, MapHandle, SessionHandle};
pub use instance::Map;
pub use snapshot::PlayerSnapshot;

pub mod handle {
    use protocol::packets::MoveAction;
    use protocol::types::{PlayerId, Position};
    use std::sync::Arc;

    pub trait MapHandle: Send + Sync {
        fn add_player(&self, snapshot: super::PlayerSnapshot, session: SessionHandle);
        fn remove_player(&self, player_id: PlayerId);
        fn broadcast(&self, packet: &[u8], except: Option<PlayerId>);
        fn move_player(&self, player_id: PlayerId, position: Position, moves: Vec<MoveAction>);
        fn player_count(&self) -> usize;
    }

    #[derive(Clone)]
    pub struct SessionHandle {
        pub send: Arc<dyn Send + Sync + Fn(&[u8])>,
    }

    impl std::fmt::Debug for SessionHandle {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "SessionHandle")
        }
    }

    impl SessionHandle {
        pub fn new(send: impl Send + Sync + 'static + Fn(&[u8]) -> ()) -> Self {
            Self {
                send: Arc::new(send),
            }
        }

        pub fn send(&self, packet: &[u8]) {
            (self.send)(packet)
        }
    }

    pub struct LocalMapHandle {
        map: Arc<tokio::sync::RwLock<super::Map>>,
    }

    impl LocalMapHandle {
        pub fn new(map: super::Map) -> Self {
            Self {
                map: Arc::new(tokio::sync::RwLock::new(map)),
            }
        }

        pub fn from_arc(map: Arc<tokio::sync::RwLock<super::Map>>) -> Self {
            Self { map }
        }
    }

    impl MapHandle for LocalMapHandle {
        fn add_player(&self, snapshot: super::PlayerSnapshot, session: SessionHandle) {
            let mut map = self.map.blocking_write();
            map.add_player(snapshot, session);
        }

        fn remove_player(&self, player_id: PlayerId) {
            let mut map = self.map.blocking_write();
            map.remove_player(player_id);
        }

        fn broadcast(&self, packet: &[u8], except: Option<PlayerId>) {
            let map = self.map.blocking_read();
            map.broadcast(packet, except);
        }

        fn move_player(&self, player_id: PlayerId, position: Position, moves: Vec<MoveAction>) {
            let mut map = self.map.blocking_write();
            map.move_player(player_id, position, moves);
        }

        fn player_count(&self) -> usize {
            let map = self.map.blocking_read();
            map.player_count()
        }
    }
}
