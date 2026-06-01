use crate::events::SessionId;
use protocol::opcodes::RecvOpcode;
use server_core::db::Database;
use server_core::world::World;
use std::sync::Arc;

pub mod character;
pub mod login;
pub mod movement;

pub async fn handle_packet(
    world: &mut World,
    db: &Arc<dyn Database>,
    session: SessionId,
    opcode: u16,
    payload: &[u8],
) {
    let Some(op) = RecvOpcode::from_u16(opcode) else { return };
    match op {
        RecvOpcode::LoginPassword => login::handle(world, db, session, payload).await,
        RecvOpcode::CharacterSelect => character::handle_select(world, db, session, payload).await,
        RecvOpcode::PlayerMove => movement::handle(world, session, payload),
        _ => {}
    }
}

pub fn handle_disconnect(world: &mut World, session: SessionId) {
    let _ = (world, session);
}
