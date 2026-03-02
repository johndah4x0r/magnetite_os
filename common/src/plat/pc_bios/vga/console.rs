/*!
    Module defining a wrapper type for the VGA text console
*/

// Wrapper type that assumes internal mutability,
// and exposes volatile memory operations
use crate::shared::structs::volatile::VolatileCell;

// Fundamental data structures
use crate::shared::structs::RingBuf;

// I/O helpers
use crate::shared::io::{Error, Write};

// Standard library imports
use core::marker::PhantomData;
use core::slice::from_raw_parts;

/*
    Constants that are assumed to be the de-facto default
    (assuming VGA mode 3, which is 80x25 colored text mode)
*/

/// Default memory address for the VGA text buffer
pub const DEF_BUF_ADDR: usize = 0xb8000;

/// Default number of columns
pub const DEF_NUM_COLS: usize = 80;

/// Default number of rows
pub const DEF_NUM_ROWS: usize = 25;

/// Default attribute word (dark white on black)
pub const DEF_ATTR: u16 = 0x0700;

/// Maximum page count
pub const MAX_PAGE: usize = 1;

/// Space character
pub const CHR_SPACE: u16 = 0x0020;

/// Maximum shadow buffer column count (worst-case)
pub const MAX_SHADOW_COLS: usize = 160;

/// Maximum shadow buffer row count (wosrt-case)
pub const MAX_SHADOW_ROWS: usize = 50;

/// Tabulation size
pub const SIZE_TABULATOR: usize = 4;

/**
    VGA console wrapper type

    # Safe application
    As this type assumes mutability, and the fact that
    instances of `VgaConsole` *may* be shared between
    threads, it is highly recommended that any instances
    are contained within a thread-safe lock with interior
    mutability, such as [`Mutex`].

    An example applicatios following such advice is as follows:
    ```rust
    use common::shared::io::Write;
    use common::shared::structs:spin_lock::Mutex;
    use common::plat::pc_bios::vga::console::VgaConsole;

    static VGA_CONSOLE: Mutex<VgaConsole> = Mutex::new(VgaConsole::defaults());

    fn hello() {
        let mut handle = VGA_CONSOLE.lock();
        writeln!(&mut handle, "Hello, world!").unwrap();
    }
    ```

    [`Mutex`]: crate::shared::structs::spin_lock::Mutex
*/

// - lifetime is made explicit, as we are
// dealing with raw pointer arithmetic
// TODO: refine shadow buffering
// TODO: implement page switching
#[repr(C)]
pub struct VgaConsole<'a> {
    buf: *const VolatileCell<u16>,
    cols: usize,
    rows: usize,
    page: usize,
    x: usize,
    y: usize,
    attr: u16,
    trunc: bool,
    buffered: bool,
    escaped: bool,
    shadow: Option<RingBuf<'a, u16>>,
    _marker: PhantomData<&'a VolatileCell<u16>>,
}

impl<'a> VgaConsole<'a> {
    /**
        Create new instance of `VgaConsole`

        # Safety
        It is the instantiator's responsibility to ensure that `addr`
        points to valid video memory, and that the provided dimensions
        `cols` and `rows`
        - are correct for the current video mode, and
        - if page-switching is reported, `addr + 2 * N * cols * rows`
        does not exceed valid video memory
    */
    pub const unsafe fn new(buf: *const VolatileCell<u16>, cols: usize, rows: usize) -> Self {
        // Set address and dimensions, and then
        // never touch them again...
        // (currently configured to work like a typewriter)
        // - using `&'a VolatileCell<u16>` is unfortunately
        // UB, as it has no provenance ...which means that
        // we're operationg at the very edge of UB, which
        // is not fun
        VgaConsole {
            buf,
            cols,
            rows,
            page: 0,
            x: 0,
            y: rows - 1,
            attr: DEF_ATTR,
            trunc: true,
            buffered: false,
            escaped: false,
            shadow: None,
            _marker: PhantomData,
        }
    }

    /**
        Create new instance of `VgaConsole` with default values

        # Safety
        It is the instantiator's responsibility to ensure that, at the minimum,
        - logical range `0xb0000-0xbffff` maps to VGA text memory
        at physical range `0xb0000-0xbffff`
        - VGA video mode is set to mode `0x03` (80x25 text mode)
    */
    pub const unsafe fn defaults() -> Self {
        unsafe { Self::new(DEF_BUF_ADDR as *const _, DEF_NUM_COLS, DEF_NUM_ROWS) }
    }

    /**
        Set console dimensions

        # Safety
        It is the caller's responsibility to ensure that the
        provided dimensions are appropriate for the current
        display mode.
    */
    pub unsafe fn set_dims(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
    }

    /**
        Initialize the internal shadow buffer

        This is not strictly necessary for normal use,
        but performance may degrade significantly.

        # Safety
        One should call [`set_dims()`] before calling `init()`,
        as `init()` commits the console to a specific buffer
        geometry.

        [`set_dims()`]: Self::set_dims
    */
    pub fn init(&mut self, buf: &'a mut [u16]) {
        // Only allow buffering if dimensions are
        // within "worst-case" bounds
        let s = RingBuf::new(buf, self.cols, self.rows);
        if s.is_some() {
            self.shadow = s;
            self.buffered = true;
        }
    }

    /// Check whether the console uses shadow-buffering
    pub fn is_shadowed(&self) -> bool {
        // - use state of `self.shadow` in case
        // `self.buffered` is set for some reason
        self.buffered && self.shadow.is_some()
    }

    /// Attempt to enable shadow-buffering
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

    /// Disable shadow buffering
    // (without throwing away the buffer, of course)
    pub fn unset_shadowed(&mut self) {
        self.buffered = false;
    }

    /**
        Set cursor position

        Currently does nothing.
    */
    // TODO: move hardware cursor
    pub fn set_cursor_pos(&mut self, x: usize, y: usize) {
        // Clamp provided coordinates
        self.x = x.min(self.cols - 1);
        self.y = y.min(self.rows - 1);
    }

    /// Get truncation mode
    pub fn get_trunc(&self) -> bool {
        self.trunc
    }

    /// Set truncation mode
    // - faithful terminal emulation conflicts
    // with time-saving truncation
    pub fn set_trunc(&mut self, t: bool) {
        self.trunc = t;
    }

    /// Clear screen, without resetting the cursor
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
        // - calculate number of cells per page
        let n = self.cols * self.rows;

        // - `pointer::add` antics are counter-intuitive, innit?
        unsafe { from_raw_parts(self.buf.add(page * n), n) }
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

    // Internal: advance cursor to the nearest
    // multiple of `SIZE_TABULATOR`
    #[inline(always)]
    fn tabulate(&mut self) {
        // 1. calculate how many cells to skip
        let n = if self.x % SIZE_TABULATOR > 0 {
            SIZE_TABULATOR - (self.x % SIZE_TABULATOR)
        } else {
            SIZE_TABULATOR
        };

        // 2. write spaces to pad
        for _ in 0..n {
            self.write_char(0x20);
        }
    }

    // Internal: handle special character sequences, if any
    // - Returns `Ok(Some(()))` if a special character is detected,
    //   or if the current character sequence is so far valid
    // - Returns `Ok(None)` if no special character is detected,
    //   or if the last character sequence has been accepted
    // - Returns `Err(u8)` if an invalid character in the current
    //   character sequence is detected
    // TODO: (try not to make the state machine too complex)
    #[inline(always)]
    fn handle_special(&mut self, chr: u8) -> Result<Option<()>, u8> {
        // Handle escape sequences if we're currently in one
        if self.escaped {
            // Propagate result
            return self.handle_esc_seq(chr);
        }

        // Otherwise, process special characters individually
        match chr {
            b'\n' => self.new_line(),
            b'\r' => {
                self.x = 0;
            }
            b'\t' => self.tabulate(),
            _ => return Ok(None),
        }

        Ok(Some(()))
    }

    // Internal: handle escape sequences specifically, if any
    // - Returns `Ok(Some(()))` if the current escape sequence
    //   is so far valid
    // - Returns `Ok(None)` if no escape sequence is detected,
    //   or if the last escape sequence has been accepted
    // - Returns `Err(u8)` if an invalid character in the current
    //   escape sequence is detected
    // TODO: (try not to make the state machine too complex)
    #[inline(always)]
    fn handle_esc_seq(&mut self, _chr: u8) -> Result<Option<()>, u8> {
        // TODO
        self.escaped = false;
        Ok(Some(()))
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
        match self.handle_special(chr) {
            Ok(Some(())) => {
                return;
            }
            Ok(None) => { /* no-op */ }
            Err(_) => { /* TODO */ }
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
        // - perform flush only if shadowing is enabled
        if self.is_shadowed() {
            // - trust that the buffer is initialized
            let shadow = self.shadow.as_ref().unwrap();

            // - copy rows by iterating over them, then
            // copying each cell (column-indexed)
            for r in 0..self.rows {
                // - calculate references to lines, so as to save cycles
                let buf_line = unsafe { self.line_get_ref(self.page, r) };
                let shadow_line = &shadow[r];

                for c in 0..self.cols {
                    // - perform volatile write to the text buffer
                    buf_line[c].store(shadow_line[c]);
                }
            }
        }
    }
}

impl Write for VgaConsole<'_> {
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
        let b_ref = if self.trunc { &buf[n - m..] } else { &buf[..] };

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

// - YOLO!
unsafe impl Sync for VgaConsole<'_> {}
unsafe impl Send for VgaConsole<'_> {}
