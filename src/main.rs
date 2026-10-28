use crate::tasks::deal_task::deal_task;
use crate::tasks::job::Job;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

mod global_queue;
mod msg_map;
mod receive_msg;
mod tasks;
mod tcp;

fn main() {
    const MAX_QUEUE_SIZE: usize = 2000;
    const WORKER_COUNT: usize = 10;

    let (_tx, rx) = mpsc::sync_channel::<Job>(MAX_QUEUE_SIZE);
    let rx = Arc::new(Mutex::new(rx));
    let mut workers = Vec::with_capacity(WORKER_COUNT);

    for _ in 0..WORKER_COUNT {
        let rx = Arc::clone(&rx);
        let worker = thread::spawn(move || {
            loop {
                // 只在取任务时持有锁，取到后立即释放，保证 10 个线程可以并发执行任务。
                let job = rx.lock().unwrap().recv();
                match job {
                    Ok(job) => deal_task(job),
                    Err(_) => break,
                }
            }
        });
        workers.push(worker);
    }

    for worker in workers {
        worker.join().unwrap();
    }
}
