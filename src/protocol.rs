use serde::{Serialize, Deserialize};

use uid::Id as IdT;
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ClientIdType(());
pub type Id = IdT<ClientIdType>;

// private module
use crate::othello::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    BoardUpdate {},
    MoveRequest {},
    GameEnd {},
    GameError {},
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "list_request")]
    ListRequest {},
    #[serde(rename = "play_request")]
    PlayRequest {},
    #[serde(rename = "watch_request")]
    WatchRequest {},
    #[serde(rename = "movereply")]
    MoveReply {},
    #[serde(rename = "disconnect")]
    Disconnect {},
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    Server(ServerMessage),
    Client(ClientMessage),
}
