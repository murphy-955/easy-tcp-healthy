use crate::tasks::deal_task::deal_task;
use crate::tasks::job::Job;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

mod api;
mod msg_map;
mod rate_limit;
mod receive_msg;
mod tasks;
mod tcp;

const MAX_QUEUE_SIZE: usize = 2000;
const WORKER_COUNT: usize = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::sync_channel::<Job>(MAX_QUEUE_SIZE);
    let rx = Arc::new(Mutex::new(rx));

    for _ in 0..WORKER_COUNT {
        let rx = Arc::clone(&rx);
        thread::spawn(move || {
            loop {
                // 只在取任务时持有锁，取到后立即释放，保证 worker 可以并发执行任务。
                let job = rx.lock().unwrap().recv();
                match job {
                    Ok(job) => deal_task(job),
                    Err(_) => break,
                }
            }
        });
    }

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            msg_map::cleanup_expired();
        }
    });

    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("HTTP server listening on http://{address}");

    let app = api::router(api::AppState::new(tx));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
