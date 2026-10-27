use crate::global_queue::GlobalQueue;
use crate::receive_msg::ReceiveMsg;

pub fn submit(msg: Vec<ReceiveMsg>){
    let mut queue = GlobalQueue::new(msg.len());
    for x in msg {
        queue.push_back(x);
    }
}