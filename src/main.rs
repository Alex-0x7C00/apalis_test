use apalis::{layers::{limit::ConcurrencyLimitLayer, retry::RetryPolicy, WorkerBuilderExt}, prelude::{Monitor, WorkerBuilder, WorkerFactoryFn}};
use apalis_sql::{sqlite::SqliteStorage, sqlx::SqlitePool};
use serde::Serialize;
use serde::Deserialize;
use tokio::signal;
use apalis::prelude::Storage;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CallbackTaskInfo {
    resource: String,
    user_id: i64,
}

async fn callback_task(job: CallbackTaskInfo) -> bool {
    println!("Callback task: {:?}", job);
    true
}

#[tokio::main]
async fn main() {

    let sql_pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    //let sql_config = apalis_sql::Config::default().buffer_size(1);
    SqliteStorage::setup(&sql_pool)
        .await
        .expect("unable to run migrations for sqlite");
    let mut apalis_storage: SqliteStorage<CallbackTaskInfo> = SqliteStorage::new(sql_pool);

    let callback_task_worker = 
    WorkerBuilder::new("callback")
        .retry(RetryPolicy::retries(3))
        .layer(ConcurrencyLimitLayer::new(1))
        .backend(apalis_storage.clone())
        .build_fn(callback_task);
    tokio::spawn(async move {
        let mut i = 0;
        loop {
            let job = CallbackTaskInfo {
                resource: "https://example.com".to_string(),
                user_id: i,
            };
            apalis_storage.push(job.clone()).await.unwrap();
            i += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    });
    let apalis_start = Monitor::new()
        .register(callback_task_worker)
    //.register(orderlist_worker)
        .run_with_signal(signal::ctrl_c()).await;
}