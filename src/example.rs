use tokio::process::{Command, Child, ChildStdin, ChildStdout, ChildStderr};
use std::process::Stdio;
use tokio::io::{
    AsyncBufReadExt,
    AsyncWriteExt,
    BufReader,
    BufWriter,
    Lines,
    Result as IOResult,
    Error as IOError,
    ErrorKind as IOErrorKind,
};
use futures_util::future::{select, Either};
use futures_util::StreamExt;

type RunnerStdin = BufWriter<ChildStdin>;
type RunnerStdout = Lines<BufReader<ChildStdout>>;
type RunnerStderr = Lines<BufReader<ChildStderr>>;

fn build_command() -> IOResult<Command> {
    let mut cmd = Command::new("python");
    cmd
        .arg("-u")
        .arg("-c")
        .arg("print(input())")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .env_clear()
        .kill_on_drop(true);

    Ok(cmd)
}

fn make_runner() -> IOResult<(Child, RunnerStdin, RunnerStdout, RunnerStderr)> {
    let mut command = build_command()?;
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

async fn actual_main() -> IOResult<()> {
    let (mut child, mut stdin, mut stdout, _) = make_runner()?;
    // I had this as part of a bigger future-powered loop, rewriting is why
    // I took so long sorry
    stdin.write(b"pingpong\n").await?;
    let mut line_fut = stdout.next();
    loop {
        match select(child, line_fut).await {
            Either::Left((child_code, line_continue)) => {
                println!("exit code: {:?}", child_code);
                break;
            },
            Either::Right((line_read, child_continue)) => {
                println!("{:?}", line_read);
                child = child_continue;
                line_fut = stdout.next();
            }
        }
    }
    println!("done!");
    Ok(())
}

#[tokio::main]
async fn main() {
    match actual_main().await {
        Ok(()) => (),
        Err(why) => { println!("Error: {}", why); }
    }
}

