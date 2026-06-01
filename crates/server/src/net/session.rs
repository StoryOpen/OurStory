use crate::events::{GameEvent, SessionId};
use crate::net::codec::MapleCodec;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tracing::warn;

pub type OutboundTx = mpsc::Sender<Vec<u8>>;

pub struct Session {
    #[allow(dead_code)]
    pub id: SessionId,
    #[allow(dead_code)]
    pub outbound_tx: OutboundTx,
}

pub async fn run(socket: TcpStream, events: mpsc::Sender<GameEvent>) {
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(256);
    let id = SessionId::new();
    let _session = Session { id, outbound_tx };

    let (read_half, mut write_half) = socket.into_split();
    let mut framed = Framed::new(read_half, MapleCodec::new());

    let writer = tokio::spawn(async move {
        while let Some(packet) = outbound_rx.recv().await {
            if let Err(e) = write_half.write_all(&packet).await {
                warn!(?e, "write failed");
                break;
            }
        }
    });

    while let Some(result) = framed.next().await {
        match result {
            Ok((opcode, payload)) => {
                if events
                    .send(GameEvent::Packet {
                        session: id,
                        opcode,
                        payload: payload.to_vec(),
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(e) => {
                warn!(?e, "decode error");
                break;
            }
        }
    }

    writer.abort();
    let _ = events.send(GameEvent::Disconnected { session: id }).await;
}
