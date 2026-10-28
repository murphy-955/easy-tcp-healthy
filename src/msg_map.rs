use crate::receive_msg::ReceiveMsg;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

// 组合 key：(uuid, index)，HashMap 的 key 实现了 Hash + Eq 就能用
// LazyLock = 首次访问时才执行初始化（对应 Java 的 static 代码块）
pub(crate) static MSG_MAP: LazyLock<RwLock<HashMap<(String, u32), ReceiveMsg>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn store(uuid: &str, index: u32, msg: ReceiveMsg) {
    // write() 拿写锁，独占；unwrap 是处理"锁中毒"（某个线程 panic 时锁被污染）
    MSG_MAP
        .write()
        .unwrap()
        .insert((uuid.to_string(), index), msg);
}

pub(crate) fn get(uuid: &str, index: u32) -> Option<ReceiveMsg> {
    // read() 拿读锁，多个读并发不互斥 —— 这就是选 RwLock 不选 Mutex 的原因
    MSG_MAP
        .read()
        .unwrap()
        .get(&(uuid.to_string(), index))
        .cloned()
}

pub(crate) fn remove_task(uuid: &str) {
    MSG_MAP
        .write()
        .unwrap()
        .retain(|(task_uuid, _), _| task_uuid != uuid);
}
