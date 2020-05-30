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
async fn send_ws_message<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    ws_sender: &mut T,
    msg: &ServerMessage,
) -> WSResult<()> 
{
    // explicit error handling everywhere!
    match serde_json::to_string(msg) {
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
fn send_peer_message(
    game_id: &Id,
    room_map: &RoomMap,
    peer_map: &PeerMap,
    msg: &ServerMessage,
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

// Does both of the above sends in one easy function!
async fn send_message<T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    game_id: &Id,
    room_map: &RoomMap,
    peer_map: &PeerMap,
    ws_sender: &mut T,
    msg: &ServerMessage,
) -> WSResult<()> {
    debug!("Sending message {:?}", msg);
    
    match send_ws_message(ws_sender, msg).await {
        Ok(()) => send_peer_message(game_id, room_map, peer_map, msg),
        Err(why) => {
            send_peer_message(game_id, room_map, peer_map,
                &ServerMessage::GameError {
                    error: format!("Error sending original message: {}", why),
                }
            )?;
            send_peer_message(game_id, room_map, peer_map, msg)?;
            Err(why)
        }
    }
}


fn make_player(name: &String) -> WSResult<PlayerType> {
    if name == settings::HUMAN_PLAYER {
        return Ok(PlayerType::Human);
    }
    
    match runner::make_runner(name) {
        Ok(runner) => Ok(PlayerType::Ai(runner)),
        Err(why) => Err(WSError::Io(why.into())),
    }
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

    let (black_name, white_name, timelimit) = (prq.black.clone(), prq.white.clone(), prq.t);
    // need to be mut because a Runner needs to be mut to send messages
    debug!("{} Making black player {}", &my_id, &black_name);
    let mut black = make_player(&black_name)?; 
    debug!("{} Making white player {}", &my_id, &white_name);
    let mut white = match make_player(&white_name) {
        Ok(white_player) => white_player,
        Err(why) => {
            debug!("Error starting white player, clean up black just in case");
            if let (Some(black_err), _) = cleanup(&my_id, &room_map, black, PlayerType::Human).await? {
                let msg = ServerMessage::GameError {
                    error: format!("Error starting white player: {}", why)
                };
                send_message(&my_id, &room_map, &peer_map, &mut ws_sender, &msg).await?;
            }
            return Err(why);
        }
    };

    // room_map is for Ids that are currently playing games
    debug!("{} Inserting room into map", &my_id);
    room_map.lock().unwrap()
        .insert(my_id.clone(), prq.to_room(&my_id));
    // peer_map is for Ids that are watching and expect to receive and mirror messages
    // As we are playing, we don't insert ourselves into it

    // start the main play loop
    let result = play_main(&my_id, &room_map, &peer_map, &mut black, &mut white,
        black_name, white_name, timelimit, ws_sender, ws_receiver).await;

    // Always clean up, no matter if the result is an error or not
    // FIXME: the way it's currently set up, we can't report stderr back to client
    cleanup(&my_id, &room_map, black, white).await?;
    return result;
}

// Main loop that does most of the work of playing a game
// Is not responsible for cleaning up after itself.
// Takes ownership of stream, so any error reporting visible on the client
// must be done here
async fn play_main<R: Stream<Item=WSResult<WSMessage>> + StreamExt<Item=WSResult<WSMessage>> + Unpin, T: Sink<WSMessage, Error=WSError> + SinkExt<WSMessage> + Unpin>(
    my_id: &Id,
    room_map: &RoomMap,
    peer_map: &PeerMap,
    black: &mut PlayerType,
    white: &mut PlayerType,
    black_name: String,
    white_name: String,
    timelimit: f32,
    mut ws_sender: T,
    mut ws_receiver: R
) -> WSResult<()> {
    let mut board = BoardStruct::new();
    let mut player = Player::Black;

    let msg = 
        ServerMessage::BoardUpdate {
            // Initial board message to let client know we have started running
            board: board.clone(),
            tomove: player.clone(),
            black: black_name.clone(),
            white: white_name.clone()
        };
    send_message(my_id, room_map, peer_map, &mut ws_sender, &msg).await?;

    loop {
        match player {
            Player::Unknown => {
                let msg = ServerMessage::GameError {
                    error: "Encoutered unkown player during game! Unrecoverable error".to_string()
                };
                send_message(my_id, room_map, peer_map, &mut ws_sender, &msg).await?;
                
                return Err(WSError::Io(IOError::new(IOErrorKind::InvalidData, format!("Encountered unknown player during game {}", &my_id).as_str())));
            },
            p => {
                debug!("{} Ticking game", &my_id);

                match tick_game_with_timeout(
                    &mut board, p, timelimit,
                    black, white,
                    &mut ws_sender, &mut ws_receiver
                ).await {
                    Ok(Some(new_player)) => {
                        player = new_player;
                    },
                    Ok(None) => {
                        // Game has successfully ended
                        break;
                    },
                    Err(why) => {
                        // Potentially send GameError too
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
                send_message(my_id, room_map, peer_map, &mut ws_sender, &msg).await?;
            }
        }
    }

    let msg = ServerMessage::GameEnd {
        board: board.clone(),
        winner: winner(&board),
        forfeit: false,
    };
    send_message(my_id, room_map, peer_map, &mut ws_sender, &msg).await?;
    
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
        &ServerMessage::ListReply {room_list: simplified_map}
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
) -> WSResult<String> {
    match runner::kill_and_get_error(runner).await {
        Ok(error_out) => {
            info!("Leftover stderr: {}", &error_out);
            Ok(error_out)
        },
        Err(why) => Err(WSError::Io(why))
    }
}

async fn cleanup(
    id: &Id,
    room_map: &RoomMap,
    mut black: PlayerType,
    mut white: PlayerType,
) -> WSResult<(Option<String>, Option<String>)> {
    debug!("{} cleaning up room...", id);
    cleanup_room(id, room_map);

    match (black, white) {
        (PlayerType::Ai(black_ai), PlayerType::Ai(white_ai)) => {
            debug!("Cleaning up both runners...");
            match futures::future::join(cleanup_runner(black_ai), cleanup_runner(white_ai)).await {
                (Ok(black_error), Ok(white_error)) => Ok((Some(black_error), Some(white_error))),
                (Err(black_err), _) => Err(black_err),
                (_, Err(white_err)) => Err(white_err),
            }
        },
        (PlayerType::Ai(black_ai), PlayerType::Human) => {
            debug!("Cleaning up just black runner...");
            Ok((Some(cleanup_runner(black_ai).await?), None))
        },
        (PlayerType::Human, PlayerType::Ai(white_ai)) => {
            debug!("Cleaning up just white runner...");
            Ok((None, Some(cleanup_runner(white_ai).await?)))
        },
        (PlayerType::Human, PlayerType::Human) => {
            debug!("No runners to clean up!");
            Ok((None, None))
        }
    }
}

fn unwrap_incomming_message(msg: WSMessage) -> WSResult<ClientMessage> {
    let text = msg.to_text()?;
    let parsed: Result<ClientMessage, SerdeError> = serde_json::from_str(text);
    match parsed {
        Ok(client_msg) => Ok(client_msg),
        Err(error) => Err(WSError::Io(error.into())),
    }
}
