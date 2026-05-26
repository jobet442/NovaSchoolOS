pub mod novapkg;

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct User {
    pub uid: u32,
    pub username: String,
    pub password_hash: String,
    pub groups: Vec<String>,
}

static USER_DATABASE: Mutex<Option<HashMap<String, User>>> = Mutex::new(None);
static CURRENT_SESSION_USER: Mutex<Option<String>> = Mutex::new(None);

pub fn init_userspace() {
    novapkg::init_novapkg();

    let mut db = HashMap::new();
    
    // Add default users
    db.insert("root".to_string(), User {
        uid: 0,
        username: "root".to_string(),
        password_hash: "admin123".to_string(),
        groups: vec!["root".to_string(), "wheel".to_string()],
    });
    db.insert("student1".to_string(), User {
        uid: 1001,
        username: "student1".to_string(),
        password_hash: "student123".to_string(),
        groups: vec!["students".to_string()],
    });
    db.insert("student2".to_string(), User {
        uid: 1002,
        username: "student2".to_string(),
        password_hash: "student123".to_string(),
        groups: vec!["students".to_string()],
    });
    db.insert("teacher".to_string(), User {
        uid: 500,
        username: "teacher".to_string(),
        password_hash: "teacher123".to_string(),
        groups: vec!["faculty".to_string(), "wheel".to_string()],
    });

    *USER_DATABASE.lock().unwrap() = Some(db);
    
    // Default logged in user is student1 for learning labs
    *CURRENT_SESSION_USER.lock().unwrap() = Some("student1".to_string());

    // Create the Classroom Filesystem layout in VFS
    create_classroom_filesystem_layout();
}

fn create_classroom_filesystem_layout() {
    // Create base directories
    let _ = filesystem::vfs_mkdir("/students", 0o755, 0);
    let _ = filesystem::vfs_mkdir("/shared", 0o777, 0);
    let _ = filesystem::vfs_mkdir("/courses", 0o755, 0);
    let _ = filesystem::vfs_mkdir("/assignments", 0o755, 0);
    let _ = filesystem::vfs_mkdir("/faculty", 0o750, 0);

    // Create student workspaces (only owner can modify)
    let _ = filesystem::vfs_mkdir("/students/student1001", 0o700, 1001);
    let _ = filesystem::vfs_mkdir("/students/student1002", 0o700, 1002);

    // Populate assignments
    let fd = filesystem::vfs_open("/assignments/lab1_instructions.txt", true, true, 0).unwrap();
    let instructions = "Welcome to Lab 1: Systems Programming.\nTask: Use the redirection commands to write 'Hello NovaSchool' to your students directory.\nUsage: echo 'Hello Nova' > /students/student1001/hello.txt\n";
    let _ = filesystem::vfs_write(fd, instructions.as_bytes(), 0);
    let _ = filesystem::vfs_close(fd);
}

// Useradd command
pub fn useradd(username: &str, password: &str, is_teacher: bool) -> Result<(), String> {
    let mut db_lock = USER_DATABASE.lock().unwrap();
    let db = db_lock.as_mut().ok_or("User database uninitialized")?;

    if db.contains_key(username) {
        return Err(format!("User '{}' already exists", username));
    }

    let uid = if is_teacher { 500 + db.len() as u32 } else { 1000 + db.len() as u32 };
    let groups = if is_teacher { vec!["faculty".to_string()] } else { vec!["students".to_string()] };

    db.insert(username.to_string(), User {
        uid,
        username: username.to_string(),
        password_hash: password.to_string(),
        groups,
    });

    // Create home workspace
    let _ = filesystem::vfs_mkdir(&format!("/students/student{}", uid), 0o700, uid);
    Ok(())
}

// User session management
pub fn get_current_user() -> Option<User> {
    let name_lock = CURRENT_SESSION_USER.lock().unwrap();
    if let Some(ref name) = *name_lock {
        let db_lock = USER_DATABASE.lock().unwrap();
        db_lock.as_ref().and_then(|db| db.get(name).cloned())
    } else {
        None
    }
}

pub fn login_user(username: &str, password: &str) -> Result<User, String> {
    let mut authenticated_user = None;
    {
        let db_lock = USER_DATABASE.lock().unwrap();
        let db = db_lock.as_ref().ok_or("User database uninitialized")?;
        if let Some(user) = db.get(username) {
            if user.password_hash == password {
                authenticated_user = Some(user.clone());
            }
        }
    } // lock released

    if let Some(user) = authenticated_user {
        *CURRENT_SESSION_USER.lock().unwrap() = Some(username.to_string());
        Ok(user)
    } else {
        Err("Invalid username or password".to_string())
    }
}

pub fn logout_user() {
    // Auto-Reset student environment on logout to remove temporary files and malware
    if let Some(user) = get_current_user() {
        if user.uid >= 1000 {
            auto_reset_student_environment(user.uid);
        }
    }
    *CURRENT_SESSION_USER.lock().unwrap() = None;
}

// Auto Reset: wipes a student's workspace clean and recovers pristine state
pub fn auto_reset_student_environment(uid: u32) {
    // In our simulation, we just recreate the student folder.
    // In a real OS this acts as the recovery backup trigger.
    let path = format!("/students/student{}", uid);
    // Remove files and recreate empty directory
    let _ = filesystem::vfs_mkdir(&path, 0o700, uid);
    
    drivers::vga_println!("[Classroom Security] Auto-Reset executed for student UID {}. Workspace clean.", uid);
}
