use std::{
    ops,
    sync::Arc,
    thread::{self, JoinHandle},
};

use bson::{doc, Document};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::Builder;

use mongodb::{action::Aggregate, options::IndexOptions, Client, Collection, Cursor, IndexModel};

use crate::{
    commands::IndexMigrate,
    indicator::{self, Indicator},
    signal::Signal,
};

pub fn run_stats(
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

pub fn index_status(uri: String, db: String, coll: String) -> JoinHandle<()> {
    let handle = thread::spawn(|| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let client = Client::with_uri_str(uri).await.unwrap();

            let db = client.database(&db);

            let index_stats: Result<mongodb::Cursor<bson::Document>, mongodb::error::Error> = db
                .collection::<bson::Document>(&coll)
                .aggregate(vec![doc! {
                    "$indexStats": {}
                }])
                .await;

            match index_stats {
                Ok(cursor) => {
                    // Iterate over the results of the cursor
                    let mut index_stats = cursor;
                    while let Some(Ok(doc)) = index_stats.next().await {
                        let name = doc.get_str("name").unwrap();
                        let accesses = doc.get_document("accesses").unwrap();
                        let ops = accesses.get_i64("ops").unwrap();
                        // let since = accesses.get_datetime("since").unwrap();

                        println!(
                            "Index: name({}) ops({})",
                            name,
                            ops,
                            // since
                            //     .try_to_rfc3339_string()
                            //     .unwrap_or_else(|_| String::new()),
                        );
                    }
                }
                Err(err) => {
                    // Handle the error here
                    panic!("Failed to retrieve index stats: {}", err);
                }
            };

            ()
        });
    });
    handle
}

pub fn index_migrate(index_migrate: IndexMigrate) -> JoinHandle<()> {
    let handle = thread::spawn(|| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let mut source_indexes = std::fs::read(index_migrate.index_path)
                .map(|line| {
                    let index:Value =
                        serde_json::from_str(std::str::from_utf8(&line).unwrap()).expect(
                            "Failed to parse index file, please check the file path or the file content, it should be a json array, run `JSON.stringify(db.xxx.getIndexes())`",
                        );
                    let mut index = index.as_array().expect("Index file should be an array").to_owned();
                    index.retain(
                        |x| x.is_object() && x.as_object().unwrap().contains_key("name"),
                    );
                    index.to_owned()
                })
                .expect(
                    "Failed to read index file, please check the file path or the file content",
                );

            if source_indexes.len() == 0 {
                panic!("Index file is empty, please check the file path or the file content");
            }
            
            let ns = source_indexes.first().unwrap().get("ns").unwrap().as_str().unwrap().to_string();
            let ns: Vec<&str> = ns.split('.').collect();
            let db = ns.first().unwrap();
            let coll = ns.last().unwrap();

            let target = Client::with_uri_str(&index_migrate.uri)
                .await
                .unwrap()
                .database(db);

            let target_coll: Collection<Document> = target.collection(coll);

            let mut target_indexes: Cursor<IndexModel> = target_coll.list_indexes().await.unwrap();

            // println!("{:?}", source_indexes);

            let mut drop_indexes:Vec<String> = vec![];

            while let Some(Ok(index)) = target_indexes.next().await {
                let index: IndexModel = index;
                let name = index.options.as_ref().unwrap().name.as_ref().unwrap();
                drop_indexes.push(name.clone());
                // println!("{:?}", name);
            }

            for index in source_indexes.iter_mut() {
                let index = index.as_object_mut().unwrap();
                let keys = index.remove("key").unwrap();
                let name = index.get("name").unwrap().as_str().unwrap();
                
                if !drop_indexes.contains(&name.to_string()) {
                    println!("IndexMigrate [{}.{}] Create index: {}", db, coll, name);

                    let create_index: Result<IndexOptions, serde_json::Error> = serde::Deserialize::deserialize(Value::Object(index.clone()));                    
                    let mut create_index = create_index.unwrap();
                    create_index.background = Some(true);
                    let index_model = IndexModel::builder().keys(Document::deserialize(keys).unwrap()).build();
                    if let Err (err) = target_coll.create_index(index_model).await {
                        if err.to_string().contains("already exists") {
                            println!("IndexMigrate [{}.{}] Create index: {} already exists", db, coll, name);
                        } else {
                            println!("IndexMigrate [{}.{}] Create index: {} failed, error: {}", db, coll, name, err);
                        }
                    }
                }
                drop_indexes.retain(|x| x != name);
            }
            for index in drop_indexes {
                println!("IndexMigrate [{}.{}] Drop   index: {}", db, coll, index);
                target_coll.drop_index(index).await.unwrap();
            }

        });
    });
    handle
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
