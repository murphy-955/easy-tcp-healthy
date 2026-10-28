use crate::tasks::job::Job;
use crate::tcp::tcp_connect::tcp_connect;

pub fn deal_task(job: Job) {
    for (offset, msg) in job.messages.iter().enumerate() {
        let index = (offset + 1) as u32;
        // 1. 记录 started_at
        let started_at = chrono::Utc::now();
        // 2. 调用 tcp_connect(msg)
        let result = tcp_connect(msg);
        // 3. 根据 Ok / Err 构造结果 ReceiveMsg
        let result = crate::receive_msg::ReceiveMsg::new(index, msg.clone(), started_at, result);
        // 4. crate::msg_map::store(&job.uuid, index, result);
        crate::msg_map::store(&job.uuid, index, result);
    }
}
