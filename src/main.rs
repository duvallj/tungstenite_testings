use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use futures_util::future::{select, Either};
use futures_util::{SinkExt, StreamExt};
use log::*;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error};
use tungstenite::{Message, Result};
use futures_channel::mpsc::{unbounded, UnboundedSender};

use uid::Id as IdT;
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
struct T(());
type Id = IdT<T>;

type Tx = UnboundedSender<Message>;
#[derive(Clone, Debug)]
struct ConnectionData {
    addr: SocketAddr,
    tx: Tx,
}

type PeerMap = Arc<Mutex<HashMap<Id, ConnectionData>>>;

async fn accept_connection(peer_map: PeerMap, addr: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer_map, addr, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer_map: PeerMap, addr: SocketAddr, stream: TcpStream) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    info!("New WebSocket connection: {}", &addr);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = unbounded();
    let my_id = Id::new(); // guaranteed to be unique
    let data = ConnectionData {
        addr: addr,
        tx: tx
    };
    // put data into shared context
    peer_map.lock().unwrap().insert(my_id, data);

    // Echo incoming WebSocket messages and send a message periodically every second.

    let mut ws_fut = ws_receiver.next();
    let mut internal_fut = rx.next();
    loop {
        match select(ws_fut, internal_fut).await {
            Either::Left((msg, internal_fut_continue)) => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text() || msg.is_binary() {
                            // If we receive a message, send it to all our peers
                            let peers = peer_map.lock().unwrap();
                            let broadcast_recipients = peers
                                .iter()
                                .filter(|(other_id, _)| other_id != &&my_id)
                                .map(|(_, conn_data)| conn_data.tx.clone());
                            for recp in broadcast_recipients {
                                recp.unbounded_send(msg.clone()).unwrap();
                            }
                            // ws_sender.send(msg).await?;
                        } else if msg.is_close() {
                            break;
                        }
                        internal_fut = internal_fut_continue; // Continue waiting for tick.
                        ws_fut = ws_receiver.next(); // Receive next WebSocket message.
                    }
                    None => break, // WebSocket stream terminated.
                };
            }
            Either::Right((msg, ws_fut_continue)) => {
                match msg {
                    Some(msg) => {
                        ws_sender.send(msg).await?;
                        ws_fut = ws_fut_continue; // Continue receiving the WebSocket message.
                        internal_fut = rx.next(); // Wait for next tick.
                    }
                    None => break, // Something went wrong ig
                };
            }
        }
    }

    // Remove id from map b/c it is no longer valid
    peer_map.lock().unwrap().remove(&my_id);
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let context = PeerMap::new(Mutex::new(HashMap::new()));

    let addr = "127.0.0.1:9002";
    let mut listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(context.clone(), peer, stream));
    }
}
