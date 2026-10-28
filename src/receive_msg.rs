use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ProbeError {
    pub code: String,
    pub message: String,
}

impl ProbeError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReceiveMsg {
    pub(crate) index: u32,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) status: String,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
    pub(crate) duration: Option<chrono::Duration>,
    pub(crate) resolved_ip: Option<String>,
    pub(crate) error: Option<ProbeError>,
    pub(crate) time_out: u32,
}

impl ReceiveMsg {
    pub fn pending(index: u32, host: String, port: u16, time_out: u32) -> Self {
        Self {
            index,
            host,
            port,
            status: String::from("pending"),
            started_at: None,
            finished_at: None,
            duration: None,
            resolved_ip: None,
            error: None,
            time_out,
        }
    }

    pub fn running(&self) -> Self {
        let mut msg = self.clone();
        msg.status = String::from("running");
        msg.started_at = Some(Utc::now());
        msg
    }

    pub fn new(
        index: u32,
        msg: ReceiveMsg,
        started_at: DateTime<Utc>,
        result: Result<String, ProbeError>,
    ) -> Self {
        let finished_at = Utc::now();
        let (status, resolved_ip, error) = match result {
            Ok(addr) => (String::from("success"), Some(addr), None),
            Err(reason) => (String::from("failed"), None, Some(reason)),
        };

        Self {
            index,
            host: msg.host,
            port: msg.port,
            status,
            started_at: Some(started_at),
            finished_at: Some(finished_at),
            duration: Some(finished_at - started_at),
            resolved_ip,
            error,
            time_out: msg.time_out,
        }
    }
}
