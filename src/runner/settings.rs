use tokio::process::Command;
use std::process::Stdio;
use std::ffi::OsStr;
use std::fs::canonicalize;

// TODO: read this in from a toml file/command line arg or something
pub const OTHELLO_ROOT : &str = "../othello_tourney/";

pub fn build_unjailed_command<S: AsRef<OsStr>>(ai_name: S) -> Result<Command, tokio::io::Error> {
    let canonical_root = canonicalize(OTHELLO_ROOT)?;
    let mut run_file = canonical_root.clone();
    run_file.push("run_ai_jailed.py");

    let mut cmd = Command::new("python");
    cmd
        .arg("-u")
        .arg(run_file)
        .arg(ai_name)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .current_dir(canonical_root)
        .env_clear()
        .kill_on_drop(true);

    Ok(cmd)
}

pub fn build_jailed_command<S: AsRef<OsStr>>(ai_name: S) -> Result<Command, tokio::io::Error> {
    // TODO: actually implement this
    build_unjailed_command(ai_name)
}
