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

// Private module
mod settings;

pub type RunnerStdin = BufWriter<ChildStdin>;
pub type RunnerStdout = Lines<BufReader<ChildStdout>>;
pub type RunnerStderr = Lines<BufReader<ChildStderr>>;

pub fn make_runner(ai_name: &String) -> IOResult<(Child, RunnerStdin, RunnerStdout, RunnerStderr)> {
    let mut command = settings::build_jailed_command(ai_name)?;
    let child = command.spawn()?;
    match (&child.stdin, &child.stdout, &child.stderr) {
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
            Ok((child, BufWriter::new(*stdin), BufReader::new(*stdout).lines(), BufReader::new(*stderr).lines()))
        }
    }
}
