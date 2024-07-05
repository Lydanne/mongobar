use tokio::runtime::Builder;

mod mongobar;

fn main() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(1000) // 设置工作线程数
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        // Mongobar::new("qxg")
        //     .clean()
        //     .op_record((
        //         DateTime::parse_rfc3339_str("2024-07-03T10:54:18.837Z").unwrap(),
        //         DateTime::parse_rfc3339_str("2024-07-05T10:54:18.838Z").unwrap(),
        //     ))
        //     .await?;
        if let Err(err) = mongobar::Mongobar::new("qxg").init().op_stress().await {
            // Handle the error here
            eprintln!("Error: {}", err);
        }
    });
}
