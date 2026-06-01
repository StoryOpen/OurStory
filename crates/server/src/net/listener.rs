use crate::events::GameEvent;
use crate::net::session;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::info;

pub async fn run(listener: TcpListener, events: mpsc::Sender<GameEvent>) {
    let local = listener.local_addr().ok();
    info!(?local, "listening");

    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                info!(?peer, "accepted");
                let events = events.clone();
                tokio::spawn(session::run(socket, events));
            }
            Err(e) => {
                tracing::warn!(?e, "accept failed");
            }
        }
    }
}
