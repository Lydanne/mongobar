use std::env;

use bson::DateTime;
use clap::Parser;
use commands::{Cli, Commands};
use indicator::print_indicator;
use tokio::runtime::Builder;

mod commands;
mod indicator;
mod mongobar;
mod signal;
mod ui;

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
            let indic = indicator::Indicator::new().init(vec![
                "boot_worker".to_string(),
                "query_count".to_string(),
                "cost_ms".to_string(),
                "progress".to_string(),
                "logs".to_string(),
                "progress_total".to_string(),
                "thread_count".to_string(),
            ]);
            print_indicator(&indic);
            mongobar::Mongobar::new(&op_stress.target)
                .set_indicator(indic)
                .init()
                .op_stress()
                .await?;
            println!("OPStress [{}] Done", chrono::Local::now().timestamp());
        }
        Commands::UI(ui) => {
            let _ = ui::boot(&ui.target);
        }
    }

    Ok(())
}

fn main() {
    if let Ok(rustflags) = env::var("RUSTFLAGS") {
        if rustflags.contains("tokio_unstable") {
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
