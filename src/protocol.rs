use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tungstenite::{
    Message as WSMessage,
    Result as WSResult,
    error::Error as WSError,
};
use futures_util::SinkExt;
use log::*;

use uuid::Uuid;
// re-export
pub type Id = Uuid;
// TODO: eventual performance issues maybe if everything is stored
// in one hashmap. idk, should be mostly fine for the scales we deal with
pub type RoomMap = Arc<Mutex<HashMap<Id, Room>>>;

// private module
use crate::othello::*;

#[derive(Clone, Debug)]
pub struct Room {
    pub id: Id,
    pub black_name: String,
    pub white_name: String,
    pub timelimit: f32,
    pub watching: Vec<Id>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalRoom {
    black: String,
    white: String,
    timelimit: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "list_reply")]
    ListReply {room_list: HashMap<Id, ExternalRoom>},
    #[serde(rename = "board_update")]
    BoardUpdate {board: BoardStruct, tomove: Player, black: String, white: String},
    #[serde(rename = "move_request")]
    MoveRequest {},
    #[serde(rename = "game_end")]
    GameEnd {board: BoardStruct, winner: Player, forfeit: bool},
    #[serde(rename = "game_error")]
    GameError {error: String},
    #[serde(rename = "disconect")]
    Disconnect {},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayRequest {black: String, white: String, t: f32}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchRequest {pub watching: Id}

// TODO: potentially have optional fields on this, make into another enum?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientRequest {
    Play(PlayRequest),
    Watch(WatchRequest),
    List(ListRequest),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "movereply")]
    MoveReply {square: usize},
    #[serde(rename = "disconnect")]
    Disconnect {},
}

pub mod urls;
mod conversions;

// Functions to actually handle the protocol
pub async fn handle_incoming_message(
    id: &Id,
    room_map: &RoomMap,
    client_msg: ClientMessage,
) -> Option<ServerMessage> {
    match client_msg {
        ClientMessage::MoveReply {square} => {
            // TODO: you know the drill
            info!("move_reply on square {} from {}", square, id);
            None
        },
        ClientMessage::Disconnect {} => {
            info!("disconnect signaled from {}", id);
            None
        },
    }
}

pub async fn send_outgoing_message<T: SinkExt<WSMessage> + std::marker::Unpin>(
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

pub async fn handle_initial_request<T: SinkExt<WSMessage> + std::marker::Unpin>(
    id: &Id,
    room_map: &RoomMap,
    ws_sender: &mut T,
    request: Option<ClientRequest>,
) {
    match request {
        Some(ClientRequest::List(lrq)) => {
            /*let rooms = room_map.lock().unwrap();
            let owned_rooms = rooms.iter().to_owned();*/
            let simplified_map : HashMap<Id, ExternalRoom> = 
                room_map.lock().unwrap().iter()
                .map(|(k, v)| (k.clone(), v.into()))
                .collect();
            send_outgoing_message(ws_sender, ServerMessage::ListReply {room_list: simplified_map}).await;
        },
        Some(ClientRequest::Play(prq)) => {
            let mut rooms = room_map.lock().unwrap();
            // TODO: handle case where room already exists (somehow) and end it
            rooms.insert(
                id.clone(),
                prq.to_room(id)
            );
        },
        Some(ClientRequest::Watch(wrq)) => {
            let mut rooms = room_map.lock().unwrap();
            
            match rooms.get_mut(&wrq.watching) {
                Some(room) => {
                    room.watching.push(id.clone());
                },
                None => {
                    // TODO: error somehow
                    warn!("Client {} tried to watch non-existent room {}!", id, wrq.watching);
                },
            };
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
