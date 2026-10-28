use crate::msg_map;
use crate::receive_msg::ReceiveMsg;
use crate::tasks::job::Job;
use crate::tcp::tcp_connect::tcp_connect;

pub fn deal_task(job: Job) {
    msg_map::start_task(&job.uuid);

    for (offset, msg) in job.messages.iter().enumerate() {
        let index = offset as u32;
        msg_map::store(&job.uuid, index, msg.running());
        let started_at = chrono::Utc::now();
        let result = tcp_connect(msg);
        let result = ReceiveMsg::new(index, msg.clone(), started_at, result);
        msg_map::store(&job.uuid, index, result);
    }

    msg_map::finish_task(&job.uuid);
}
