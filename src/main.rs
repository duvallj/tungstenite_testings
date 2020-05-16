use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    env,
};
use futures_util::future::{select, Either};
use futures_util::{sink::Sink, SinkExt, StreamExt};
use log::*;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error};
use tungstenite::{Message as WSMessage, Result as WSResult, error::Error as WSError};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use serde_json::Result as SerdeResult;
use serde_json::error::Error as SerdeError;

// Private modules
mod othello;
mod protocol;
use crate::protocol::*;

type Tx = UnboundedSender<ServerMessage>;
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

async fn handle_connection(peer_map: PeerMap, addr: SocketAddr, stream: TcpStream) -> WSResult<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (mut tx, mut rx) = unbounded();
    let my_id = Id::new(); // guaranteed to be unique
    let data = ConnectionData {
        addr: addr,
        tx: tx.clone()
    };
    // put data into shared context
    peer_map.lock().unwrap().insert(my_id, data);
    info!("New WebSocket connection {} assigned id {}", &addr, &my_id);

    let mut ws_fut = ws_receiver.next();
    let mut internal_fut = rx.next();
    loop {
        match select(ws_fut, internal_fut).await {
            Either::Left((ws_msg, internal_fut_continue)) => {
                match ws_msg {
                    Some(msg) => {
                        let msg = msg?;
                        info!("Received message {:?}", &msg);
                        if msg.is_text() {
                            // Attempt to decode message as json w/ Serde
                            let client_msg = unwrap_incomming_message(msg)?;
                            handle_incoming_message(&peer_map, &my_id, &mut tx, client_msg).await;
                            // If we receive a message, send it to all our peers
                            /* let peers = peer_map.lock().unwrap();
                            let broadcast_recipients = peers
                                .iter()
                                .filter(|(other_id, _)| other_id != &&my_id)
                                .map(|(_, conn_data)| conn_data.tx.clone());
                            for recp in broadcast_recipients {
                                recp.unbounded_send(msg.clone()).unwrap();
                            } */
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
            Either::Right((internal_msg, ws_fut_continue)) => {
                match internal_msg {
                    Some(msg) => {
                        send_outgoing_message(&mut ws_sender, msg).await;
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

fn unwrap_incomming_message(msg: WSMessage) -> WSResult<ClientMessage> {
    let text = msg.to_text()?;
    let parsed: Result<ClientMessage, SerdeError> = serde_json::from_str(text);
    match parsed {
        Ok(client_msg) => Ok(client_msg),
        Err(error) => Err(WSError::Io(error.into())),
    }
}
async fn handle_incoming_message(
    peer_map: &PeerMap, 
    room_id: &Id, 
    tx: &mut Tx,
    client_msg: ClientMessage,
) {
    match client_msg {
        ClientMessage::ListRequest {} => {
            // TODO: figure out how/where to store a list of
            // currently running games
            info!("list_request from {}", room_id);
        },
        ClientMessage::PlayRequest {black, white, t} => {
            // TODO: actually start a game running somewhere when this is received
            info!("play_request for {} vs {} ({}) from {}", black, white, t, room_id);
        },
        ClientMessage::WatchRequest {watching} => {
            // TODO: actually hook up things so that it watches the game
            info!("watch_request to watch {} from {}", watching, room_id);
        },
        ClientMessage::MoveReply {square} => {
            // TODO: you know the drill
            info!("move_reply on square {} from {}", square, room_id);
        },
        ClientMessage::Disconnect {} => {
            info!("disconnect signaled from {}", room_id);
        },
    }
}

async fn send_outgoing_message<T: SinkExt<WSMessage> + std::marker::Unpin>(
    ws_sender: &mut T,
    msg: ServerMessage,
) {
    // stupid explicit error handling everywhere!
    match serde_json::to_string(&msg) {
        Err(why) => {
            error!("Error serializing message {:?}: {}", msg, why);
            // Stupid error types everywhere
            // FIXME: once i find more about "associated types"
        },
        Ok(server_msg) => {
            // FIXME: explicitly handle errors here
            info!("Sending out message {}", &server_msg);
            ws_sender.send(WSMessage::Text(server_msg)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let context = PeerMap::new(Mutex::new(HashMap::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
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
