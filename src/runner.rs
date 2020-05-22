use serde_json::error::{Error as SerdeError};
use tokio::io::{
    AsyncBufReadExt,
    AsyncWriteExt,
    BufReader,
    Result as IOResult,
    Error as IOError,
    ErrorKind as IOErrorKind,
};
use tokio::stream::StreamExt;

pub mod structs;
pub mod settings;
// Re-export structs
pub use structs::*;

use crate::othello::{
    BoardStruct,
    Player,
};

pub fn make_runner(ai_name: &String) -> IOResult<Runner> {
    let mut command = settings::build_jailed_command(ai_name)?;
    let mut child = command.spawn()?;
    match (child.stdin.take(), child.stdout.take(), child.stderr.take()) {
        (None, _, _) => {
            Err(IOError::new(IOErrorKind::BrokenPipe, "Could not open stdin on subprocess!"))
        },
        (_, None, _) => {
            Err(IOError::new(IOErrorKind::BrokenPipe, "Could not open stdout on subprocess!"))
        },
        (_, _, None) => {
            Err(IOError::new(IOErrorKind::BrokenPipe, "Could not open stderr on subprocess!"))
        },
        (Some(stdin), Some(stdout), Some(stderr)) => {
            Ok(Runner {
                child: child,
                stdin: stdin,
                stdout: BufReader::new(stdout).lines(),
                stderr: BufReader::new(stderr).lines(),
                ai_name: ai_name.clone(),
            })
        }
    }
}

fn serialize_request(board: &BoardStruct, player: &Player, timelimit: f32, ai_name: &String) -> Result<String, SerdeError> {
    let board_str = serde_json::to_string(board)?;
    let player_str = serde_json::to_string(player)?;
    let timelimit_str = serde_json::to_string(&timelimit)?;

    // This is the format that the JailedRunner python code expects
    let to_send = format!("{}\n{}\n{}\n{}\n", ai_name, timelimit_str, player_str, board_str);

    Ok(to_send)
}

pub async fn get_move(runner: &mut Runner, board: &BoardStruct, player: &Player, timelimit: f32) -> IOResult<usize> {
    let to_send = serialize_request(board, player, timelimit, &runner.ai_name);
    if let Err(why) = to_send {
        // there is an impl From<SerdeError> for io::Error, nice!
        return Err(why.into()); 
    }
    let to_send = to_send.unwrap();

    runner.stdin.write_all(to_send.as_bytes()).await?;

    // TODO: add some way to time out this await in case the JailedRunner hangs for whatever reason
    match runner.stdout.next().await {
        Some(reply) => {
            let reply = reply.unwrap();
            let square : Result<usize, SerdeError> = serde_json::from_str(&reply);
            if let Err(why) = square {
                // Read from stderr, but somehow also know when it ends?
                return Err(why.into());
            }
            let square : usize = square.unwrap();
            
            // TODO: read from stderr as well, report it
            Ok(square)
        },
        None => {
            Err(IOError::new(IOErrorKind::BrokenPipe, "Stream ended when trying to read reply from runner!"))
        }
    }
}
