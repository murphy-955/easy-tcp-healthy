#[derive(Clone, Debug)]
pub struct ReceiveMsg{
    pub(crate) index: u32,
    pub(crate) host: Vec<String>,
    pub(crate) port: u16,
    pub(crate) status: String,
    pub(crate) started_at: chrono::DateTime<chrono::Utc>,
    pub(crate) finished_at: chrono::DateTime<chrono::Utc>,
    pub(crate) duration: Option<chrono::Duration>,
    pub(crate) resolved_ip : Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) time_out: u32,
}

impl ReceiveMsg {
    pub fn new(
        index: u32,
        msg: ReceiveMsg,
        started_at: chrono::DateTime<chrono::Utc>,
        result: Result<String, String>,
    ) -> Self {
        let finished_at = chrono::Utc::now();
        let (status, resolved_ip, error) = match result {
            Ok(addr) => (String::from("OK"), vec![addr], None),
            Err(reason) => (String::from("ERR"), Vec::new(), Some(reason)),
        };

        ReceiveMsg {
            index,
            host: msg.host,
            port: msg.port,
            status,
            started_at,
            finished_at,
            duration: Some(finished_at - started_at),
            resolved_ip,
            error,
            time_out: msg.time_out,
        }
    }
}