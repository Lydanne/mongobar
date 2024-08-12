use clap::Parser;

#[derive(Parser)]
#[clap(
    name = "mongobar",
    version = "0.1.2",
    author = "@xgj/wmc",
    about = "mongo bar cli tool"
)]
pub struct Cli {
    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(Parser)]
pub enum Commands {
    /// 录制 mongo profile 并生成测试的 oplogs
    OPRecord(OPRecord),

    /// 从 mongo 中拉取 oplogs
    OPPull(OPPull),

    /// 压力测试，对数据进行无序的压力测试
    OPStress(OPStress),

    /// 压力回放，对数据库进行压力的回放测试，有一定的顺序执行
    OPReplay(OPReplay),

    /// 压力回放前进行状态重置
    OPRevert(OPReplay),

    /// 压测完成后恢复命令
    OPResume(OPReplay),

    /// 构建压测恢复的 oplogs
    OPBuildResume(OPReplay),

    /// 导出相关的数据到 op 文件内
    OPExport(OPReplay),

    /// 将上门的导出的 op 文件导出到数据指定的数据库
    OPImport(OPImport),

    /// 另存为压力测试的 op 文件
    SaveAs(SaveAs),

    /// 查看 mongo 数据库状态（读写队列）
    Stats(Stats),

    /// 查看索引被使用状态
    IndexStatus(Stats),

    /// 一些高效的辅助命令，包括文件的的行正则筛选、分析阿里云的审计日志、转换阿里云的审计日志为压测 oplogs.op等
    #[clap(subcommand)]
    Tool(Tool),

    /// 启动 UI 进行压力测试
    UI(UI),
}

#[derive(clap::Parser, Debug, Clone)]
pub enum Tool {
    /// 分析阿里云的审计日志
    Ana(Analyzer),

    /// 转换阿里云的审计日志为压测 oplogs.op
    Cov(Convert),

    /// 通过正则过滤文件的行
    Filter(Filter),
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
pub struct Stats {
    /// 目标数据库连接
    pub uri: Option<String>,

    /// 目标数据库 db
    pub db: Option<String>,

    /// 目标数据库集合
    #[clap(short, long)]
    pub coll: Option<String>,
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

    /// 插入的时候去除某个字段
    #[clap(short, long)]
    pub ignore_field: Vec<String>,
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
pub struct SaveAs {
    /// eg: qxg
    pub target: String,

    /// 保存到某个目录
    pub outdir: String,

    /// 如果文件存在强制保存
    #[clap(short, long)]
    pub force: bool,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct UI {
    /// eg: qxg
    pub target: String,

    /// regex filter oplog
    #[clap(long)]
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

    /// 插入的时候去除某个字段
    #[clap(short, long)]
    pub ignore_field: Vec<String>,
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

#[derive(clap::Parser, Debug, Clone)]

pub struct Filter {
    /// 目标文件或者是 oplogs 的目录名称
    pub target: String,

    /// 匹配字符串，默认是通过正则匹配，如果传入 -m 则是通过模式匹配
    #[clap(long)]
    pub filter: String,

    /// 开启模式匹配
    #[clap(short, long)]
    pub mode: bool,
}
