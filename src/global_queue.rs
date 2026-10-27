use crate::recive_msg::ReceiveMsg;
use crate::submit_msg::SubmitMsg;
use crate::tcp::tcp_connect;
use std::collections::VecDeque;

pub struct GlobalQueue<T> {
    queue: VecDeque<T>,
    max_size: usize,
}

impl<T: SubmitMsg> GlobalQueue<T> {
    pub(crate) fn deal_task(&self) {
        let mut index = 0;
        for x in &self.queue {
            index += 1;
            let started_at = std::time::Instant::now();
            match tcp_connect(&x) {
                Ok(addr) => ReceiveMsg {
                    index: index,
                    host: &x.host,
                    port: &x.port,
                    status: String::new("OK"),
                    started_at: started_at,
                    finished_at: std::time::Instant::now(),
                    error: None,
                },
                Err(reason) => println!("失败原因: {}", reason),
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
