/*!
    Module defining a wrapper type for the VGA text console
*/

// Wrapper type that assumes internal mutability,
// and exposes volatile memory operations
use crate::shared::structs::volatile::VolatileCell;

// Fundamental data structures
use crate::shared::structs::{Array, RingBuf};

// I/O helpers
use crate::shared::io::{Error, Write};

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

// Maximum shadow buffer column count (worst-case)
pub const MAX_SHADOW_COLS: usize = 160;

// Maximum shadow buffer row count (worst-case)
pub const MAX_SHADOW_ROWS: usize = 50;

// VGA console wrapper type
// - should be contained within a lock
// with interior mutability
//
// TODO: refine shadow buffering
// TODO: implement page switching
pub struct VgaConsole {
    addr: usize,
    cols: usize,
    rows: usize,
    page: usize,
    x: usize,
    y: usize,
    attr: u16,
    trunc: bool,
    shadow: Option<RingBuf<Array<u16, MAX_SHADOW_COLS>, MAX_SHADOW_ROWS>>,
    buffered: bool,
}

impl VgaConsole {
    // Create new instance of `VgaConsole'
    // - we trust that the provided dimensions are sane
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
            shadow: None,
            buffered: false,
        }
    }

    // Create new instance of `VgaConsole' with default values
    pub const fn defaults() -> Self {
        Self::new(DEF_BUF_ADDR, DEF_NUM_COLS, DEF_NUM_ROWS)
    }

    // Initialize the shadow buffer
    // - not strictly necessary for normal use, but
    // performance may degrade significantly
    pub fn init(&mut self) {
        // Only allow buffering if dimensions are
        // within "worst-case" bounds
        if self.cols <= MAX_SHADOW_COLS && self.rows <= MAX_SHADOW_ROWS {
            self.shadow = Some(RingBuf::new());
            self.buffered = true;
        }
    }

    // Check whether the console is shadow-buffered
    pub fn is_shadowed(&self) -> bool {
        // - use state of `self.shadow` in case
        // `self.buffered` is set for some reason
        self.buffered && self.shadow.is_some()
    }

    // Attempt to enable shadow buffering
    // - returns `Ok(bool)` on success, and `Err(())` on failure
    // TODO: synchronize contents with the text buffer
    pub fn try_set_shadowed(&mut self) -> Result<bool, ()> {
        // Only return a positive result if the
        // shadow buffer is initialized
        if self.shadow.is_none() {
            Err(())
        } else {
            // - return previous flag value
            let r = Ok(self.buffered);

            // - set flag, then return generated value
            self.buffered = true;
            r
        }
    }

    // Disable shadow buffering
    // (without throwing away the buffer, of course)
    pub fn unset_shadowed(&mut self) {
        self.buffered = false;
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
    // - not strictly required by `Write`
    pub fn clear(&mut self) -> Result<(), Error> {
        // Scroll up until the screen is visually empty
        self.scroll_page(self.rows - 1);
        self.scroll_page(1);

        // Commit changes to the text buffer (if needed)
        self.commit();

        Ok(())
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
    // - this is basically a ring buffer advance
    // FIXME: optimize me!
    #[inline(always)]
    fn scroll_page(&mut self, n: usize) {
        // Forcibly clamp `n'
        let m = n.min(self.rows - 1);

        // Perform buffered scrolling whenever possible
        if self.is_shadowed() {
            self.scroll_shadow(m);
        } else {
            self.scroll_in_place(m);
        }
    }

    // Internal: scroll the shadow buffer
    #[inline(always)]
    fn scroll_shadow(&mut self, m: usize) {
        // Trust that the shadow buffer is initialized
        let shadow = self.shadow.as_mut().unwrap();

        // Advance the shadow buffer by `m` lines
        // - this is zero-copy, as advancing the
        // buffer is the same as "scrolling up"
        shadow.rol(m);

        // Clear the bottom `m` lines
        for r in 0..m {
            for c in 0..self.cols {
                shadow[self.rows - m + r][c] = self.attr | CHR_SPACE;
            }
        }
    }

    // Internal: perform in-place scrolling of the text buffer
    #[inline(always)]
    fn scroll_in_place(&mut self, m: usize) {
        // Perform manual `memmove` on the text buffer
        for r in 0..(self.rows - m) {
            let dest = unsafe { self.line_get_ref(self.page, r) };
            let src = unsafe { self.line_get_ref(self.page, r + m) };

            for c in 0..self.cols {
                dest[c].store(src[c].load());
            }
        }

        // Clear the bottom `m` lines
        for r in 0..m {
            let line = unsafe { self.line_get_ref(self.page, self.rows - m + r) };

            for c in 0..self.cols {
                line[c].store(self.attr | CHR_SPACE);
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

    // Internal: write character to the current page,
    // manipulating console state whenever special
    // characters are encountered
    // TODO:
    // - allow input bytes to control console state
    //   (kind of like VT100-compatible terminals)
    // - implement page switching
    #[inline(always)]
    fn write_char(&mut self, chr: u8) {
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
        let x = self.x;
        let y = self.y;
        let c = self.attr | (chr as u16);

        if self.is_shadowed() {
            // - trust that the shadow buffer is initialized
            let shadow = self.shadow.as_mut().unwrap();

            // - perform shadowed write
            shadow[y][x] = c;
        } else {
            // - perform in-place write
            unsafe {
                self.char_get_ref(self.page, x, y).store(c);
            }
        }

        // - update `x'
        self.x += 1;

        // - reset it and apply newline if
        // it exceeds `cols - 1'
        if self.x >= self.cols {
            self.new_line();
        }
    }

    // Internal: copy shadow buffer contents to the text buffer
    // TODO: implement page switching and vectorization
    #[inline(always)]
    fn commit(&mut self) {
        // - do not perform flush if shadow-buffering
        // is either unavailable or disabled
        if self.is_shadowed() {
            // - trust that the buffer is initialized
            let shadow = self.shadow.as_ref().unwrap();

            // - copy rows by iterating over them, then
            // copying each cell (column-indexed)
            for r in 0..self.rows {
                // - calculate references to lines, so as to save cycles
                let buf_line = unsafe { self.line_get_ref(self.page, r) };
                let shadow_line = shadow[r];

                for c in 0..self.cols {
                    // - perform volatile write to the text buffer
                    buf_line[c].store(shadow_line[c]);
                }
            }
        }
    }
}

impl Write for VgaConsole {
    // Copy bytes from provided buffer to console
    // output WITHOUT committing changes
    // - this operation should NOT fail under any circumstance
    // TODO: a `BufWrite` or two should be considered internally
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        // Obtain the input buffer dimensions, then
        // fit it to the output buffer dimensions
        let n = buf.len();
        let m = n.min(self.cols * self.rows);

        // Choose whether to truncate the buffer
        let b_ref = if self.trunc { &buf[n - m..n] } else { &buf[..] };

        // Obtain the tail of the buffer (so as to
        // avoid unnecessary copying), then write to
        // the shadow buffe  character-by-character
        // FIXME: optimize me!
        for &chr in b_ref {
            self.write_char(chr);
        }

        Ok(b_ref.len())
    }

    // Flush buffer
    // TODO: define semantics
    fn flush(&mut self) -> Result<(), Error> {
        // Commit any changes to the text buffer
        self.commit();

        Ok(())
    }
}
