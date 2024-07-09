use clap::Parser;

#[derive(Parser)]
#[clap(
    name = "mongobar",
    version = "0.0.1",
    author = "@xgj/wmc",
    about = "mongo bar cli tool"
)]
pub struct Cli {
    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(Parser)]
pub enum Commands {
    /// record mongo profile by time range
    OPRecord(OPRecord),

    /// stress test OPRecord find operation
    OPStress(OPStress),
}

#[derive(clap::Parser, Debug)]
pub struct OPRecord {
    /// eg: qxg
    pub target: String,

    /// eg: "2024-07-08T00:00:00.837Z 2024-07-09T00:00:00.838Z"
    #[clap(short, long)]
    pub time_range: String,

    /// force to clean
    #[clap(short, long)]
    pub force: bool,
}

#[derive(clap::Parser, Debug)]
pub struct OPStress {
    /// eg: qxg
    pub target: String,
}
