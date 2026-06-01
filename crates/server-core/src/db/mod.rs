pub mod models;
pub mod postgres;

use crate::player::Player;

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn load_player(&self, id: i32) -> Option<Player>;
    async fn save_player(&self, player: &Player);
}
