use crate::events::SessionId;
use server_core::db::Database;
use server_core::world::World;
use std::sync::Arc;

pub async fn handle_select(
    _world: &mut World,
    _db: &Arc<dyn Database>,
    _session: SessionId,
    _payload: &[u8],
) {
}
