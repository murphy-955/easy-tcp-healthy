use crate::receive_msg::ReceiveMsg;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

#[derive(Clone)]
pub(crate) struct TaskRecord {
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
    pub(crate) total: usize,
}

pub(crate) struct TaskSnapshot {
    pub(crate) record: TaskRecord,
    pub(crate) targets: Vec<ReceiveMsg>,
}

static MSG_MAP: LazyLock<RwLock<HashMap<(String, u32), ReceiveMsg>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static TASK_MAP: LazyLock<RwLock<HashMap<String, TaskRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn create_task(uuid: &str, messages: &[ReceiveMsg]) {
    {
        let mut msg_map = MSG_MAP.write().unwrap();
        for msg in messages {
            msg_map.insert((uuid.to_string(), msg.index), msg.clone());
        }
    }

    TASK_MAP.write().unwrap().insert(
        uuid.to_string(),
        TaskRecord {
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            total: messages.len(),
        },
    );
}

pub(crate) fn store(uuid: &str, index: u32, msg: ReceiveMsg) {
    MSG_MAP
        .write()
        .unwrap()
        .insert((uuid.to_string(), index), msg);
}

pub(crate) fn get_task(uuid: &str) -> Option<TaskSnapshot> {
    let record = TASK_MAP.read().unwrap().get(uuid).cloned()?;
    let msg_map = MSG_MAP.read().unwrap();
    let targets = (0..record.total as u32)
        .map(|index| msg_map.get(&(uuid.to_string(), index)).cloned())
        .collect::<Option<Vec<_>>>()?;

    Some(TaskSnapshot { record, targets })
}

pub(crate) fn start_task(uuid: &str) {
    if let Some(task) = TASK_MAP.write().unwrap().get_mut(uuid)
        && task.started_at.is_none()
    {
        task.started_at = Some(Utc::now());
    }
}

pub(crate) fn finish_task(uuid: &str) {
    if let Some(task) = TASK_MAP.write().unwrap().get_mut(uuid) {
        task.finished_at = Some(Utc::now());
    }
}

pub(crate) fn remove_task(uuid: &str) {
    MSG_MAP
        .write()
        .unwrap()
        .retain(|(task_uuid, _), _| task_uuid != uuid);
    TASK_MAP.write().unwrap().remove(uuid);
}

pub(crate) fn cleanup_expired() {
    let now = Utc::now();
    let expired: Vec<String> = TASK_MAP
        .read()
        .unwrap()
        .iter()
        .filter(|(_, task)| {
            task.finished_at
                .is_some_and(|finished_at| now - finished_at > Duration::hours(1))
        })
        .map(|(uuid, _)| uuid.clone())
        .collect();

    for uuid in expired {
        remove_task(&uuid);
    }
}
