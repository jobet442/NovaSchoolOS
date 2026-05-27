use std::sync::Mutex;
use std::collections::VecDeque;

static SSH_OUTPUT_QUEUE: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static SSH_AUTHENTICATED: Mutex<bool> = Mutex::new(false);

pub fn init_ssh() {
    SSH_OUTPUT_QUEUE.lock().unwrap().clear();
    *SSH_AUTHENTICATED.lock().unwrap() = false;
    let _ = super::tcp::bind_socket(22); // SSH port
}

// Check for SSH connections and execute incoming requests
pub fn poll_ssh_events() {
    if let Some(incoming_data) = super::tcp::receive_data_from_port(22) {
        if let Ok(msg) = std::str::from_utf8(&incoming_data) {
            let cmd = msg.trim();
            
            // Authentication check
            let is_auth = *SSH_AUTHENTICATED.lock().unwrap();
            if !is_auth {
                if cmd == "auth password_teacher123" {
                    *SSH_AUTHENTICATED.lock().unwrap() = true;
                    queue_output("Authentication Successful. Welcome to NovaOS Remote SSH Console.\n");
                } else {
                    queue_output("SSH Connection: Please authenticate using: 'auth <password>'\n");
                }
                return;
            }

            // Authenticated commands
            if cmd == "exit" {
                *SSH_AUTHENTICATED.lock().unwrap() = false;
                queue_output("Connection closed.\n");
            } else if cmd == "teacher_lock" {
                // Trigger terminal lock (the classroom dashboard monitors this state)
                queue_output("LOCK_COMMAND_BROADCASTED\n");
            } else if cmd.starts_with("announcement ") {
                let announcement = &cmd[13..];
                queue_output(&format!("ANNOUNCEMENT_BROADCAST: {}\n", announcement));
            } else {
                queue_output(&format!("SSH: Command execution mock for '{}' -> Success.\n", cmd));
            }
        }
    }
}

fn queue_output(s: &str) {
    let mut queue = SSH_OUTPUT_QUEUE.lock().unwrap();
    if queue.len() < 100 {
        queue.push_back(s.to_string());
    }
}

pub fn read_ssh_output() -> Option<String> {
    SSH_OUTPUT_QUEUE.lock().unwrap().pop_front()
}
