/*
    An include file defining a wrapper type for
    the VGA text console (0xb8000)
*/

// Wrapper type that assumes internal mutability,
// and exposes volatile memory operations
use crate::shared::structs::VolatileCell;

// Standard library imports
use core::slice::from_raw_parts;

/*
    Constants that are assumed to be the de-facto default
    (assuming VGA mode 3, which is 80x25 colored text mode)
*/

// Default memory address for the text buffer
pub const DEF_BUF_ADDR: usize = 0xb8000;

// Default number of columns
pub const DEF_NUM_COLS: usize = 80;

// Default number of rows
pub const DEF_NUM_ROWS: usize = 25;

// Default attribute word (dark white on black)
pub const DEF_ATTR: u16 = 0x0700;

// Maximum page count
pub const MAX_PAGE: usize = 1;

// Space character
pub const CHR_SPACE: u16 = 0x0020;

// VGA console wrapper type
// - should be contained within a lock
// with interior mutability
//
// TODO: use `Write'-like traits as
// soon as they are implemetned
pub struct VgaConsole {
    addr: usize,
    cols: usize,
    rows: usize,
    page: usize,
    x: usize,
    y: usize,
    attr: u16,
    trunc: bool,
}

impl VgaConsole {
    // Create new instance of `VgaConsole'
    pub const fn new(addr: usize, cols: usize, rows: usize) -> Self {
        // Set address and dimensions, and then
        // never touch them again...
        // (currently configured to work like a typewriter)
        VgaConsole {
            addr,
            cols,
            rows,
            page: 0,
            x: 0,
            y: rows - 1,
            attr: DEF_ATTR,
            trunc: true,
        }
    }

    // Create new instance of `VgaConsole' with default values
    pub const fn defaults() -> Self {
        Self::new(DEF_BUF_ADDR, DEF_NUM_COLS, DEF_NUM_ROWS)
    }

    // Internal: get a linear reference to a page in the text buffer
    // SAFETY:
    // We can't possibly guarantee that `page' won't exceed bounds,
    // It is the instantiator's responsibility to make sure that
    // `self.addr' is not equal to zero, and that it points to a
    // valid area in video memory.
    #[inline(always)]
    unsafe fn buf_get_ref(&self, page: usize) -> &[VolatileCell<u16>] {
        let n = self.cols * self.rows;
        let addr = self.addr + 2 * n * page;

        unsafe { from_raw_parts(addr as *const _, n) }
    }

    // Internal: get a reference to a specific
    // line within the current page
    //
    // The provided line number will be forcibly clamped
    // to one less its maximum value, as defined in `rows'
    //
    // SAFETY: (see `buf_get_ref')
    #[inline(always)]
    unsafe fn line_get_ref(&self, page: usize, y: usize) -> &[VolatileCell<u16>] {
        let buf_ref = unsafe { self.buf_get_ref(page) };
        let z = y.min(self.rows - 1);
        let m = self.cols * z;
        let n = self.cols * (z + 1);

        // Return a constant range
        &buf_ref[m..n]
    }

    // Internal: get a reference to a specific
    // character cell within the provided page
    //
    // The provided coordinates will be forcibly clamped to one
    // less their maximum values, as defined in `cols' and `rows'
    //
    // SAFETY: (see `buf_get_ref')
    #[inline(always)]
    unsafe fn char_get_ref(&self, page: usize, x: usize, y: usize) -> &VolatileCell<u16> {
        let line_ref = unsafe { self.line_get_ref(page, y) };
        let u = x.min(self.cols - 1);

        // Return constant reference
        &line_ref[u]
    }

    // Internal: scroll the current page by a specific amount
    // TODO: implement page switching
    // FIXME: optimize me!
    fn scroll_page(&self, n: usize) {
        // Forcibly clamp `n'
        let m = n.min(self.rows - 1);

        // Copy target lines
        for r in 0..(self.rows - m) {
            // Obtain upper and lower lines
            let upper = unsafe { self.line_get_ref(self.page, r) };
            let lower = unsafe { self.line_get_ref(self.page, r + m) };

            for c in 0..self.cols {
                // Transfer cells from the lower line to the upper line
                upper[c].store(lower[c].load());
            }
        }

        // Clear the remaining lines
        for r in (self.rows - m)..self.rows {
            for c in 0..self.cols {
                let blank = self.attr | CHR_SPACE;
                unsafe {
                    self.char_get_ref(self.page, c, r).store(blank);
                }
            }
        }
    }

    // Internal: start a new line, and scroll if necessary
    // TODO: decide if we should settle for LF or CR-LF
    #[inline(always)]
    fn new_line(&mut self) {
        if self.y < self.rows - 1 {
            // Implement CR-LF for now
            // - increment `y' by 1, then clamp
            // - reset `x'
            let z = self.y + 1;
            self.y = z.min(self.rows - 1);
        } else {
            // scroll by one
            self.scroll_page(1);
        }

        // reset `x'
        self.x = 0;
    }

    // Internal: parse character, manipulating
    // console state whenever special characters
    // are encountered
    // TODO:
    // - allow input bytes to control console state
    //   (kind of like VT100-compatible terminals)
    // - implement page switching
    #[inline(always)]
    fn parse_char(&mut self, chr: u8) {
        // - parse special characters
        match chr {
            b'\n' => {
                self.new_line();
                return;
            }
            b'\r' => {
                self.x = 0;
                return;
            }
            _ => {}
        }

        // - apply attributes to character, then
        // write it to current cell
        unsafe {
            let c = self.attr | (chr as u16);
            self.char_get_ref(self.page, self.x, self.y).store(c);
        }

        // - update `x'
        self.x += 1;

        // - reset it and apply newline if
        // it exceeds `cols - 1'
        if self.x >= self.cols {
            self.new_line();
        }
    }

    // Copy bytes from provided buffer to console output
    #[inline(never)]
    pub fn write_bytes(&mut self, buf: &[u8]) -> usize {
        // Obtain the input buffer dimensions, then
        // fit it to the output buffer dimensions
        let n = buf.len();
        let m = n.min(self.cols * self.rows);

        // Choose whether to truncate the buffer
        let b_ref = if self.trunc { &buf[n - m..n] } else { &buf[..] };

        // Obtain the tail of the buffer (so as to
        // avoid unnecessary copying), then write to
        // the console character-by-character
        // FIXME: optimize me!
        for &chr in b_ref {
            self.parse_char(chr);
        }

        b_ref.len()
    }

    // Write string to console output
    // - basically a wrapper around `write_bytes'
    pub fn write_str(&mut self, s: &str) -> usize {
        self.write_bytes(s.as_bytes())
    }

    // Set cursor position
    // TODO: move hardware cursor
    pub fn set_cursor_pos(&mut self, x: usize, y: usize) {
        // Clamp provided coordinates
        self.x = x.min(self.cols - 1);
        self.y = y.min(self.rows - 1);
    }

    // Get truncation mode
    pub fn get_trunc(&self) -> bool {
        self.trunc
    }

    // Set truncation mode
    // - faithful terminal emulation conflicts
    // with time-saving truncation
    pub fn set_trunc(&mut self, t: bool) {
        self.trunc = t;
    }

    // Clear screen, without resetting the cursor
    pub fn clear(&self) {
        // Scroll up until the screen is visually empty
        self.scroll_page(self.rows - 1);
        self.scroll_page(1);
    }
}
