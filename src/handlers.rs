// Functions that actually handle the protocol
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{
    sink::Sink,
    stream::Stream,
    SinkExt,
    StreamExt,
    future::{
        select,
        Either,
    },
};
use log::*;
use std::marker::Unpin;
use tokio::io::{
    AsyncRead,
    AsyncWrite,
};
use tokio_tungstenite::WebSocketStream;
use tungstenite::{
    Message as WSMessage,
    Result as WSResult,
    error::Error as WSError,
};

use crate::protocol::*;
use crate::runner::*;
use crate::othello::BoardStruct;

type Tx = UnboundedSender<ServerMessage>;
pub type PeerMap = Arc<Mutex<HashMap<Id, Tx>>>;

pub enum PlayerType {
    Human,
    Ai(Runner),
}

pub struct Game {
    black: PlayerType,
    white: PlayerType,
    board: BoardStruct,
}

pub async fn handle_incoming_message(
    id: &Id,
    room_map: &RoomMap,
    client_msg: ClientMessage,
) {
    match client_msg {
        ClientMessage::MoveReply {square} => {
            // TODO: you know the drill
            info!("move_reply on square {} from {}", square, id);
        },
        ClientMessage::Disconnect {} => {
            info!("disconnect signaled from {}", id);
        },
    }
}

pub async fn send_outgoing_message<T: Sink<WSMessage, Error = WSError> + SinkExt<WSMessage> + Unpin>(
    ws_sender: &mut T,
    msg: ServerMessage,
) -> WSResult<()> 
{
    // explicit error handling everywhere!
    match serde_json::to_string(&msg) {
        Err(why) => {
            error!("Error serializing message {:?}: {}", msg, why);
            // Stupid error types everywhere
            // FIXME: actually error once i find more about "associated types"
            Ok(())
        },
        Ok(server_msg) => {
            info!("Sending out message {}", &server_msg);
            ws_sender.send(WSMessage::Text(server_msg)).await
        }
    }
}


// I need to do this "multiple functions" thing instead of impl some trait for 
// each of the request structs because Rust doesn't support async fns in traits
// yet :(
//
// Also, associated type is whack b/c I can't directly use WebSocketStream ???
pub async fn play<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Stream<Item=WSResult<WSMessage>> + StreamExt<Item=WSResult<WSMessage>> + Unpin>(
    prq: PlayRequest,
    room_map: RoomMap,
    peer_map: PeerMap,
    mut ws_stream: T,
) -> WSResult<()> {
    let my_id = Id::new_v4(); // guaranteed to be unique
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // room_map is for Ids that are currently playing games
    room_map.lock().unwrap()
        .insert(my_id.clone(), prq.to_room(&my_id));
    // peer_map is for Ids that are watching and expect to receive and mirror messages
    // As we are playing, we don't insert ourselves into it

    let mut ws_fut = ws_receiver.next();
    
    loop {
        break;
    }
    Ok(())
}

pub async fn watch<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Stream<Item=WSResult<WSMessage>> + StreamExt<Item=WSResult<WSMessage>> + Unpin>(
    wrq: WatchRequest,
    room_map: RoomMap,
    peer_map: PeerMap,
    mut ws_stream: T,
) -> WSResult<()> {
    let my_id = Id::new_v4(); // guaranteed to be unique
    // New scope so we don't keep holding on to lock
    {
        let mut rooms = room_map.lock().unwrap();
        let watch_id : Id = wrq.into();

        match rooms.get_mut(&watch_id) {
            Some(room) => {
                room.watching.push(my_id.clone());
            },
            None => {
                // TODO: error somehow
                warn!("Client {} tried to watch non-existent room {}!", my_id, watch_id);
            },
        };
    }
    Ok(())
}


pub async fn list<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Stream<Item=WSResult<WSMessage>> + StreamExt<Item=WSResult<WSMessage>> + Unpin>(
    _lrq: ListRequest,
    room_map: RoomMap,
    mut ws_stream: T,
) -> WSResult<()> {
    let simplified_map : HashMap<Id, ExternalRoom> = 
        room_map.lock().unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.into()))
        .collect();
    
    send_outgoing_message(
        &mut ws_stream,
        ServerMessage::ListReply {room_list: simplified_map}
    ).await
    
    // websocket is closed as soon as our handler finishes, nice!
}

pub async fn handle_initial_request<T: SinkExt<WSMessage> + std::marker::Unpin>(
    id: &Id,
    room_map: &RoomMap,
    ws_sender: &mut T,
    request: &Option<ClientRequest>,
) {
    match request {
        Some(ClientRequest::List(lrq)) => {
        },
        Some(ClientRequest::Play(prq)) => {
        },
        Some(ClientRequest::Watch(wrq)) => {
        }
        _ => {}
    }
}

pub fn cleanup_room(
    id: &Id,
    room_map: &RoomMap,
) {
   room_map.lock().unwrap().remove(id); 
   // TODO: do any other tasks required to stop a running game if we are 
   // removing an item
}
