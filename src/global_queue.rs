use crate::receive_msg::ReceiveMsg;
use crate::tcp::tcp_connect::tcp_connect;
use chrono::Utc;
use std::collections::VecDeque;

pub struct GlobalQueue<T> {
    queue: VecDeque<T>,
    max_size: usize,
}

impl GlobalQueue<ReceiveMsg> {
    pub fn deal_task(&self) {
        let mut index = 0;
        for x in &self.queue {
            index += 1;
            let started_at = Utc::now();
            match tcp_connect(&x) {
                Ok(addr) => {
                    let msg = ReceiveMsg::new(
                        index,
                        x.host.clone(),
                        x.port,
                        String::from("OK"),
                        started_at,
                        Utc::now(),
                        Some(Utc::now() - started_at),
                        Vec::from([addr]),
                        None,
                        x.time_out,
                    );
                    println!("{}", index);
                }
                Err(reason) => {
                    let msg1 = ReceiveMsg::new(
                        index,
                        x.host.clone(),
                        x.port,
                        String::from("ERR"),
                        started_at,
                        Utc::now(),
                        Some(Utc::now() - started_at),
                        Vec::from([reason.split_once("-").unwrap().0.to_string()]),
                        Some(reason.split_once("-").unwrap().1.to_string()),
                        x.time_out,
                    );
                    println!("{}", index);
                }
            }
        }
    }
}

impl<T> GlobalQueue<T> {
    pub fn new(max_size: usize) -> Self {
        GlobalQueue {
            queue: VecDeque::new(),
            max_size,
        }
    }

    pub fn push_back(&mut self, item: T) {
        if self.queue.len() >= self.max_size {
            self.queue.pop_front();
        }
        self.queue.push_back(item);
    }
}
