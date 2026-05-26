use crate::mutex::Mutex;

// Standard VGA text mode dimensions
pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;
pub const VGA_BUFFER_SIZE: usize = VGA_WIDTH * VGA_HEIGHT * 2;

// VGA color attributes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: VgaColor, background: VgaColor) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// Global simulated VGA memory buffer
pub static VGA_BUFFER: Mutex<[u8; VGA_BUFFER_SIZE]> = Mutex::new([0; VGA_BUFFER_SIZE]);
pub static VGA_CURSOR: Mutex<(usize, usize)> = Mutex::new((0, 0));

pub struct VgaWriter {
    color_code: ColorCode,
}

impl VgaWriter {
    pub fn new(foreground: VgaColor, background: VgaColor) -> Self {
        VgaWriter {
            color_code: ColorCode::new(foreground, background),
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let mut cursor = VGA_CURSOR.lock();
                let (_row, col) = *cursor;

                if col >= VGA_WIDTH {
                    self.new_line_locked(&mut cursor);
                }

                let (row, col) = *cursor;
                let offset = (row * VGA_WIDTH + col) * 2;
                let mut buffer = VGA_BUFFER.lock();
                buffer[offset] = byte;
                buffer[offset + 1] = self.color_code.0;

                *cursor = (row, col + 1);
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        let mut cursor = VGA_CURSOR.lock();
        self.new_line_locked(&mut cursor);
    }

    fn new_line_locked(&mut self, cursor: &mut (usize, usize)) {
        let (row, _col) = *cursor;
        if row < VGA_HEIGHT - 1 {
            *cursor = (row + 1, 0);
        } else {
            // Scroll the buffer up by one row
            let mut buffer = VGA_BUFFER.lock();
            let mut temp = [0u8; VGA_WIDTH * 2];
            for r in 1..VGA_HEIGHT {
                let dest_offset = (r - 1) * VGA_WIDTH * 2;
                let src_offset = r * VGA_WIDTH * 2;
                temp.copy_from_slice(&buffer[src_offset..(src_offset + VGA_WIDTH * 2)]);
                buffer[dest_offset..(dest_offset + VGA_WIDTH * 2)].copy_from_slice(&temp);
            }
            // Clear the bottom row
            let clear_offset = (VGA_HEIGHT - 1) * VGA_WIDTH * 2;
            let blank = b' ';
            for i in 0..VGA_WIDTH {
                buffer[clear_offset + i * 2] = blank;
                buffer[clear_offset + i * 2 + 1] = self.color_code.0;
            }
            *cursor = (VGA_HEIGHT - 1, 0);
        }
    }

    pub fn clear_screen(&mut self) {
        let mut buffer = VGA_BUFFER.lock();
        let mut cursor = VGA_CURSOR.lock();
        let blank = b' ';
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            buffer[i * 2] = blank;
            buffer[i * 2 + 1] = self.color_code.0;
        }
        *cursor = (0, 0);
    }
}

impl core::fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// Global thread-safe writer reference for printf/println macros
pub static VGA_WRITER: Mutex<Option<VgaWriter>> = Mutex::new(None);

pub fn init_vga() {
    let mut writer = VgaWriter::new(VgaColor::White, VgaColor::Black);
    writer.clear_screen();
    *VGA_WRITER.lock() = Some(writer);
}

// A macro similar to println! for printing to the simulated VGA display
#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => {
        if let Some(ref mut writer) = *$crate::vga::drv_vga_writer() {
            use core::fmt::Write;
            let _ = write!(writer, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! vga_println {
    () => ($crate::vga_print!("\n"));
    ($($arg:tt)*) => {
        if let Some(ref mut writer) = *$crate::vga::drv_vga_writer() {
            use core::fmt::Write;
            let _ = write!(writer, $($arg)*);
            let _ = write!(writer, "\n");
        }
    };
}

// Helper to access writer from macro
pub fn drv_vga_writer() -> crate::mutex::MutexGuard<'static, Option<VgaWriter>> {
    VGA_WRITER.lock()
}
