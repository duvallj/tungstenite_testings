// Functions that actually handle the protocol
use serde_json::{
    error::Error as SerdeError
};
use std::collections::HashMap;
use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};
use std::sync::{Arc, Mutex};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{
    pin_mut,
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

const SEND_TIMEOUT : u64 = 10; // Number of seconds to wait before treating send to client as a failure

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

async fn get_move<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    board: &BoardStruct,
    player: &Player,
    timelimit: f32,
    how: &mut PlayerType,
    ws_sender: &mut T,
) -> WSResult<usize> {
    match how {
        PlayerType::Human => Err(WSError::Io(IOError::new(IOErrorKind::AddrNotAvailable, "Playing as human not implemented!"))),
        PlayerType::Ai(r) => {
            match runner::get_move(r, board, player, timelimit).await {
                Ok(res) => Ok(res),
                Err(why) => Err(WSError::Io(why)),
            }
        }
    }
}

async fn tick_game<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    board: &mut BoardStruct,
    player: Player,
    timelimit: f32,
    black: &mut PlayerType,
    white: &mut PlayerType,
    ws_sender: &mut T,
) -> WSResult<Option<Player>> {
    match player {
        Player::Unknown => Ok(Some(Player::Unknown)),
        Player::Black => {
            let p = Player::Black;
            let square = get_move(board, &p, timelimit, black, ws_sender).await?;
            if let Err(ill) = make_move(&square, &p, board) {
                return Err(WSError::Io(IOError::new(IOErrorKind::InvalidInput, format!("{:?}", ill).as_str())));
            }

            Ok(next_player(board, &p))
        },
        Player::White => {
            let p = Player::White;
            let square = get_move(board, &p, timelimit, white, ws_sender).await?;
            if let Err(ill) = make_move(&square, &p, board) {
                return Err(WSError::Io(IOError::new(IOErrorKind::InvalidInput, format!("{:?}", ill).as_str())));
            }

            Ok(next_player(board, &p))
        }
    }
}

async fn tick_game_with_timeout<R: Stream<Item=WSResult<WSMessage>> + StreamExt<Item=WSResult<WSMessage>> + Unpin, T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    board: &mut BoardStruct,
    player: Player,
    timelimit: f32,
    black: &mut PlayerType,
    white: &mut PlayerType,
    ws_sender: &mut T,
    ws_receiver: &mut R,
) -> WSResult<Option<Player>> {
    
    let mut tick_fut = tick_game(board, player, timelimit, black, white, ws_sender);
    pin_mut!(tick_fut); // black magic right here. Delete this to see a very confusing error
    let mut ws_fut = ws_receiver.next();
    
    loop {
        match select(tick_fut, ws_fut).await {
            Either::Left((tick_res, ws_fut_continue)) => {
                ws_fut = ws_fut_continue;
                debug!("Standard case");
                return tick_res;
            },
            Either::Right((ws_res, tick_fut_continue)) => {
                tick_fut = tick_fut_continue;
                match ws_res {
                    Some(Ok(WSMessage::Close(_))) => {
                        // TODO: decide if we actually want to error
                        // here instead of just ending the game. Prob not
                        debug!("Normal error case");
                        return Err(WSError::ConnectionClosed);
                    },
                    Some(Ok(_)) => {
                        // Ignore any other message type 
                        debug!("Ignore case");
                        ws_fut = ws_receiver.next();
                    },
                    Some(Err(why)) => {
                        debug!("Abnormal error case");
                        return Err(why);
                    },
                    None => {
                        // websocket stream has ended w/o close message?
                        // end game as normal ig
                        debug!("stupid werid error case");
                        return Err(WSError::AlreadyClosed);
                    },
                }
            }
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
 
    let mut board = BoardStruct::new();
    let mut player = Player::Black;

    // Closures don't work b/c Rust is """safe"""
    // aSyNc ClOsUrEs ArE uNsTaBlE
    let msg = 
        ServerMessage::BoardUpdate {
            // Initial board message to let client know we have started running
            board: board.clone(),
            tomove: player.clone(),
            black: black_name.clone(),
            white: white_name.clone()
        };
    debug!("{} Sending {:?}", &my_id, &msg);
    send_peer_message(&my_id, &room_map, &peer_map, msg.clone())?;
    send_ws_message(&mut ws_sender, msg).await?;

    // named lifetimes wowee getting fancy are we? (for the named break statements)
    'main: loop {
        match player {
            Player::Unknown => {
                cleanup(&my_id, &room_map, black, white).await?;
                return Err(WSError::Io(IOError::new(IOErrorKind::InvalidData, format!("Encountered unknown player during game {}", &my_id).as_str())));
            },
            p => {
                debug!("{} Ticking game", &my_id);
                
                // if our receiver gets cut off mid-tick, then we should quit 
                // immediately w/o sending a "game_end" message. This very
                // convoluted loop checks for all that
                // FIXME: currently, i don't know how to make it so that the
                // black and white runners aren't twice borrowed, once in the tick game
                // future and once in the cleanup when that doesn't fall through

                match tick_game_with_timeout(
                    &mut board, p, timelimit,
                    &mut black,
                    &mut white,
                    &mut ws_sender,
                    &mut ws_receiver
                ).await {
                    Ok(Some(new_player)) => {
                        player = new_player;
                    },
                    Ok(None) => {
                        break;
                    },
                    Err(why) => {
                        // Potentially send GameError too
                        cleanup(&my_id, &room_map, black, white).await?;

                        return Err(why);
                    },
                }

                // If we successfully get here, that means we know the game
                // has been ticked and the player updated
                
                // That's a lot of clones... ah well
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

    // almost forgot to clean up here :P
    cleanup(&my_id, &room_map, black, white).await?;
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

    // TODO: actually start listening for incomming messages and re-sending them
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

fn cleanup_room(
    id: &Id,
    room_map: &RoomMap,
) -> () {
    room_map.lock().unwrap().remove(id); 
}

async fn cleanup_runner(
    mut runner: Runner
) -> WSResult<()> {
    match runner::kill_and_get_error(runner).await {
        Ok(error_out) => {
            info!("Leftover stderr: {}", error_out);
            Ok(())
        },
        Err(why) => Err(WSError::Io(why))
    }
}

async fn cleanup(
    id: &Id,
    room_map: &RoomMap,
    mut black: PlayerType,
    mut white: PlayerType,
) -> WSResult<()> {
    debug!("{} cleaning up room...", id);
    
    cleanup_room(id, room_map);
    if let PlayerType::Ai(black_ai) = black {
        debug!("Cleaning up black runner...");
        cleanup_runner(black_ai).await?;
        debug!("Done w/ black!");
    }
    if let PlayerType::Ai(white_ai) = white {
        debug!("Cleaning up white runner...");
        cleanup_runner(white_ai).await?;
        debug!("Done w/ white!");
    }

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
