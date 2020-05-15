use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures::{
    channel::mpsc::{self, unbounded, UnboundedSender, UnboundedReceiver},
    future::{self, TryFuture, TryFutureExt}, pin_mut,
    stream::{self, StreamExt, TryStreamExt, SplitSink, SplitStream},
    sink::{self, SinkExt},//, Sink},
};

use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;
use tungstenite::error::Error;
use tokio_tungstenite::WebSocketStream;
use snowflake::ProcessUniqueId;

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Result<Message, mpsc::SendError>>;

struct Connection {
    uid: ProcessUniqueId,
    incoming: Rx,
    outgoing: Tx,
}

type ConnectionMap = Arc<Mutex<HashMap<ProcessUniqueId, Connection>>>;
/*
impl From<tungstenite::error::Error> for mpsc::SendError {
    fn from(error: tungstenite::error::Error) -> Self {
        match error {
            Error::ConnectionClosed |
            Error::AlreadyClosed |
            Error::Io(err) |
            Error::Tls(err) |
            Error::Utf8 => {
                mpsc::SendError {
                    kind: mpsc::SendErrorKind::Disconnected
                }
            },
            _ => {
                mpsc::SendError {
                    kind: mpsc::SendErrorKind::Full
                }
            }
        }
    }
}*/

fn attach_echo(connection: Connection, addr: SocketAddr) -> impl TryFutureExt + 'static {
    let (incoming, outgoing) = (connection.incoming, connection.outgoing);
    
    let reflect_incoming = incoming.forward(outgoing)
        .and_then(|_| {
            println!("This case is actually more worrisome");
            future::ok(())
        })
        .or_else(|err| {
            println!("Stream closed with error \"{}\"", err);
            future::err(err)
        });

    reflect_incoming
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) -> (Connection, impl future::Future) {
    println!("Incoming TCP connection from: {}", addr);
    let uid = ProcessUniqueId::new();
    println!("Assigning id {} to {}", uid, addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (ws_outgoing, ws_incoming) = ws_stream.split();

    let wrapped_outgoing = ws_outgoing
    .with::<_, _, _, Error>(move |msg: Message| {
        let raw_msg = msg.to_text().unwrap();
        println!(
            "Sending a message to {}: {}",
            &addr,
            raw_msg
        );

        future::ok(msg)
    });

    let wrapped_incoming = ws_incoming
    .map_ok(move |msg: Message| {
        let raw_msg = msg.to_text().unwrap();
        println!(
            "Received a message from {}: {}",
            &addr,
            raw_msg
        );

        msg
    })
    .into_stream()
    .map(|i: Result<Message, tungstenite::error::Error>| {
        match i {
            Ok(msg) => msg,
            Err(_) => Message::Close(None),
        }
    });

    
    let (tx_out, rx_out) = unbounded();
    let (tx_in, rx_in) = unbounded();

    let forward_to_outgoing = rx_out.map(Ok).forward(wrapped_outgoing);
    let forward_from_incoming = wrapped_incoming.map(Ok).map(Ok).forward(tx_in);

    let conn = Connection {
        uid: ProcessUniqueId::new(),
        incoming: rx_in,
        outgoing: tx_out,
    };

    (conn, future::select(forward_to_outgoing, forward_from_incoming))
}

async fn full_handle_connection(
        connection_map: ConnectionMap,
        raw_stream: TcpStream,
        addr: SocketAddr)
{
    let (conn, forward_fut) = handle_connection(raw_stream, addr.clone()).await;
    let echo_fut = attach_echo(conn, addr.clone());
    future::select(forward_fut, echo_fut.into_future()).await;
    println!("{} disconnected", addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let state = ConnectionMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let mut listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(full_handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}
