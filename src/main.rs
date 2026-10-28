use std::sync::mpsc;
use std::thread;
use crate::tasks::deal_task::deal_task;
use crate::tasks::job::Job;

mod tasks;
mod global_queue;
mod receive_msg;
mod tcp;
mod msg_map;

fn main() {
    let (tx, rx) = mpsc::channel::<Job>();
    let worker = thread::spawn(move || {
        // recv 没有任务时会阻塞等待，不会空转消耗 CPU
        while let Ok(job) = rx.recv() {
            deal_task(job);
        }
    });
}
