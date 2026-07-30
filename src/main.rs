mod delete;
mod download;
mod judge;
mod lark;
mod validate;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "feishu-wiki", about = "飞书知识库管理工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 下载知识库节点（递归），按飞书原始目录结构保存
    Download(download::Args),
    /// 审阅文档并记录整理判断
    Judge(judge::Args),
    /// 根据 manifest 删除飞书源文档
    Delete(delete::Args),
    /// 根据契约验证已下载的知识库
    Validate(validate::Args),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Download(args) => download::run(args),
        Command::Judge(args) => judge::run(args),
        Command::Delete(args) => delete::run(args),
        Command::Validate(args) => validate::run(args),
    }
}
