use serde_json::error::{Error as SerdeError};
use tokio::io::{
    AsyncBufReadExt,
    AsyncReadExt,
    AsyncWriteExt,
    BufReader,
    Result as IOResult,
    Error as IOError,
    ErrorKind as IOErrorKind,
};
use tokio::stream::StreamExt;
use tokio::time::{timeout, Duration};

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
                stderr: BufReader::new(stderr),
                ai_name: ai_name.clone(),
            })
        }
    }
}

fn serialize_request(board: &BoardStruct, player: &Player, timelimit: f32, ai_name: &String) -> Result<String, SerdeError> {
    let mut board_str = serde_json::to_string(board)?;
    let mut player_str = serde_json::to_string(player)?;
    let timelimit_str = serde_json::to_string(&timelimit)?;
    
    // Remove quotes added by serde
    board_str.retain(|c| c != '"');
    player_str.retain(|c| c != '"');

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

    let timeout_fut = timeout(
        // Add 1 sec to timeout here to account for overhead of communication
        Duration::from_millis((timelimit * 1000.0 + 1000.0) as u64),
        runner.stdout.next()
    );
    // TODO: add some way to time out this await in case the JailedRunner hangs for whatever reason
    match timeout_fut.await {
        Ok(Some(reply)) => {
            let reply = reply?;
            log::debug!("Got line \"{}\" from subprocess", &reply);
            let square : Result<usize, SerdeError> = serde_json::from_str(&reply);
            if let Err(why) = square {
                return Err(why.into());
            }
            let square : usize = square.unwrap();
            
            Ok(square)
        },
        Ok(None) => {
            Err(IOError::new(IOErrorKind::BrokenPipe, "Stream ended when trying to read reply from runner!"))
        },
        Err(_) => {
            Err(IOError::new(IOErrorKind::TimedOut, "Stream timed out when trying to read reply from runner!"))
        }
    }
}

pub async fn kill_and_get_error(mut runner: Runner) -> IOResult<String> {
    runner.child.kill()?;
    // Can't use the wait_with_output command here because we have taken the
    // stream away from the child object.
    // So instead we just use an AsyncReadExt method ourselves
    let mut error_output = String::new();
    runner.stderr.read_to_string(&mut error_output).await?;

    Ok(error_output)
}
