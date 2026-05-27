use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MessageQueue {
    pub name: String,
    pub messages: Vec<String>,
}

static IPC_QUEUES: Mutex<Option<HashMap<String, MessageQueue>>> = Mutex::new(None);

pub fn init_ipc() {
    *IPC_QUEUES.lock().unwrap() = Some(HashMap::new());
}

pub fn create_queue(name: &str) -> Result<(), String> {
    let mut queues_lock = IPC_QUEUES.lock().unwrap();
    let queues = queues_lock.as_mut().ok_or("IPC uninitialized")?;
    if queues.contains_key(name) {
        return Err("Queue already exists".to_string());
    }
    queues.insert(name.to_string(), MessageQueue {
        name: name.to_string(),
        messages: Vec::new(),
    });
    Ok(())
}

pub fn send_message(name: &str, msg: String) -> Result<(), String> {
    let mut queues_lock = IPC_QUEUES.lock().unwrap();
    let queues = queues_lock.as_mut().ok_or("IPC uninitialized")?;
    let q = queues.get_mut(name).ok_or("Queue not found")?;
    if q.messages.len() >= 50 {
        q.messages.remove(0); // Keep size bounded
    }
    q.messages.push(msg);
    Ok(())
}

pub fn recv_message(name: &str) -> Result<Option<String>, String> {
    let mut queues_lock = IPC_QUEUES.lock().unwrap();
    let queues = queues_lock.as_mut().ok_or("IPC uninitialized")?;
    let q = queues.get_mut(name).ok_or("Queue not found")?;
    if q.messages.is_empty() {
        Ok(None)
    } else {
        Ok(Some(q.messages.remove(0)))
    }
}

pub fn get_queues() -> Vec<(String, usize, Vec<String>)> {
    let queues_lock = IPC_QUEUES.lock().unwrap();
    if let Some(ref queues) = *queues_lock {
        queues.values()
            .map(|q| (q.name.clone(), q.messages.len(), q.messages.clone()))
            .collect()
    } else {
        Vec::new()
    }
}
