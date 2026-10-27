pub struct ReceiveMsg{
    pub(crate) index: i32,
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

impl ReceiveMsg{
    pub fn new(index: i32, host: Vec<String>, port: u16, status: String, started_at: chrono::DateTime<chrono::Utc>, finished_at: chrono::DateTime<chrono::Utc>, duration: Option<chrono::Duration>, resolved_ip : Vec<String>, error: Option<String>, time_out: u32) -> Self {
        ReceiveMsg {
            index,
            host,
            port,
            status,
            started_at,
            finished_at,
            duration,
            resolved_ip,
            error,
            time_out,
        }
    }
}