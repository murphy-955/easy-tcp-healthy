use std::sync::mpsc::{self, Sender};

use crate::receive_msg::ReceiveMsg;
use crate::tasks::job::Job;

pub fn submit(
    tx: &Sender<Job>,
    uuid: String,
    messages: Vec<ReceiveMsg>,
) -> Result<(), mpsc::SendError<Job>> {
    let job = Job { uuid, messages };
    tx.send(job)
}