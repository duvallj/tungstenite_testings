use serde::{Serialize, Deserialize};

use crate::othello::*;

pub type RunnerStdin = ChildStdin;
pub type RunnerStdout = Lines<BufReader<ChildStdout>>;
pub type RunnerStderr = Lines<BufReader<ChildStderr>>;

pub struct Runner {
    child: Child,
    stdin: RunnerStdin,
    stdout: RunnerStdout,
    stderr: RunnerStderr,
}

pub enum PlayerType {
    Human,
    Ai {name: String, runner: Runner},
}

pub struct Game {
    black: PlayerType,
    white: PlayerType,
    board: BoardStruct,
}

// It'll be much easier to slightly change the Python deserialization 
// than it will be to change this serialization
#[derive(Debug, Serialize)]
pub struct GetMove {
    board: BoardStruct,
    player: Player,
    ai_name: String,
    timelimit: f32,
};
#[derive(Deserialize)]
#[serde(untagged)]
pub type ReceiveMove = usize;
