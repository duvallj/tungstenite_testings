use futures_channel::mpsc::UnboundedSender;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
// re-export
pub type Id = Uuid;
// TODO: eventual performance issues maybe if everything is stored
// in one hashmap. idk, should be mostly fine for the scales we deal with
pub type RoomMap = Arc<Mutex<HashMap<Id, Room>>>;
type Tx = UnboundedSender<ServerMessage>;
pub type PeerMap = Arc<Mutex<HashMap<Id, Tx>>>;

// private module
use crate::othello::*;

#[derive(Clone, Debug)]
pub struct Room {
    pub id: Id,
    pub black_name: String,
    pub white_name: String,
    pub timelimit: f32,
    pub watching: Vec<Id>,
    pub tx: Tx,
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
pub struct WatchRequest {watching: Id}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "movereply")]
    MoveReply {square: usize},
    #[serde(rename = "disconnect")]
    Disconnect {},
}
