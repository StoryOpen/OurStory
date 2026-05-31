use server_core::map::handle::SessionHandle;
use server_core::player::Player;
use tokio::sync::mpsc;

pub struct Session {
    pub tx: mpsc::Sender<Vec<u8>>,
    pub player: Option<Player>,
}

impl Session {
    pub fn handle(&self) -> SessionHandle {
        let tx = self.tx.clone();
        SessionHandle::new(move |packet| {
            let _ = tx.try_send(packet.to_vec());
        })
    }
}
