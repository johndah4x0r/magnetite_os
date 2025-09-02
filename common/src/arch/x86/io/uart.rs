/*
    UART driver module for x86

    The UART only supports byte-sized operations,
    and this module suppors PIO only.

    TODO: implement `Read` and `Write`
*/

// Use internal definitions
use crate::arch::x86::io::{in_b, out_b};
use crate::shared::structs::UartPort;

// Defintion uses
use core::hint::spin_loop;

// Default UART ports in x86
// - count from zero
pub const UART_PORTS: [UartPort; 4] = [
    UartPort::new(0, 0x3f8),
    UartPort::new(1, 0x2f8),
    UartPort::new(2, 0x3e8),
    UartPort::new(3, 0x2e8),
];

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
// - returns bytes read, regardless of outcome
// TODO: serial I/O usually never fails, but there
// might be edge cases that we have failed to take
// into account...
fn read_bytes(port: UartPort, buf: &mut [u8], wait: bool, fill: bool) -> Result<usize, usize> {
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
    while (is_dr_set(port) || fill) && (num_bytes < buf_size) && (!is_err_set(port)) {
        // Wait ONLY if `fill == true` OR
        // if `wait == true`
        if fill || wait {
            wait_dr_set(port);
        }

        buf[num_bytes] = unsafe { in_b(port_num) };
        num_bytes += 1;
    }

    Ok(num_bytes)
}

// Write from byte array
// - stops on source exhaustion OR target overrun
// - returns bytes read, regardless of outcome
// TODO: serial I/O is usually reliable, but there
// might be edge cases that we have failed to take
// into account...
fn write_bytes(port: UartPort, buf: &[u8], wait: bool, consume: bool) -> Result<usize, usize> {
    // Count number of bytes written
    // - use as start-from-zero index
    let mut num_bytes: usize = 0;

    // Store buffer size
    let buf_size: usize = buf.len();

    // Obtain port number
    let port_num = (port.addr + TX_BUF) as u16;

    // Break on source exhaustion OR target
    // overrun, whichever happens first
    while (is_thre_set(port) || consume) && (num_bytes < buf_size) && (!is_err_set(port)) {
        // Wait ONLY if `conume == true` OR
        // if `wait == true`
        if consume || wait {
            wait_thre_set(port);
        }

        unsafe {
            out_b(port_num, buf[num_bytes]);
        }
        num_bytes += 1;
    }

    Ok(num_bytes)
}

// Abstract polling UART instance
// - assumes explicit mutability
pub struct PollingUart {
    port: Option<UartPort>,
}

impl PollingUart {
    // Create uninitialized UART instance
    pub const fn new() -> Self {
        PollingUart { port: None }
    }

    // Initializes UART instance using
    // provided UART port and optional
    // baud rate
    // - the baud rate must be an integer
    // fraction of `BAUD_RATE` (115,200 baud)
    // - returns `Ok(())` on success
    pub fn initialize(&mut self, port: UartPort, rate: Option<usize>) -> Result<(), ()> {
        // Calculate rate divisor
        let divisor: u16;

        if let Some(r) = rate {
            // - forbid zero rate
            if r == 0 {
                return Err(());
            }

            // - divisibility WILL be enforced
            if BAUD_RATE % r != 0 {
                return Err(());
            }

            // - alias to 16-bit integer, as
            // the divisor registers only accept
            // one byte (8 bits) each
            divisor = (BAUD_RATE / r) as u16;
        } else {
            divisor = 1;
        }

        // Clear interrupts
        // - return value reserved for future use
        let _ = exchange_int(port, 0x00);

        // Enable DLAB
        store_lcr(port, 0x80);

        // Set divisor
        // - trust LE encoding
        let divisor_low = divisor as u8;
        let divisor_high = (divisor >> 8) as u8;

        unsafe {
            out_b((port.addr + DIVISOR_LOW) as u16, divisor_low);
            out_b((port.addr + DIVISOR_HIGH) as u16, divisor_high);
        }

        // Set encoding to 8/N/1, clearing DLAB
        store_lcr(port, 0x03);

        // Write to FIFO CR
        // - enable FIFO, clear FIFOs, interrupt at 14 bytes
        unsafe {
            out_b((port.addr + FIFO_CR) as u16, 0xc7);
        }

        // Set RTS+DSR, then set in loopback mode
        store_mcr(port, 0x03);
        store_mcr(port, 0x1e);

        // Write to TX buffer
        unsafe {
            out_b((port.addr + TX_BUF) as u16, 0xae);
        }

        // Check if serial returns sent byte
        let val = unsafe { in_b((port.addr + RX_BUF) as u16) };
        if val != 0xae {
            Err(())
        } else {
            // Set internal port
            self.port = Some(port);

            Ok(())
        }
    }
}
