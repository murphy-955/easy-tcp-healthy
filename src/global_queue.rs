use crate::receive_msg::ReceiveMsg;
use crate::tcp::tcp_connect::tcp_connect;
use chrono::Utc;
use std::collections::VecDeque;
use uuid::Uuid;

pub struct GlobalQueue<T> {
    queue: VecDeque<T>,
    max_size: usize,
}

impl GlobalQueue<ReceiveMsg> {
    pub fn deal_task(&self, uuid: &str) {
        let mut index = 0;
        for x in &self.queue {
            index += 1;
            let started_at = Utc::now();
            let result = tcp_connect(x);
            let msg = ReceiveMsg::new(index, x.clone(), started_at, result);
            crate::msg_map::store(uuid, index, msg);
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
