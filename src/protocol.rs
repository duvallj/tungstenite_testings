use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use snowflake::ProcessUniqueId;
// re-export
pub type Id = ProcessUniqueId;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "list_request")]
    ListRequest {},
    #[serde(rename = "play_request")]
    PlayRequest {black: String, white: String, t: f32},
    #[serde(rename = "watch_request")]
    WatchRequest {watching: Id},
    #[serde(rename = "movereply")]
    MoveReply {square: usize},
    #[serde(rename = "disconnect")]
    Disconnect {},
}
