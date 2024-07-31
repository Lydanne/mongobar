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

    /// record mongo profile by time range
    OPPull(OPPull),

    /// stress test OPRecord find operation
    OPStress(OPStress),

    /// stress test OPRecord find operation
    OPReplay(OPReplay),

    /// stress test OPRecord find operation
    OPRevert(OPReplay),

    /// export oplogs to local
    OPExport(OPReplay),

    /// export oplogs to local
    OPImport(OPImport),

    /// 分析阿里云的审计日志
    Ana(Analyzer),

    /// 转换阿里云的审计日志为压测 oplogs.op
    Cov(Convert),

    /// start a tui.
    UI(UI),
}

#[derive(clap::Parser, Debug, Clone)]
pub struct OPRecord {
    /// eg: qxg
    pub target: String,

    /// force to clean
    #[clap(short, long)]
    pub force: bool,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct OPPull {
    /// eg: qxg
    pub target: String,

    /// eg: "2024-07-08T00:00:00.837Z 2024-07-09T00:00:00.838Z"
    #[clap(short, long)]
    pub time_range: String,

    /// force to clean
    #[clap(short, long)]
    pub force: bool,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct OPStress {
    /// eg: qxg
    pub target: String,

    /// 是否强制更新
    #[clap(long)]
    pub update: Option<bool>,

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,

    ///覆盖配置的 uri
    #[clap(short, long)]
    pub uri: Option<String>,

    /// 循环次数
    #[clap(short, long)]
    pub loop_count: Option<usize>,

    /// 线程数量
    #[clap(short, long)]
    pub thread_count: Option<usize>,

    /// 只运行读操作
    #[clap(long)]
    pub readonly: bool,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct OPReplay {
    /// eg: qxg
    pub target: String,

    /// 是否强制更新
    #[clap(long)]
    pub update: Option<bool>,

    /// 强制重新构建恢复恢复 oplogs
    #[clap(short, long)]
    pub rebuild: Option<bool>,

    ///覆盖配置的 uri
    #[clap(short, long)]
    pub uri: Option<String>,

    /// 线程数量
    #[clap(short, long)]
    pub thread_count: Option<usize>,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct OPImport {
    /// eg: qxg
    pub target: String,

    /// mongo uri
    pub uri: String,

    /// 强制重新构建恢复恢复 oplogs
    #[clap(short, long)]
    pub rebuild: Option<bool>,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct UI {
    /// eg: qxg
    pub target: String,

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,

    /// 强制重新构建恢复恢复 oplogs
    #[clap(short, long)]
    pub rebuild: Option<bool>,

    /// 是否强制更新
    #[clap(long)]
    pub update: Option<bool>,

    ///覆盖配置的 uri
    #[clap(short, long)]
    pub uri: Option<String>,

    /// 循环次数
    #[clap(short, long)]
    pub loop_count: Option<usize>,

    /// 线程数量
    #[clap(short, long)]
    pub thread_count: Option<usize>,

    /// 只运行读操作
    #[clap(long)]
    pub readonly: bool,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct Analyzer {
    pub target: String,

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,

    /// 强制重新构建恢复恢复 oplogs
    #[clap(short, long)]
    pub rebuild: Option<bool>,

    ///覆盖配置的 uri
    #[clap(short, long)]
    pub uri: Option<String>,

    /// 循环次数
    #[clap(short, long)]
    pub loop_count: Option<usize>,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct Convert {
    pub target: String,

    /// regex filter oplog
    #[clap(short, long)]
    pub filter: Option<String>,

    /// filter db
    #[clap(long)]
    pub filter_db: Option<String>,

    /// 是否重新构建
    #[clap(short, long)]
    pub rebuild: Option<bool>,
}
