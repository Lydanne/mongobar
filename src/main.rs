use std::{env, path::PathBuf};

use bson::DateTime;
use clap::{builder::Str, Parser};
use commands::{Cli, Commands};
use convert::convert_alilog_csv;
use futures::Future;
use indicator::print_indicator;
use mongobar::Mongobar;
use tokio::runtime::Builder;

mod analyze;
mod commands;
mod convert;
mod indicator;
mod mongobar;
mod signal;
mod ui;
mod utils;

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
        Commands::OPRecord(args) => {
            exec_tokio(move || async move {
                if args.force {
                    mongobar::Mongobar::new(&args.target)
                        .clean()
                        .op_record()
                        .await?;
                } else {
                    mongobar::Mongobar::new(&args.target)
                        .init()
                        .op_record()
                        .await?;
                }

                Ok(())
            });
        }
        Commands::OPPull(args) => {
            exec_tokio(move || async move {
                let time_range: Vec<_> = args
                    .time_range
                    .split_whitespace()
                    .map(|s| DateTime::parse_rfc3339_str(s).unwrap())
                    .collect();
                let start = time_range[0];
                let end = time_range[1];
                if args.force {
                    mongobar::Mongobar::new(&args.target)
                        .clean()
                        .op_pull((start, end))
                        .await?;
                } else {
                    mongobar::Mongobar::new(&args.target)
                        .init()
                        .op_pull((start, end))
                        .await?;
                }

                println!("OPRecord done output to `./mongobar/{}/*`.", args.target);

                Ok(())
            });
        }
        Commands::OPStress(mut op_stress) => {
            target_parse(&mut op_stress.target, op_stress.update);
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys(), op_stress.target.clone());
                print_indicator(&indic);
                let m = mongobar::Mongobar::new(&op_stress.target)
                    .set_indicator(indic)
                    .merge_config_uri(op_stress.uri)
                    .merge_config_loop_count(op_stress.loop_count)
                    .merge_config_thread_count(op_stress.thread_count)
                    .init();
                println!("OPStress [{}] Start.", chrono::Local::now().timestamp());
                m.op_stress(op_stress.filter, op_stress.readonly).await?;
                println!("OPStress [{}] Done", chrono::Local::now().timestamp());

                Ok(())
            });
        }
        Commands::OPReplay(mut op_replay) => {
            target_parse(&mut op_replay.target, op_replay.update);
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys(), op_replay.target.clone());
                print_indicator(&indic);
                let m = mongobar::Mongobar::new(&op_replay.target)
                    .set_indicator(indic)
                    .merge_config_rebuild(op_replay.rebuild)
                    .merge_config_uri(op_replay.uri)
                    .merge_config_thread_count(op_replay.thread_count)
                    .init();
                println!("OPReplay [{}] Start.", chrono::Local::now().timestamp());
                m.op_replay().await?;
                println!("OPReplay [{}] Done", chrono::Local::now().timestamp());

                Ok(())
            });
        }
        Commands::OPRevert(mut args) => {
            target_parse(&mut args.target, args.update);
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys(), args.target.clone());
                print_indicator(&indic);
                let m = mongobar::Mongobar::new(&args.target)
                    .set_indicator(indic)
                    .merge_config_rebuild(args.rebuild)
                    .merge_config_uri(args.uri)
                    .init();
                println!("OPReplay [{}] Start.", chrono::Local::now().timestamp());
                m.op_run_revert().await?;
                println!("OPReplay [{}] Done", chrono::Local::now().timestamp());

                Ok(())
            });
        }
        Commands::UI(mut ui) => {
            target_parse(&mut ui.target, ui.update);
            let _ = ui::boot(ui);
        }
        Commands::OPExport(args) => exec_tokio(move || async move {
            mongobar::Mongobar::new(&args.target)
                .init()
                .op_export()
                .await?;

            println!(
                "OPExport done output to `./mongobar/{}/data.op`.",
                args.target
            );

            Ok(())
        }),
        Commands::OPImport(args) => {
            exec_tokio(move || async move {
                let indic = indicator::Indicator::new().init(ind_keys(), args.target.clone());
                print_indicator(&indic);
                mongobar::Mongobar::new(&args.target)
                    .merge_config_uri(Some(args.uri))
                    .set_indicator(indic)
                    .init()
                    .op_import()
                    .await?;

                println!("OPImport done by `./mongobar/{}/data.op`.", args.target);
                Ok(())
            });
        }
        Commands::Ana(args) => {
            analyze::analysis_alilog_csv(&args.target).unwrap();
        }
        Commands::Cov(args) => {
            convert::convert_alilog_csv(&args.target, args.filter_db.unwrap_or_default()).unwrap();
        }
    }

    Ok(())
}

fn target_parse(target: &mut String, update: Option<bool>) {
    let path = PathBuf::from(target.clone());
    if path.exists() {
        let ext = path.extension().unwrap();
        match ext.to_str().unwrap() {
            "op" => {
                let name = path.file_stem().unwrap().to_str().unwrap();
                *target = name.to_string();
                // 复制文件到 .mongobar/{name}/oplogs.op
                let m = Mongobar::new(name);
                if m.exists() {
                    if update.unwrap_or_default() {
                        m.clean();
                        let _ =
                            std::fs::copy(path.clone(), format!("./.mongobar/{}/oplogs.op", name));
                    }
                } else {
                    m.init();
                    let _ = std::fs::copy(path.clone(), format!("./.mongobar/{}/oplogs.op", name));
                }
            }
            "csv" => {
                let name = path.file_stem().unwrap().to_str().unwrap();
                *target = name.to_string();
                let m = Mongobar::new(name);

                // 复制文件到 .mongobar/{name}/oplogs.csv
                if m.exists() {
                    if update.unwrap_or_default() {
                        let oplogs_path =
                            convert_alilog_csv(path.to_str().unwrap(), m.config.db.clone()).expect(
                                "convert_alilog_csv failed, please check the csv file format.",
                            );
                        m.clean();
                        let _ =
                            std::fs::rename(oplogs_path, format!("./.mongobar/{}/oplogs.op", name));
                    }
                } else {
                    let oplogs_path =
                        convert_alilog_csv(path.to_str().unwrap(), m.config.db.clone())
                            .expect("convert_alilog_csv failed, please check the csv file format.");
                    m.init();
                    let _ = std::fs::rename(oplogs_path, format!("./.mongobar/{}/oplogs.op", name));
                }
            }
            _ => {
                println!("Invalid file type: {:?}", ext);
            }
        }
    }
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
                * 2
                + 1,
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
