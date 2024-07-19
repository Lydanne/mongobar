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

    /// stress test OPRecord find operation
    OPReplay(OPReplay),

    /// export oplogs to local
    OPExport(OPReplay),

    /// export oplogs to local
    OPImport(OPReplay),

    /// start a tui.
    UI(UI),
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

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,
}

#[derive(clap::Parser, Debug)]
pub struct OPReplay {
    /// eg: qxg
    pub target: String,

    /// 强制重新构建恢复恢复 oplogs
    #[clap(short, long)]
    pub force_build_revert: Option<bool>,
}

#[derive(clap::Parser, Debug)]
pub struct UI {
    /// eg: qxg
    pub target: String,

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,
}
