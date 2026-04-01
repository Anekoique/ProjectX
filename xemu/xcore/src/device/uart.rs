use std::{
    collections::VecDeque,
    os::fd::{AsRawFd, OwnedFd},
    sync::{Arc, Mutex},
};

use super::Device;
use crate::{
    config::Word,
    error::{XError, XResult},
};

fn pty_write(fd: &OwnedFd, data: &[u8]) {
    // Non-blocking: drops bytes if PTY buffer is full (no reader attached yet).
    unsafe { libc::write(fd.as_raw_fd(), data.as_ptr().cast(), data.len()) };
}

fn set_nonblock(fd: &OwnedFd) {
    unsafe {
        let flags = libc::fcntl(fd.as_raw_fd(), libc::F_GETFL);
        libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

fn open_pty() -> Result<(OwnedFd, OwnedFd, String), String> {
    use std::os::fd::FromRawFd;

    let (mut master, mut slave) = (0, 0);
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0
    {
        return Err("openpty failed".into());
    }

    // Raw input, preserve \n → \r\n output processing.
    unsafe {
        let mut attr = std::mem::MaybeUninit::uninit();
        if libc::tcgetattr(slave, attr.as_mut_ptr()) == 0 {
            let mut attr = attr.assume_init();
            libc::cfmakeraw(&mut attr);
            attr.c_oflag |= libc::OPOST | libc::ONLCR;
            libc::tcsetattr(slave, libc::TCSANOW, &attr);
        }
    }

    let name = unsafe {
        let ptr = libc::ttyname(slave);
        if ptr.is_null() {
            "unknown".to_string()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };

    unsafe {
        Ok((
            OwnedFd::from_raw_fd(master),
            OwnedFd::from_raw_fd(slave),
            name,
        ))
    }
}

static mut ORIG_TERMIOS: Option<libc::termios> = None;

extern "C" fn restore_termios() {
    unsafe {
        if let Some(ref t) = ORIG_TERMIOS {
            libc::tcsetattr(0, libc::TCSANOW, t);
        }
    }
}

extern "C" fn restore_and_exit(_sig: i32) {
    restore_termios();
    std::process::exit(0);
}

/// Spawn a background reader thread that drains `fd` into a shared buffer.
/// Non-blocking fds (PTY) retry on EAGAIN; blocking fds (stdin) block
/// naturally.
fn spawn_reader(fd: i32) -> Arc<Mutex<VecDeque<u8>>> {
    let buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));
    let rx = buf.clone();
    std::thread::spawn(move || {
        let mut b = [0u8; 64];
        loop {
            let n = unsafe { libc::read(fd, b.as_mut_ptr().cast(), b.len()) };
            if n > 0 {
                rx.lock().unwrap().extend(&b[..n as usize]);
            } else if n == 0 {
                break;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    });
    buf
}

enum TxSink {
    Stdout,
    /// PTY for RX input + stdout echo for visible boot output.
    PtyWithStdout(OwnedFd),
}

pub struct Uart {
    // NS16550 registers
    ier: u8,
    lcr: u8,
    mcr: u8,
    dll: u8,
    dlm: u8,
    scr: u8,
    // TX: THRE interrupt fires when THR empties.
    // thre_pending stages through tick() so the CPU has a chance to write
    // another byte before the interrupt fires (avoids per-char trap storm).
    thre_pending: bool,
    thre_ip: bool,
    // RX pipeline: reader thread → rx_buf → tick() → rx_fifo → guest read
    rx_fifo: VecDeque<u8>,
    rx_buf: Arc<Mutex<VecDeque<u8>>>,
    tx: TxSink,
    // Prevent PTY teardown while the UART is alive.
    _pty_slave: Option<OwnedFd>,
}

impl Default for Uart {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart {
    /// TX-only UART. Output goes to stdout, no RX.
    pub fn new() -> Self {
        Self {
            ier: 0,
            lcr: 0x03,
            mcr: 0,
            dll: 0,
            dlm: 0,
            scr: 0,
            thre_pending: false,
            thre_ip: false,
            rx_fifo: VecDeque::new(),
            rx_buf: Arc::new(Mutex::new(VecDeque::new())),
            tx: TxSink::Stdout,
            _pty_slave: None,
        }
    }

    /// Bidirectional UART using the process's stdin/stdout.
    /// TX goes to stdout, RX reads from stdin (raw mode).
    pub fn with_stdio() -> Self {
        // Save original termios and put stdin into raw mode.
        unsafe {
            let mut orig = std::mem::MaybeUninit::uninit();
            if libc::tcgetattr(0, orig.as_mut_ptr()) == 0 {
                let orig = orig.assume_init();
                // Restore terminal on exit (normal or Ctrl-C).
                ORIG_TERMIOS = Some(orig);
                libc::atexit(restore_termios);
                libc::signal(
                    libc::SIGINT,
                    restore_and_exit as extern "C" fn(i32) as libc::sighandler_t,
                );
                let mut raw = orig;
                libc::cfmakeraw(&mut raw);
                libc::tcsetattr(0, libc::TCSANOW, &raw);
            }
        }
        Self {
            rx_buf: spawn_reader(0),
            ..Self::new()
        }
    }

    /// UART backed by a pseudo-terminal. TX and RX go through the PTY master;
    /// the slave path is printed so the user can `screen <path>` in another
    /// terminal.
    pub fn with_pty() -> Result<Self, String> {
        let (master, slave, name) = open_pty()?;
        let rx_buf = spawn_reader(master.as_raw_fd());

        // Non-blocking TX prevents emulator from stalling when no reader
        // is attached. Bytes are dropped until `screen` connects.
        set_nonblock(&master);
        eprintln!("UART: serial console at {name}");
        eprintln!("UART: attach with: screen {name}");

        Ok(Self {
            rx_buf,
            tx: TxSink::PtyWithStdout(master),
            _pty_slave: Some(slave),
            ..Self::new()
        })
    }

    fn dlab(&self) -> bool {
        self.lcr & 0x80 != 0
    }

    fn lsr(&self) -> u8 {
        let dr = u8::from(!self.rx_fifo.is_empty());
        dr | 0x60 // THRE + TEMT always set
    }

    fn iir(&mut self) -> u8 {
        // Priority: RX data > THRE (NS16550 spec)
        if !self.rx_fifo.is_empty() && self.ier & 0x01 != 0 {
            0xC4 // RX data available (priority 2)
        } else if self.thre_ip && self.ier & 0x02 != 0 {
            self.thre_ip = false; // reading IIR clears THRE interrupt
            0xC2 // THRE — transmitter holding register empty (priority 3)
        } else {
            0xC1 // no interrupt pending
        }
    }

    fn tx_byte(&self, b: u8) {
        use std::io::Write;
        let _ = std::io::stdout()
            .lock()
            .write_all(&[b])
            .and_then(|_| std::io::stdout().flush());
        if let TxSink::PtyWithStdout(fd) = &self.tx {
            let buf = [b];
            pty_write(fd, if b == b'\n' { b"\r\n" } else { &buf });
        }
    }
}

impl Device for Uart {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        if size != 1 {
            return Err(XError::BadAddress);
        }
        Ok(match offset {
            0 if self.dlab() => self.dll,
            0 => self.rx_fifo.pop_front().unwrap_or(0),
            1 if self.dlab() => self.dlm,
            1 => self.ier,
            2 => self.iir(),
            3 => self.lcr,
            4 => self.mcr,
            5 => self.lsr(),
            6 => 0,
            7 => self.scr,
            _ => 0,
        } as Word)
    }

    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult {
        if size != 1 {
            return Err(XError::BadAddress);
        }
        let b = val as u8;
        match offset {
            0 if self.dlab() => self.dll = b,
            0 => {
                self.tx_byte(b);
                self.thre_pending = true; // assert THRE on next tick
            }
            1 if self.dlab() => self.dlm = b,
            1 => {
                let old = self.ier;
                self.ier = b & 0x0F;
                // THRE interrupt fires when IER[1] transitions 0→1 and THR is empty
                if old & 0x02 == 0 && self.ier & 0x02 != 0 {
                    self.thre_pending = true;
                }
            }
            2 => {} // FCR: FIFO control — ignored; IIR always reports FIFOs enabled
            3 => self.lcr = b,
            4 => self.mcr = b,
            7 => self.scr = b,
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        // Promote pending THRE → assertable (one-tick delay avoids per-char trap storm)
        if self.thre_pending {
            self.thre_pending = false;
            self.thre_ip = true;
        }
        if let Ok(mut buf) = self.rx_buf.try_lock() {
            self.rx_fifo.extend(buf.drain(..));
        }
    }

    fn irq_line(&self) -> bool {
        let rx = !self.rx_fifo.is_empty() && self.ier & 0x01 != 0;
        let thre = self.thre_ip && self.ier & 0x02 != 0;
        rx || thre
    }

    fn reset(&mut self) {
        debug!("uart: reset");
        self.ier = 0;
        self.lcr = 0x03;
        self.mcr = 0;
        self.dll = 0;
        self.dlm = 0;
        self.scr = 0;
        self.thre_pending = false;
        self.thre_ip = false;
        self.rx_fifo.clear();
        if let Ok(mut buf) = self.rx_buf.try_lock() {
            buf.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsr_thre_always_set() {
        let mut u = Uart::new();
        assert_ne!(u.read(5, 1).unwrap() as u8 & 0x60, 0);
    }

    #[test]
    fn lsr_dr_reflects_rx_fifo() {
        let mut u = Uart::new();
        assert_eq!(u.read(5, 1).unwrap() as u8 & 0x01, 0);
        u.rx_fifo.push_back(0x42);
        assert_ne!(u.read(5, 1).unwrap() as u8 & 0x01, 0);
    }

    #[test]
    fn rbr_pops_from_fifo() {
        let mut u = Uart::new();
        u.rx_fifo.push_back(0xAA);
        u.rx_fifo.push_back(0xBB);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xAA);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xBB);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0x00);
    }

    #[test]
    fn dlab_switches_registers() {
        let mut u = Uart::new();
        u.write(3, 1, 0x80).unwrap();
        u.write(0, 1, 0x03).unwrap();
        u.write(1, 1, 0x00).unwrap();
        assert_eq!(u.read(0, 1).unwrap() as u8, 0x03);
        assert_eq!(u.read(1, 1).unwrap() as u8, 0x00);
        u.write(3, 1, 0x03).unwrap();
        assert_eq!(u.read(1, 1).unwrap() as u8, 0x00);
    }

    #[test]
    fn ier_write_masked() {
        let mut u = Uart::new();
        u.write(1, 1, 0xFF).unwrap();
        assert_eq!(u.read(1, 1).unwrap() as u8, 0x0F);
    }

    #[test]
    fn iir_rx_data_available() {
        let mut u = Uart::new();
        u.ier = 0x01;
        assert_eq!(u.read(2, 1).unwrap() as u8, 0xC1);
        u.rx_fifo.push_back(0x42);
        assert_eq!(u.read(2, 1).unwrap() as u8, 0xC4);
    }

    #[test]
    fn irq_line_rx_data_and_ier() {
        let mut u = Uart::new();
        assert!(!u.irq_line());
        u.rx_fifo.push_back(0x42);
        assert!(!u.irq_line());
        u.ier = 0x01;
        assert!(u.irq_line());
    }

    #[test]
    fn scratch_register() {
        let mut u = Uart::new();
        u.write(7, 1, 0xAB).unwrap();
        assert_eq!(u.read(7, 1).unwrap() as u8, 0xAB);
    }

    #[test]
    fn non_byte_access_error() {
        let mut u = Uart::new();
        assert!(u.read(0, 4).is_err());
        assert!(u.write(0, 2, 0).is_err());
    }

    #[test]
    fn tick_drains_rx_buf() {
        let mut u = Uart::new();
        u.rx_buf.lock().unwrap().push_back(0x11);
        u.rx_buf.lock().unwrap().push_back(0x22);
        u.tick();
        assert_eq!(u.rx_fifo.len(), 2);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0x11);
    }

    #[test]
    fn reset_clears_buffers_and_registers() {
        let mut u = Uart::new();
        u.rx_buf.lock().unwrap().push_back(0xAA);
        u.rx_fifo.push_back(0xBB);
        u.ier = 0x0F;
        u.reset();
        assert!(u.rx_fifo.is_empty());
        assert!(u.rx_buf.lock().unwrap().is_empty());
        assert_eq!(u.ier, 0);
    }

    #[test]
    fn reset_preserves_backend_for_post_reset_rx() {
        let mut u = Uart::new();
        u.rx_buf.lock().unwrap().push_back(0xAA);
        u.reset();
        u.rx_buf.lock().unwrap().push_back(0xBB);
        u.tick();
        assert_eq!(u.rx_fifo.len(), 1);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xBB);
    }

    #[test]
    fn pty_creates_working_uart() {
        let mut u = Uart::with_pty().unwrap();
        assert_ne!(u.read(5, 1).unwrap() as u8 & 0x60, 0);
        u.write(7, 1, 0xCD).unwrap();
        assert_eq!(u.read(7, 1).unwrap() as u8, 0xCD);
    }
}
