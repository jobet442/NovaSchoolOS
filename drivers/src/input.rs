use crate::mutex::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Escape,
    Control(char),
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

// A simple array-based circular queue for no_std compatibility without heap allocation
struct KeyQueue {
    buffer: [KeyEvent; 256],
    head: usize,
    tail: usize,
    count: usize,
}

impl KeyQueue {
    const fn new() -> Self {
        KeyQueue {
            buffer: [KeyEvent { code: KeyCode::None, shift: false, ctrl: false, alt: false }; 256],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, event: KeyEvent) {
        if self.count < 256 {
            self.buffer[self.tail] = event;
            self.tail = (self.tail + 1) % 256;
            self.count += 1;
        }
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.count > 0 {
            let event = self.buffer[self.head];
            self.head = (self.head + 1) % 256;
            self.count -= 1;
            Some(event)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

// Keyboard input buffer queue
static KEYBOARD_QUEUE: Mutex<KeyQueue> = Mutex::new(KeyQueue::new());

// Mouse coordinate state
static MOUSE_STATE: Mutex<(i32, i32, bool)> = Mutex::new((0, 0, false)); // (x, y, left_clicked)

pub fn init_input() {
    KEYBOARD_QUEUE.lock().clear();
    *MOUSE_STATE.lock() = (0, 0, false);
}

// Push a key event into the system queue
pub fn push_key_event(event: KeyEvent) {
    KEYBOARD_QUEUE.lock().push(event);
}

// Read a key event from the buffer (non-blocking)
pub fn read_key_event() -> Option<KeyEvent> {
    KEYBOARD_QUEUE.lock().pop()
}

// Flush keyboard queue
pub fn flush_keyboard() {
    KEYBOARD_QUEUE.lock().clear();
}

// Update simulated mouse state
pub fn update_mouse(x: i32, y: i32, clicked: bool) {
    *MOUSE_STATE.lock() = (x, y, clicked);
}

// Read simulated mouse state
pub fn read_mouse() -> (i32, i32, bool) {
    *MOUSE_STATE.lock()
}
