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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

// fiiine, we'll make these struct fields public
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayRequest {pub black: String, pub white: String, pub t: f32}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "movereply")]
    MoveReply {square: usize},
    #[serde(rename = "disconnect")]
    Disconnect {},
}

impl From<Room> for ExternalRoom {
    fn from(r: Room) -> Self {
        ExternalRoom {
            black: r.black_name,
            white: r.white_name,
            timelimit: r.timelimit,
        }
    }
}

impl From<&Room> for ExternalRoom {
    fn from(r: &Room) -> Self {
        ExternalRoom {
            black: r.black_name.clone(),
            white: r.white_name.clone(),
            timelimit: r.timelimit,
        }
    }
}

impl PlayRequest {
    pub fn to_room(self, id: &Id) -> Room {
        Room {
            id: id.clone(),
            black_name: self.black,
            white_name: self.white,
            timelimit: self.t,
            watching: Vec::new(),
        }
    }
}

impl From<WatchRequest> for Id {
    fn from(wrq: WatchRequest) -> Self {
        wrq.watching
    }
}

impl From<&WatchRequest> for Id {
    fn from(wrq: &WatchRequest) -> Self {
        wrq.watching.clone()
    }
}
