pub mod models;

pub trait Database: Send + Sync {
    fn load_player(&self, id: i32) -> Option<crate::player::Player>;
    fn save_player(&self, player: &crate::player::Player);
}
