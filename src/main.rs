use bson::DateTime;
use tokio::runtime::Builder;

mod mongobar;

async fn boot() -> Result<(), Box<dyn std::error::Error>> {
    // mongobar::Mongobar::new("qxg")
    //     .clean()
    //     .op_record((
    //         DateTime::parse_rfc3339_str("2024-07-08T00:00:00.837Z").unwrap(),
    //         DateTime::parse_rfc3339_str("2024-07-09T00:00:00.838Z").unwrap(),
    //     ))
    //     .await?;

    mongobar::Mongobar::new("qxg").init().op_stress().await?;
    Ok(())
}

fn main() {
    let runtime = Builder::new_multi_thread()
        // .worker_threads(1000) // 设置工作线程数
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let r = boot().await;
        if let Err(err) = r {
            eprintln!("Error: {}", err);
        }
    });
}
