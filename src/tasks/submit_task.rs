use crate::global_queue::GlobalQueue;
use crate::submit_msg::SubmitMsg;

pub fn submit(msg: Vec<SubmitMsg>){
    let mut queue = GlobalQueue::new(msg.len());
    for x in msg {
        queue.push_back(x);
    }
    queue.deal_task();
}