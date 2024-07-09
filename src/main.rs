use std::env;

use bson::DateTime;
use clap::Parser;
use commands::{Cli, Commands};
use tokio::runtime::Builder;

mod commands;
mod mongobar;

async fn boot() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.commands {
        Commands::OPRecord(op_record) => {
            let time_range: Vec<_> = op_record
                .time_range
                .split_whitespace()
                .map(|s| DateTime::parse_rfc3339_str(s).unwrap())
                .collect();
            let start = time_range[0];
            let end = time_range[1];
            if op_record.force {
                mongobar::Mongobar::new(&op_record.target)
                    .clean()
                    .op_record((start, end))
                    .await?;
            } else {
                mongobar::Mongobar::new(&op_record.target)
                    .init()
                    .op_record((start, end))
                    .await?;
            }

            println!(
                "OPRecord done output to `./runtime/{}/*`.",
                op_record.target
            );
        }
        Commands::OPStress(op_stress) => {
            mongobar::Mongobar::new(&op_stress.target)
                .init()
                .op_stress()
                .await?;
            println!("OPStress [{}] Done", chrono::Local::now().timestamp());
        }
    }

    Ok(())
}

fn main() {
    if let Ok(RUSTFLAGS) = env::var("RUSTFLAGS") {
        if RUSTFLAGS.contains("tokio_unstable") {
            console_subscriber::init();
        }
    }

    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    runtime.block_on(async {
        let r = boot().await;
        if let Err(err) = r {
            eprintln!("Error: {}", err);
        }
    });
}
