use crate::receive_msg::ReceiveMsg;

pub struct Job {
    pub uuid: String,
    pub messages: Vec<ReceiveMsg>,
}
