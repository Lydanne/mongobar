use std::env;

use bson::DateTime;
use clap::Parser;
use commands::{Cli, Commands};
use futures::Future;
use indicator::print_indicator;
use tokio::runtime::Builder;

mod commands;
mod indicator;
mod mongobar;
mod signal;
mod ui;

pub fn ind_keys() -> Vec<String> {
    vec![
        "boot_worker".to_string(),
        "query_count".to_string(),
        "cost_ms".to_string(),
        "progress".to_string(),
        "logs".to_string(),
        "progress_total".to_string(),
        "thread_count".to_string(),
        "done_worker".to_string(),
        "query_qps".to_string(),
        "querying".to_string(),
        "dyn_threads".to_string(),
        "dyn_cc_limit".to_string(),
    ]
}

fn boot() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.commands {
        Commands::OPRecord(op_record) => {
            exec_tokio(move || async move {
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

                Ok(())
            });
        }
        Commands::OPStress(op_stress) => {
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys());
                print_indicator(&indic);
                let m = mongobar::Mongobar::new(&op_stress.target)
                    .set_indicator(indic)
                    .merge_config_uri(op_stress.uri)
                    .merge_config_loop_count(op_stress.loop_count)
                    .init();
                println!("OPStress [{}] Start.", chrono::Local::now().timestamp());
                m.op_stress(op_stress.filter).await?;
                println!("OPStress [{}] Done", chrono::Local::now().timestamp());

                Ok(())
            });
        }
        Commands::OPReplay(op_replay) => {
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys());
                print_indicator(&indic);
                let m = mongobar::Mongobar::new(&op_replay.target)
                    .set_indicator(indic)
                    .merge_config_rebuild(op_replay.rebuild)
                    .merge_config_uri(op_replay.uri)
                    .init();
                println!("OPReplay [{}] Start.", chrono::Local::now().timestamp());
                m.op_replay().await?;
                println!("OPReplay [{}] Done", chrono::Local::now().timestamp());

                Ok(())
            });
        }
        Commands::UI(ui) => {
            let _ = ui::boot(ui);
        }
        Commands::OPExport(args) => exec_tokio(move || async move {
            mongobar::Mongobar::new(&args.target)
                .init()
                .op_export()
                .await?;

            println!(
                "OPExport done output to `./runtime/{}/data.op`.",
                args.target
            );

            Ok(())
        }),
        Commands::OPImport(args) => {
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys());
                print_indicator(&indic);
                mongobar::Mongobar::new(&args.target)
                    .merge_config_uri(Some(args.uri))
                    .set_indicator(indic)
                    .init()
                    .op_import()
                    .await?;

                println!("OPImport done by `./runtime/{}/data.op`.", args.target);
                Ok(())
            });
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

    boot().unwrap();
}

pub fn exec_tokio<F, Fut>(cb: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'static,
{
    let runtime = Builder::new_multi_thread()
        .worker_threads(
            num_cpus::get()
                .checked_sub(1)
                .unwrap_or(num_cpus::get_physical())
                * 4,
        )
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let r = cb().await;
        if let Err(err) = r {
            eprintln!("Error: {}", err);
        }
    });
}
