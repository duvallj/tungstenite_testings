use tokio::process::{
    Child,
    ChildStdin,
    ChildStdout,
    ChildStderr,
};
use tokio::io::{
    AsyncBufReadExt,
    BufReader,
    BufWriter,
    Lines,
    Result as IOResult,
    Error as IOError,
    ErrorKind as IOErrorKind,
};

pub mod structs;
use structs::*;
mod settings;

use crate::protocol::*;

pub fn make_runner(ai_name: &String) -> IOResult<(Child, RunnerStdin, RunnerStdout, RunnerStderr)> {
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
            Ok((child, stdin, BufReader::new(stdout).lines(), BufReader::new(stderr).lines()))
        }
    }
}

pub async fn get_move(runner: &Runner) -> IOResult<usize> {
    // TODO: implement the functionality to send a move to the runner
    // and await its result
    Ok(0)
}
