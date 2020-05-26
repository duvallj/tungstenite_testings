use tokio::process::{
    Child,
    ChildStdin,
    ChildStdout,
    ChildStderr,
};
use tokio::io::{
    BufReader,
    Lines,
};

pub type RunnerStdin = ChildStdin;
pub type RunnerStdout = Lines<BufReader<ChildStdout>>;
pub type RunnerStderr = BufReader<ChildStderr>;

pub struct Runner {
    pub child: Child,
    pub stdin: RunnerStdin,
    pub stdout: RunnerStdout,
    pub stderr: RunnerStderr,
    pub ai_name: String,
}
