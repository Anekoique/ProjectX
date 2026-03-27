use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use super::Device;
use crate::{
    config::Word,
    error::{XError, XResult},
};

pub struct Uart {
    ier: u8,
    lcr: u8,
    mcr: u8,
    dll: u8,
    dlm: u8,
    scr: u8,
    rx_fifo: VecDeque<u8>,
    rx_buf: Arc<Mutex<VecDeque<u8>>>,
}

impl Uart {
    /// TX-only UART. No background thread, no RX input.
    pub fn new() -> Self {
        Self {
            ier: 0,
            lcr: 0x03,
            mcr: 0,
            dll: 0,
            dlm: 0,
            scr: 0,
            rx_fifo: VecDeque::new(),
            rx_buf: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// UART with TCP RX backend. Spawns a listener thread on the given port.
    /// Port 0 lets the OS pick a free port. Bind failure degrades to TX-only.
    #[allow(dead_code)]
    pub fn with_tcp(port: u16) -> Self {
        let buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));
        let rx = buf.clone();
        std::thread::spawn(move || {
            let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) else {
                warn!("UART: TCP bind failed on port {port}, TX-only");
                return;
            };
            info!("UART: listening on {}", listener.local_addr().unwrap());
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            use std::io::Read;
            let mut reader = std::io::BufReader::new(stream);
            let mut b = [0u8; 1];
            while reader.read_exact(&mut b).is_ok() {
                rx.lock().unwrap().push_back(b[0]);
            }
        });
        Self {
            rx_buf: buf,
            ..Self::new()
        }
    }

    fn dlab(&self) -> bool {
        self.lcr & 0x80 != 0
    }

    fn lsr(&self) -> u8 {
        (if self.rx_fifo.is_empty() { 0 } else { 0x01 }) | 0x60
    }

    fn iir(&self) -> u8 {
        if !self.rx_fifo.is_empty() && self.ier & 0x01 != 0 {
            0xC4
        } else {
            0xC1
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
                use std::io::Write;
                let _ = std::io::stdout()
                    .lock()
                    .write_all(&[b])
                    .and_then(|_| std::io::stdout().flush());
            }
            1 if self.dlab() => self.dlm = b,
            1 => self.ier = b & 0x0F,
            2 => {}
            3 => self.lcr = b,
            4 => self.mcr = b,
            7 => self.scr = b,
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        if let Ok(mut buf) = self.rx_buf.try_lock() {
            self.rx_fifo.extend(buf.drain(..));
        }
    }

    fn irq_line(&self) -> bool {
        !self.rx_fifo.is_empty() && self.ier & 0x01 != 0
    }

    /// Reset clears emulator-side register state, rx_fifo, and rx_buf.
    /// The TCP backend thread (if any) is preserved — post-reset bytes
    /// from the same connection continue to arrive, just like real hardware
    /// where a FIFO reset does not disconnect the serial line.
    fn reset(&mut self) {
        self.ier = 0;
        self.lcr = 0x03;
        self.mcr = 0;
        self.dll = 0;
        self.dlm = 0;
        self.scr = 0;
        self.rx_fifo.clear();
        self.rx_buf.lock().unwrap().clear();
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
        // Backend still usable — new data arrives normally
        u.rx_buf.lock().unwrap().push_back(0xBB);
        u.tick();
        assert_eq!(u.rx_fifo.len(), 1);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xBB);
    }

    #[test]
    fn tcp_bind_failure_falls_back_to_tx_only() {
        let blocker = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = blocker.local_addr().unwrap().port();
        let mut u = Uart::with_tcp(port);
        std::thread::sleep(std::time::Duration::from_millis(50));
        u.tick();
        assert!(u.rx_fifo.is_empty());
        assert!(u.write(0, 1, b'A' as Word).is_ok());
    }

    #[test]
    fn tcp_disconnect_stops_rx() {
        use std::{io::Write, net::TcpStream};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut u = Uart::with_tcp(port);
        std::thread::sleep(std::time::Duration::from_millis(50));
        {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(&[0xAA, 0xBB]).unwrap();
            s.flush().unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        u.tick();
        assert_eq!(u.rx_fifo.len(), 2);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xAA);
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xBB);
        u.tick();
        assert!(u.rx_fifo.is_empty());
    }

    #[test]
    fn tcp_reset_clears_and_preserves_backend() {
        use std::{io::Write, net::TcpStream};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut u = Uart::with_tcp(port);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();

        // Pre-reset data
        s.write_all(&[0x11]).unwrap();
        s.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        u.tick();
        assert_eq!(u.rx_fifo.len(), 1);

        // Reset clears everything
        u.reset();
        assert!(u.rx_fifo.is_empty());

        // Post-reset: backend still works
        s.write_all(&[0xAA]).unwrap();
        s.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        u.tick();
        assert_eq!(u.rx_fifo.len(), 1, "post-reset RX must work");
        assert_eq!(u.read(0, 1).unwrap() as u8, 0xAA);
    }
}
