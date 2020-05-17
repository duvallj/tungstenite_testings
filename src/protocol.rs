use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use uuid::Uuid;
// re-export
pub type Id = Uuid;
// private module
use crate::othello::*;

pub struct Room {
    id: Id,
    black_name: String,
    white_name: String,
    timelimit: f32,
    watching: Vec<Id>,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "movereply")]
    MoveReply {square: usize},
    #[serde(rename = "disconnect")]
    Disconnect {},
}

pub mod urls;
