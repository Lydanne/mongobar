use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

use bson::{doc, Document};
use tokio::runtime::Builder;

use mongodb::Client;

use crate::{
    indicator::{self, Indicator},
    signal::Signal,
};

pub fn mongo_stats(
    uri: String,
    db: String,
    signal: Arc<Signal>,
) -> (JoinHandle<()>, indicator::Indicator) {
    let indicator = Indicator::new().init(
        vec!["write_queue".to_string(), "read_queue".to_string()],
        "./t.log".to_string(),
    );
    let write_queue = indicator.take("write_queue").unwrap();
    let read_queue = indicator.take("read_queue").unwrap();
    let handle = thread::spawn(|| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let client = Client::with_uri_str(uri).await.unwrap();

            loop {
                if signal.get() != 0 {
                    return;
                }
                let db = client.database(&db);

                let server_status: Document =
                    db.run_command(doc! {  "serverStatus": 1 }).await.unwrap();

                let read_queue_value = server_status
                    .get_document("globalLock")
                    .unwrap()
                    .get_document("currentQueue")
                    .unwrap()
                    .get_i32("readers")
                    .unwrap();
                let write_queue_value = server_status
                    .get_document("globalLock")
                    .unwrap()
                    .get_document("currentQueue")
                    .unwrap()
                    .get_i32("writers")
                    .unwrap();

                read_queue.set(read_queue_value as usize);
                write_queue.set(write_queue_value as usize);

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    });
    (handle, indicator)
}

pub fn print_indicator(indicator: &Indicator) {
    let read_queue = indicator.take("read_queue").unwrap();
    let write_queue = indicator.take("write_queue").unwrap();

    thread::spawn({
        let read_queue = read_queue.clone();
        let write_queue = write_queue.clone();
        move || loop {
            let read_queue = read_queue.get();
            let write_queue = write_queue.get();

            println!(
                "Stats [{}] rw_queue: r({}) w({})",
                chrono::Local::now().timestamp(),
                read_queue,
                write_queue
            );

            thread::sleep(tokio::time::Duration::from_secs(1));
        }
    });
}
