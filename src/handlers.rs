// Functions that actually handle the protocol
use std::collections::HashMap;
use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};
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
use crate::runner::{self, Runner, settings};
use crate::othello::{
    BoardStruct,
    Player,
    moves::*,
};

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

// Serializes then sends a message across a websocket
pub async fn send_ws_message<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    ws_sender: &mut T,
    msg: ServerMessage,
) -> WSResult<()> 
{
    // explicit error handling everywhere!
    match serde_json::to_string(&msg) {
        Err(why) => {
            // error types everywhere
            Err(WSError::Io(why.into()))
        },
        Ok(server_msg) => {
            info!("Sending out message {}", &server_msg);
            ws_sender.send(WSMessage::Text(server_msg)).await
        }
    }
}

// Sends a message to all the peers of a game
pub fn send_peer_message(
    game_id: &Id,
    room_map: &RoomMap,
    peer_map: &PeerMap,
    msg: ServerMessage,
) -> WSResult<()> {
    let room_map = room_map.lock().unwrap();
    let room = room_map.get(game_id);
    if room.is_none() {
        return Err(WSError::Io(IOError::new(IOErrorKind::NotFound, format!("Tried to send to peers of game id {}, but it wasn't found!", game_id).as_str())));
    }
    let room = room.unwrap();

    let peers = peer_map.lock().unwrap();
    let broadcast_recipients : Vec<Option<&Tx>> = room.watching
        .iter()
        .map(|id| { peers.get(id).clone() })
        .collect();
    
    // FIXME: currently "ignoring" errors here.
    // figure out correct failure mode for when:
    // * tx fails to send
    // * tx doesn't exist
    for recp in broadcast_recipients {
        match recp {
            Some(tx) => {
                if let Err(why) = tx.unbounded_send(msg.clone()) {
                    warn!("{}", why);
                }
            },
            None => {
                warn!("nonexistent peer still in peer list for {}", game_id);
            }
        }
    }

    Ok(())
}


fn make_player(name: &String) -> WSResult<PlayerType> {
    if name == settings::HUMAN_PLAYER {
        return Ok(PlayerType::Human);
    }
    
    let runner = runner::make_runner(name);
    if let Err(why) = runner {
        return Err(WSError::Io(why.into()));
    }

    Ok(PlayerType::Ai(runner.unwrap()))
}

async fn get_move(board: &BoardStruct, player: &Player, timelimit: f32, how: &mut PlayerType) -> Result<usize, IOError> {
    match how {
        PlayerType::Human => Err(IOError::new(IOErrorKind::AddrNotAvailable, "Playing as human not implemented!")),
        PlayerType::Ai(r) => runner::get_move(r, board, player, timelimit).await,
    }
}

async fn tick_game(board: &mut BoardStruct, player: Player, timelimit: f32, black: &mut PlayerType, white: &mut PlayerType) -> Result<Option<Player>, IOError> {
    match player {
        Player::Unknown => Ok(Some(Player::Unknown)),
        Player::Black => {
            let p = Player::Black;
            let square = get_move(board, &p, timelimit, black).await?;
            if let Err(ill) = make_move(&square, &p, board) {
                return Err(IOError::new(IOErrorKind::InvalidInput, format!("{:?}", ill).as_str()));
            }

            Ok(next_player(board, &p))
        },
        Player::White => {
            let p = Player::White;
            let square = get_move(board, &p, timelimit, white).await?;
            if let Err(ill) = make_move(&square, &p, board) {
                return Err(IOError::new(IOErrorKind::InvalidInput, format!("{:?}", ill).as_str()));
            }

            Ok(next_player(board, &p))
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

    let black_name = prq.black.clone();
    let white_name = prq.white.clone();
    // need to be mut because a Runner needs to be mut to send messages
    debug!("{} Making black player {}", &my_id, &black_name);
    let mut black = make_player(&black_name)?; 
    debug!("{} Making white player {}", &my_id, &white_name);
    let mut white = make_player(&white_name)?;
    let timelimit = prq.t;
    
    // room_map is for Ids that are currently playing games
    debug!("{} Inserting room into map", &my_id);
    room_map.lock().unwrap()
        .insert(my_id.clone(), prq.to_room(&my_id));
    // peer_map is for Ids that are watching and expect to receive and mirror messages
    // As we are playing, we don't insert ourselves into it

    let mut ws_fut = ws_receiver.next();
 
    let mut board = BoardStruct::new();
    let mut player = Player::Black;

    loop {
        match &player {
            Player::Unknown => {
                cleanup_room(&my_id, &room_map);
                return Err(WSError::Io(IOError::new(IOErrorKind::InvalidData, format!("Encountered unknown player during game {}", &my_id).as_str())));
            },
            p => {
                debug!("{} Ticking game", &my_id);
                // TODO: if our receiver gets cut off mid-tick (check w/ a select future),
                // then we should quit immediately w/o sending a "game_end" message
                let opt_player = tick_game(&mut board, *p, timelimit, &mut black, &mut white).await?;
                if opt_player.is_none() {
                    // Game is over
                    break;
                } else {
                    player = opt_player.unwrap();
                }
                // that's a lot of data cloning... ah well
                let msg = 
                    ServerMessage::BoardUpdate {
                        board: board.clone(),
                        tomove: player.clone(),
                        black: black_name.clone(),
                        white: white_name.clone()
                    };
                debug!("{} Sending {:?}", &my_id, &msg);
                send_peer_message(&my_id, &room_map, &peer_map, msg.clone())?;
                send_ws_message(&mut ws_sender, msg).await?;
            }
        }
    }

    send_ws_message(
        &mut ws_sender,
        ServerMessage::GameEnd {
            board: board.clone(),
            winner: winner(&board),
            forfeit: false,
        }
    ).await?;

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
    
    send_ws_message(
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
