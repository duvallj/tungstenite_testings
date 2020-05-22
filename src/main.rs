use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Mutex},
    env,
};
use log::*;
use serde_json::{
    error::Error as SerdeError
};
use tokio::net::{
    TcpListener,
    TcpStream
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::Error,
};
use tungstenite::{
    Message as WSMessage,
    Result as WSResult,
    error::Error as WSError,
};

// Private modules
mod othello;
mod protocol;
mod runner;
mod handlers;
use crate::protocol::*;
use handlers::PeerMap;

async fn accept_connection(room_map: RoomMap, peer_map: PeerMap, addr: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(room_map, peer_map, addr, stream).await {
        match e {
            Error::ConnectionClosed | Error::AlreadyClosed => (),
            err => error!("Error processing connection: {:?}", err),
        }
    }
}

async fn handle_connection(room_map: RoomMap, peer_map: PeerMap, addr: SocketAddr, stream: TcpStream) -> WSResult<()> {
    let mut request_type: Option<ClientRequest> = None;

    let ws_stream = accept_hdr_async(
        stream,
        |request: &http::Request<()>, response: http::Response<()>| {
            match protocol::urls::parse_uri(request.uri().clone()) {
                Ok(rq) => {
                    request_type = Some(rq);

                    Ok(response)
                },
                Err(why) => {
                    let err_msg = format!("Error parsing request: {}", why);
                    // This error gets turned into type Error::Protocol, is logged later
                    error!("{}", &err_msg);

                    let (mut parts, _) = response.into_parts();
                    parts.status = http::StatusCode::BAD_REQUEST;
                    Err(http::Response::from_parts(parts, Some(err_msg)))
                }
            }
        }
    ).await?;

    match request_type {
        Some(ClientRequest::Play(prq)) => {
            handlers::play(prq, room_map, peer_map, ws_stream).await
        },
        Some(ClientRequest::Watch(wrq)) => {
            handlers::watch(wrq, room_map, peer_map, ws_stream).await
        },
        Some(ClientRequest::List(lrq)) => {
            handlers::list(lrq, room_map, ws_stream).await
        },
        None => {
            // TODO: error somehow b/c this shouldn't be possible
            Err(Error::Protocol(std::borrow::Cow::from("Something went wrong; failed to parse request type but fell through anyways")))
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


#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let watchers = PeerMap::new(Mutex::new(HashMap::new()));
    let players = RoomMap::new(Mutex::new(HashMap::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let mut listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(players.clone(), watchers.clone(), peer, stream));
    }
}
