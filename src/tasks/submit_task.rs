use std::sync::mpsc::{SyncSender, TrySendError};

use crate::msg_map;
use crate::receive_msg::ReceiveMsg;
use crate::tasks::job::Job;

pub fn submit(
    tx: &SyncSender<Job>,
    uuid: String,
    messages: Vec<ReceiveMsg>,
) -> Result<(), TrySendError<Job>> {
    // 请求进入后，先在 HashMap 中记录任务和所有待处理目标。
    msg_map::create_task(&uuid, &messages);

    let job = Job { uuid, messages };
    match tx.try_send(job) {
        Ok(()) => Ok(()),
        Err(error) => {
            // 入队失败的任务不会被执行，清理已经写入的记录。
            let failed_job = match &error {
                TrySendError::Full(job) | TrySendError::Disconnected(job) => job,
            };
            msg_map::remove_task(&failed_job.uuid);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn message() -> ReceiveMsg {
        ReceiveMsg::pending(0, String::from("127.0.0.1"), 8080, 3000)
    }

    #[test]
    fn submit_stores_pending_messages_before_queueing() {
        let (tx, rx) = mpsc::sync_channel(1);
        let uuid = String::from("submit-pending-test");

        submit(&tx, uuid.clone(), vec![message()]).unwrap();

        let task = msg_map::get_task(&uuid).unwrap();
        assert_eq!(task.targets[0].status, "pending");
        assert_eq!(rx.try_recv().unwrap().uuid, uuid);
        msg_map::remove_task(&uuid);
    }

    #[test]
    fn submit_returns_full_and_cleans_up_when_queue_is_full() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let first_uuid = String::from("submit-full-first-test");
        let second_uuid = String::from("submit-full-second-test");
        submit(&tx, first_uuid.clone(), vec![message()]).unwrap();

        let error = submit(&tx, second_uuid.clone(), vec![message()]).unwrap_err();

        assert!(matches!(error, TrySendError::Full(_)));
        assert!(msg_map::get_task(&first_uuid).is_some());
        assert!(msg_map::get_task(&second_uuid).is_none());
        msg_map::remove_task(&first_uuid);
    }
}
