use crate::msg_map::{self, TaskSnapshot};
use crate::rate_limit::RateLimiter;
use crate::receive_msg::{ProbeError, ReceiveMsg};
use crate::tasks::job::Job;
use crate::tasks::submit_task;
use axum::Json;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, State};
use axum::http::header::{LOCATION, RETRY_AFTER};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, TrySendError};
use uuid::Uuid;

const MAX_BODY_SIZE: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct AppState {
    tx: SyncSender<Job>,
    rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub(crate) fn new(tx: SyncSender<Job>) -> Self {
        Self {
            tx,
            rate_limiter: Arc::new(RateLimiter::new()),
        }
    }
}

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/probe-tasks", post(create_task))
        .route("/api/v1/probe-tasks/{task_id}", get(get_task))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTaskRequest {
    timeout_ms: Option<u32>,
    targets: Vec<TargetRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetRequest {
    host: String,
    port: u32,
}

#[derive(Serialize)]
struct CreateTaskResponse {
    task_id: String,
    status: &'static str,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
    request_id: String,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    request_id: String,
    retry_after: Option<u64>,
}

impl ApiError {
    fn new(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        request_id: String,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id,
            retry_after: None,
        }
    }

    fn with_retry_after(mut self, retry_after: u64) -> Self {
        self.retry_after = Some(retry_after);
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code,
                message: self.message,
            },
            request_id: self.request_id.clone(),
        };
        let mut response = (self.status, Json(body)).into_response();
        insert_request_id(&mut response, &self.request_id);
        if let Some(retry_after) = self.retry_after {
            response.headers_mut().insert(
                RETRY_AFTER,
                HeaderValue::from_str(&retry_after.to_string())
                    .expect("retry-after is a valid header"),
            );
        }
        response
    }
}

#[derive(Serialize)]
struct TaskResponse {
    task_id: String,
    status: &'static str,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    total: usize,
    completed: usize,
    targets: Vec<TargetResponse>,
}

#[derive(Serialize)]
struct TargetResponse {
    index: u32,
    host: String,
    port: u16,
    status: String,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    duration_ms: Option<i64>,
    resolved_ip: Option<String>,
    error: Option<ProbeError>,
}

async fn create_task(
    State(state): State<AppState>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    body: Result<Json<CreateTaskRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::now_v7().to_string();
    if let Err(retry_after) = state.rate_limiter.check(client.ip()) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "task submission rate limit exceeded",
            request_id,
        )
        .with_retry_after(retry_after));
    }

    let Json(payload) = body.map_err(|rejection| {
        let code = if matches!(rejection, JsonRejection::JsonSyntaxError(_)) {
            "INVALID_JSON"
        } else {
            "INVALID_REQUEST"
        };
        ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            if code == "INVALID_JSON" {
                "request body is not valid JSON"
            } else {
                "invalid request body or parameters"
            },
            request_id.clone(),
        )
    })?;

    let messages = validate_payload(payload).map_err(|message| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            message,
            request_id.clone(),
        )
    })?;
    let task_id = Uuid::now_v7().to_string();

    match submit_task::submit(&state.tx, task_id.clone(), messages) {
        Ok(()) => {
            let created_at = msg_map::get_task(&task_id)
                .expect("submitted task must exist")
                .record
                .created_at;
            let body = CreateTaskResponse {
                task_id: task_id.clone(),
                status: "queued",
                created_at,
            };
            let mut response = (StatusCode::ACCEPTED, Json(body)).into_response();
            response.headers_mut().insert(
                LOCATION,
                HeaderValue::from_str(&format!("/api/v1/probe-tasks/{task_id}"))
                    .expect("task location is a valid header"),
            );
            insert_request_id(&mut response, &request_id);
            Ok(response)
        }
        Err(TrySendError::Full(_)) => Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "QUEUE_FULL",
            "background task queue is full",
            request_id,
        )),
        Err(TrySendError::Disconnected(_)) => Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "failed to submit background task",
            request_id,
        )),
    }
}

async fn get_task(Path(task_id): Path<String>) -> Result<Response, ApiError> {
    let request_id = Uuid::now_v7().to_string();
    let snapshot = msg_map::get_task(&task_id).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "TASK_NOT_FOUND",
            "probe task not found",
            request_id.clone(),
        )
    })?;

    let response = task_response(task_id, snapshot);
    let mut response = (StatusCode::OK, Json(response)).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

fn task_response(task_id: String, snapshot: TaskSnapshot) -> TaskResponse {
    let TaskSnapshot { record, targets } = snapshot;
    let status = if record.started_at.is_none() {
        "queued"
    } else if record.finished_at.is_some() {
        "completed"
    } else {
        "running"
    };
    let completed = targets
        .iter()
        .filter(|target| matches!(target.status.as_str(), "success" | "failed"))
        .count();
    let targets = targets
        .into_iter()
        .map(|target| TargetResponse {
            index: target.index,
            host: target.host,
            port: target.port,
            status: target.status,
            started_at: target.started_at,
            finished_at: target.finished_at,
            duration_ms: target.duration.map(|duration| duration.num_milliseconds()),
            resolved_ip: target.resolved_ip,
            error: target.error,
        })
        .collect();

    TaskResponse {
        task_id,
        status,
        created_at: record.created_at,
        started_at: record.started_at,
        finished_at: record.finished_at,
        total: record.total,
        completed,
        targets,
    }
}

fn validate_payload(payload: CreateTaskRequest) -> Result<Vec<ReceiveMsg>, String> {
    let timeout_ms = payload.timeout_ms.unwrap_or(3000);
    if !(100..=10000).contains(&timeout_ms) {
        return Err(String::from("timeout_ms must be between 100 and 10000"));
    }
    if !(1..=100).contains(&payload.targets.len()) {
        return Err(String::from("targets must contain between 1 and 100 items"));
    }

    payload
        .targets
        .into_iter()
        .enumerate()
        .map(|(index, target)| {
            validate_host(&target.host)
                .map_err(|message| format!("targets[{index}].host {message}"))?;
            let port = u16::try_from(target.port)
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| format!("targets[{index}].port must be between 1 and 65535"))?;
            Ok(ReceiveMsg::pending(
                index as u32,
                target.host,
                port,
                timeout_ms,
            ))
        })
        .collect()
}

fn validate_host(host: &str) -> Result<(), &'static str> {
    if host.is_empty() {
        return Err("must not be empty");
    }
    if host.chars().count() > 253 {
        return Err("must contain at most 253 characters");
    }
    if host.chars().any(char::is_whitespace)
        || host.contains("://")
        || host.contains(['/', '?', '#'])
    {
        return Err("must be a host name or IP address without a protocol or path");
    }
    if host.parse::<IpAddr>().is_err() && host.contains(':') {
        return Err("must not contain a port or bracketed IPv6 address");
    }
    Ok(())
}

fn insert_request_id(response: &mut Response, request_id: &str) {
    response.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(request_id).expect("UUID is a valid request ID header"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;
    use std::sync::mpsc;

    fn request() -> CreateTaskRequest {
        CreateTaskRequest {
            timeout_ms: Some(3000),
            targets: vec![TargetRequest {
                host: String::from("127.0.0.1"),
                port: 8080,
            }],
        }
    }

    fn client() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345)))
    }

    #[test]
    fn validation_rejects_invalid_parameters() {
        let mut empty_targets = request();
        empty_targets.targets.clear();
        assert!(validate_payload(empty_targets).is_err());

        let mut invalid_port = request();
        invalid_port.targets[0].port = 0;
        assert!(validate_payload(invalid_port).is_err());

        let mut invalid_timeout = request();
        invalid_timeout.timeout_ms = Some(50);
        assert!(validate_payload(invalid_timeout).is_err());

        let mut invalid_host = request();
        invalid_host.targets[0].host = String::from("http://127.0.0.1");
        assert!(validate_payload(invalid_host).is_err());
    }

    #[tokio::test]
    async fn creates_and_queries_a_queued_task() {
        let (tx, rx) = mpsc::sync_channel(1);
        let state = AppState::new(tx);

        let response = create_task(State(state), client(), Ok(Json(request())))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(response.headers().contains_key(LOCATION));
        assert!(response.headers().contains_key("x-request-id"));

        let job = rx.try_recv().unwrap();
        let task_id = job.uuid.clone();
        let response = get_task(Path(task_id.clone())).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["status"], "queued");
        assert_eq!(body["total"], 1);
        assert_eq!(body["completed"], 0);
        assert_eq!(body["targets"][0]["index"], 0);
        assert_eq!(body["targets"][0]["status"], "pending");

        msg_map::remove_task(&task_id);
    }

    #[tokio::test]
    async fn returns_queue_full_and_task_not_found() {
        let (tx, rx) = mpsc::sync_channel(1);
        let state = AppState::new(tx);
        let first = create_task(State(state.clone()), client(), Ok(Json(request())))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::ACCEPTED);

        let error = create_task(State(state), client(), Ok(Json(request())))
            .await
            .unwrap_err()
            .into_response();
        assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);

        let error = get_task(Path(String::from("missing-task")))
            .await
            .unwrap_err()
            .into_response();
        assert_eq!(error.status(), StatusCode::NOT_FOUND);

        let task_id = rx.try_recv().unwrap().uuid;
        msg_map::remove_task(&task_id);
    }
}
