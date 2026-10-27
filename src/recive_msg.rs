pub struct ReceiveMsg{
    index: i32,
    host: Vec<String>,
    port: u16,
    status: String,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    duration: Option<chrono::Duration>,
    resolved_ip : Option<Vec<String>>,
    error: Option<String>,
}