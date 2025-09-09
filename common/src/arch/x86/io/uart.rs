/*
    UART driver module for x86

    This module only supports PIO and byte-wise I/O, as the
    UART only supports a maximum word width of 8 bits, in
    addition to parity and stop bits.

    TODO: implement detailed error types
*/

// Use internal definitions
use crate::arch::x86::io::{in_b, out_b};
use crate::shared::io::{CharDevice, LockableDevice};
use crate::shared::io::uart::UartPort;
use crate::shared::structs::VolatileCell;

// Defintion uses
use core::arch::asm;
use core::hint::spin_loop;
use core::ops::Drop;
use core::sync::atomic::{AtomicBool, Ordering};
use core::marker::{Sync, PhantomData};

// Port base addresses
// - count from COM1
pub const BASE_COM1: usize = 0x3f8;
pub const BASE_COM2: usize = 0x2f8;
pub const BASE_COM3: usize = 0x3e8;
pub const BASE_COM4: usize = 0x2e8;

// Register addresses (relative to base)
pub const RX_BUF: usize = 0;
pub const TX_BUF: usize = 0;
pub const DIVISOR_LOW: usize = 0;
pub const IE_REG: usize = 1;
pub const DIVISOR_HIGH: usize = 1;
pub const INTERRUPT_ID: usize = 2;
pub const FIFO_CR: usize = 2;
pub const LINE_CR: usize = 3;
pub const MODEM_CR: usize = 4;
pub const LINE_SR: usize = 5;
pub const MODEM_SR: usize = 6;
pub const SCRATCH_REG: usize = 7;

// Baud rate (for accountability purposes)
pub const BAUD_RATE: usize = 115_200;

// Bit masks
pub const DATA_READY: u8 = 1 << 0;
pub const OVERRUN_ERR: u8 = 1 << 1;
pub const PARITY_ERR: u8 = 1 << 2;
pub const FRAMING_ERR: u8 = 1 << 3;
pub const BREAK_IND: u8 = 1 << 4;
pub const THR_EMPTY: u8 = 1 << 5;
pub const TX_EMPTY: u8 = 1 << 6;
pub const IMPEND_ERR: u8 = 1 << 7;

pub const LCR_DLAB: u8 = 1 << 7;
pub const LCR_8N1: u8 = (1 << 1) | (1 << 0);

pub const LCR_8N1_NO_DLAB: u8 = LCR_8N1;

// MCR mode constants
pub const MCR_RTS_DSR: u8 = 0x03;
pub const MCR_LOOPBACK: u8 = 0x1e;

// FIFO CR value
// - enable FIFOs
// - clear FIFOs,
// - set interrupt level to 14 bytes
pub const FIFO_CR_VAL: u8 = 0xc7;

// Default UART ports in x86
// - count from zero
pub const RAW_UART_PORTS: [UartPort; 4] = [
    UartPort::new(0, BASE_COM1),
    UartPort::new(1, BASE_COM2),
    UartPort::new(2, BASE_COM3),
    UartPort::new(3, BASE_COM4),
];

// Read from Line Status Register
#[inline(always)]
fn load_lsr(port: UartPort) -> u8 {
    // Obtain port number
    let port_num = (port.addr + LINE_SR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LSR
    unsafe { in_b(port_num) }
}

// Read from Modem Status Register
#[inline(always)]
fn load_msr(port: UartPort) -> u8 {
    // Obtain port number
    let port_num = (port.addr + MODEM_SR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LSR
    unsafe { in_b(port_num) }
}

// Read from Line Control Register
#[inline(always)]
fn load_lcr(port: UartPort) -> u8 {
    // Obtain port number
    let port_num = (port.addr + LINE_CR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LCR
    unsafe { in_b(port_num) }
}

// Write to Line Control Register
#[inline(always)]
fn store_lcr(port: UartPort, val: u8) {
    // Obtain port number
    let port_num = (port.addr + LINE_CR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LSR
    unsafe {
        out_b(port_num, val);
    }
}

// Read from Modem Control Register
#[inline(always)]
fn load_mcr(port: UartPort) -> u8 {
    // Obtain port number
    let port_num = (port.addr + MODEM_CR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LCR
    unsafe { in_b(port_num) }
}

// Write to Modem Control Register
#[inline(always)]
fn store_mcr(port: UartPort, val: u8) {
    // Obtain port number
    let port_num = (port.addr + MODEM_CR) as u16;

    // SAFETY: This should be fine, as we're
    // simply reading the contents of the LSR
    unsafe {
        out_b(port_num, val);
    }
}

// Fetch old value of Interrupt Enable Register,
// then replace it with provided value
// TODO: make operation atomic if possible
#[inline(always)]
fn exchange_int(port: UartPort, new_val: u8) -> u8 {
    // Obtain port number
    let port_num = (port.addr + IE_REG) as u16;

    // Obtain old value, masking bits 7-4
    let old_val: u8 = unsafe { in_b(port_num) & 0x0F };

    // Set IE with new value
    unsafe {
        out_b(port_num, new_val);
    }

    // Return old value
    old_val
}

// Check Data Ready bit
#[inline(always)]
fn is_dr_set(port: UartPort) -> bool {
    (load_lsr(port) & DATA_READY) != 0
}

// Wait until Data Ready bit is set
#[inline(always)]
fn wait_dr_set(port: UartPort) {
    // Poll DR bit
    while !is_dr_set(port) {
        // Perform busy-waiting
        spin_loop();
    }
}

// Check Transmitter Holding Register Empty bit
#[inline(always)]
fn is_thre_set(port: UartPort) -> bool {
    (load_lsr(port) & THR_EMPTY) != 0
}

// Wait until THRE is set
#[inline(always)]
fn wait_thre_set(port: UartPort) {
    // Poll DR bit
    while !is_thre_set(port) {
        // Perform busy-waiting
        spin_loop();
    }
}

// Check if *any* of the error bits are set
#[inline(always)]
fn is_err_set(port: UartPort) -> bool {
    (load_lsr(port) & (OVERRUN_ERR | PARITY_ERR | FRAMING_ERR | IMPEND_ERR)) != 0
}

// Read to byte array
// - stops on source exhaustion OR target overrun
// - returns bytes read on success
// TODO: serial I/O usually never fails, but there
// might be edge cases that we have failed to take
// into account...
fn read_bytes(port: UartPort, buf: &mut [u8], wait: bool, fill: bool) -> Result<usize, ()> {
    // Count number of bytes read
    // - use as start-from-zero index
    let mut num_bytes: usize = 0;

    // Store buffer size
    let buf_size: usize = buf.len();

    // Obtain port number
    let port_num = (port.addr + RX_BUF) as u16;

    // Break on source exhaustion (maskable)
    // OR target overrun, whichever happens
    // first
    while (is_dr_set(port) || fill) && (num_bytes < buf_size) {
        // Break on error
        if is_err_set(port) {
            return Err(());
        }

        // Wait ONLY if `fill == true` OR
        // if `wait == true`
        if fill || wait {
            wait_dr_set(port);
        }

        // Break on error
        if is_err_set(port) {
            return Err(());
        }

        buf[num_bytes] = unsafe { in_b(port_num) };
        num_bytes += 1;
    }

    Ok(num_bytes)
}

// Write from byte array
// - stops on source exhaustion OR target overrun
// - returns bytes read on success
// TODO: serial I/O is usually reliable, but there
// might be edge cases that we have failed to take
// into account...
fn write_bytes(port: UartPort, buf: &[u8], wait: bool, consume: bool) -> Result<usize, ()> {
    // Count number of bytes written
    // - use as start-from-zero index
    let mut num_bytes: usize = 0;

    // Store buffer size
    let buf_size: usize = buf.len();

    // Obtain port number
    let port_num = (port.addr + TX_BUF) as u16;

    // Break on source exhaustion OR target
    // overrun, whichever happens first
    while (is_thre_set(port) || consume) && (num_bytes < buf_size) {
        // Break on error
        if is_err_set(port) {
            return Err(());
        }

        // Wait ONLY if `conume == true` OR
        // if `wait == true`
        if consume || wait {
            wait_thre_set(port);
        }

        // Break on error
        if is_err_set(port) {
            return Err(());
        }

        unsafe {
            out_b(port_num, buf[num_bytes]);
        }

        num_bytes += 1;
    }

    Ok(num_bytes)
}

// Initialization error types
#[derive(Debug, Copy, Clone)]
pub enum InitError {
    ZeroBaudRate,
    InvalidDivisor,
    LoopbackError,
    Uninitialized,
}

// Port initialization status
#[repr(usize)]
#[derive(Debug, Copy, Clone)]
pub enum PortInit {
    Uninit(UartPort),
    Init(UartPort),
}

// Abstract polling UART instance
// - uses interior mutability
// - cannot be read from or written
// to directly; must be locked for
// exclusive I/O
pub struct PollingUart<'a> {
    port: VolatileCell<PortInit>,
    _lock: AtomicBool,
    _marker: PhantomData<&'a AtomicBool>,
}

impl PollingUart<'_> {
    // Create uninitialized UART instance
    pub const fn new(port: UartPort) -> Self {
        PollingUart {
            port: VolatileCell::new(PortInit::Uninit(port)),
            _lock: AtomicBool::new(false),
            _marker: PhantomData,
        }
    }

    // Query if the port is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        match self.port.load() {
            PortInit::Init(_) => true,
            _ => false,
        }
    }

    // Initializes UART instance using
    // provided UART port and optional
    // baud rate
    // - the baud rate must be an integer
    // fraction of `BAUD_RATE` (115,200 baud)
    // - returns `Ok(())` on success
    #[inline(never)]
    pub fn initialize(&self, rate: Option<usize>) -> Result<(), InitError> {
        unsafe { asm!("xchg bx, bx"); }

        // Obtain write lock
        self.obtain_lock();

        unsafe { asm!("xchg cx, cx"); }
        // Check if port is already initialized
        // - DO NOT propagate error, as that would
        // lead to the lock being "poisoned"
        let ret = match self.port.load() {
            PortInit::Uninit(port) => unsafe { self.__initialize(port, rate) },
            PortInit::Init(_) => Ok(()),
        };

        // Release write lock
        unsafe { self.release_lock(); }

        ret
    }

    // Obtain exclusive lock
    #[inline(always)]
    fn obtain_lock(&self) {
        let new = true;
        let mut old = false;

        loop {
            match self
                ._lock
                .compare_exchange(old, new, Ordering::SeqCst, Ordering::Acquire)
            {
                Ok(_) => {
                    break;
                }
                Err(x) => {
                    old = x;
                    spin_loop();
                }
            }
        }
    }

    // Release exclusive lock
    // - the operation is safe in theory, but
    // opening for race cond
    #[inline(always)]
    unsafe fn release_lock(&self) {
        self._lock.store(false, Ordering::Release);
    }

    // // Initializes UART instance using
    // provided UART port and optional
    // baud rate
    // - the baud rate must be an integer
    // fraction of `BAUD_RATE` (115,200 baud)
    // - returns `Ok(())` on success
    // Initializes UART instance using
    // provided UART port and optional
    // baud rate
    // - the baud rate must be an integer
    // fraction of `BAUD_RATE` (115,200 baud)
    // - returns `Ok(())` on success
    // - must be invoked by `initialize`, as this
    // does not obtain or release the write lock
    #[inline(never)]
    unsafe fn __initialize(&self, port: UartPort, rate: Option<usize>) -> Result<(), InitError> {
        // Calculate rate divisor
        let divisor: u16 = match rate {
            Some(0) => {
                unsafe { self.release_lock(); }
                return Err(InitError::ZeroBaudRate);
            },

            Some(r) if BAUD_RATE % r != 0 => {
                unsafe { self.release_lock(); }
                return Err(InitError::InvalidDivisor);
            },

            Some(r) => (BAUD_RATE / r) as u16,
            None => 1,
        };

        // Clear interrupts
        // - return value reserved for future use
        let _ = exchange_int(port, 0x00);

        // Enable DLAB
        store_lcr(port, LCR_DLAB);

        // Set divisor
        // - trust LE encoding
        let divisor_low = divisor as u8;
        let divisor_high = (divisor >> 8) as u8;

        unsafe {
            out_b((port.addr + DIVISOR_LOW) as u16, divisor_low);
            out_b((port.addr + DIVISOR_HIGH) as u16, divisor_high);
        }

        // Set encoding to 8/N/1, clearing DLAB
        store_lcr(port, LCR_8N1_NO_DLAB);

        // Write to FIFO CR
        // - enable FIFO, clear FIFOs, interrupt at 14 bytes
        unsafe {
            out_b((port.addr + FIFO_CR) as u16, FIFO_CR_VAL);
        }

        // Set RTS+DSR, then set in loopback mode
        store_mcr(port, MCR_RTS_DSR);
        store_mcr(port, MCR_LOOPBACK);

        // Write to TX buffer
        unsafe {
            out_b((port.addr + TX_BUF) as u16, 0xae);
        }

        // Return to normal operation
        store_mcr(port, MCR_RTS_DSR);

        // Check if serial returns sent byte
        let val = unsafe { in_b((port.addr + RX_BUF) as u16) };
        if val != 0xae {
            Err(InitError::LoopbackError)
        } else {
            // Set internal port
            // - `self.port` has interior mutability
            self.port.store(PortInit::Init(port));

            Ok(())
        }
    }
}

unsafe impl Sync for PollingUart<'_> {}
impl !CharDevice<'_> for PollingUart<'_> {}

impl<'a> LockableDevice<'a> for PollingUart<'a> {
    // - should be enforced by constraints in
    // `UartDevice`
    type GuardType = PollingUartGuard<'a>;
    type Error = InitError;

    // Locks UART instance and returns guard type
    // with I/O traits, but only if the instance
    // has been initialized
    fn lock(&'a self) -> Result<Self::GuardType, Self::Error> {
        // Obtain exclusive lock
        self.obtain_lock();

        // Check if the instance has been correctly instantiated
        if let PortInit::Init(port) = self.port.load() {
            Ok(PollingUartGuard {
                port,
                lock: &self._lock,
            })
        } else {
            // Release exclusive lock
            unsafe {
                self.release_lock();
            }

            Err(InitError::Uninitialized)
        }
    }
}

// Guard type for `PollingUart`
// - the only type allowed to implement
// read and write access, as it is owned
// entirely by the instantiating scope
pub struct PollingUartGuard<'a> {
    port: UartPort,
    lock: &'a AtomicBool,
}

// - implement automatic lock release
impl Drop for PollingUartGuard<'_> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

// - mark type as a character device
impl CharDevice<'_> for PollingUartGuard<'_> {
    type ReadError = ();
    type WriteError = ();

    // Read some bytes from serial input
    fn char_read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        read_bytes(self.port, buf, true, false)
    }

    // Write some bytes to serial output
    fn char_write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        write_bytes(self.port, buf, true, false)
    }
}

impl !LockableDevice<'_> for PollingUartGuard<'_> {}
