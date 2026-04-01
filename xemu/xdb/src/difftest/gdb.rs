//! GDB Remote Serial Protocol client over TCP.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    time::Duration,
};

/// GDB Remote Serial Protocol client over a buffered TCP connection.
pub struct GdbClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    buf: Vec<u8>,
}

fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |a, &b| a.wrapping_add(b))
}

fn parse_hex_le(hex: &[u8], word_size: usize) -> Result<u64, String> {
    let len = word_size * 2;
    if hex.len() < len {
        return Err(format!("hex too short: need {len}, got {}", hex.len()));
    }
    hex[..len]
        .chunks(2)
        .enumerate()
        .try_fold(0u64, |acc, (i, pair)| {
            let s = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            let b = u8::from_str_radix(s, 16).map_err(|e| e.to_string())?;
            Ok(acc | (b as u64) << (i * 8))
        })
}

fn encode_le_hex(val: u64, word_size: usize) -> String {
    (0..word_size)
        .map(|i| format!("{:02x}", (val >> (i * 8)) & 0xFF))
        .collect()
}

fn expect_ok(resp: &[u8]) -> Result<(), String> {
    if resp.starts_with(b"OK") {
        Ok(())
    } else {
        Err(format!(
            "GDB expected OK, got: {}",
            String::from_utf8_lossy(resp)
        ))
    }
}

impl GdbClient {
    /// Connect to a GDB server.
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr).map_err(|e| format!("GDB connect to {addr}: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| format!("set timeout: {e}"))?;
        let writer = stream
            .try_clone()
            .map_err(|e| format!("clone stream: {e}"))?;
        let mut client = Self {
            reader: BufReader::new(stream),
            writer,
            buf: Vec::with_capacity(4096),
        };
        // Consume initial handshake byte(s)
        let _ = client.reader.read(&mut [0u8; 1]);
        Ok(client)
    }

    /// Send a command packet and receive the response payload.
    pub fn send_recv(&mut self, cmd: &str) -> Result<Vec<u8>, String> {
        self.send_packet(cmd)?;
        self.recv_packet()
    }

    // ── High-level commands ──

    /// Single-step the remote target.
    pub fn step(&mut self) -> Result<(), String> {
        self.send_recv("vCont;s:p1.-1").map(|_| ())
    }

    /// Continue the remote target.
    pub fn cont(&mut self) -> Result<(), String> {
        self.send_recv("vCont;c:p1.-1").map(|_| ())
    }

    /// Read all GPRs + PC.
    pub fn read_regs(&mut self) -> Result<Vec<u64>, String> {
        let data = self.send_recv("g")?;
        let word_size = if data.len() >= 33 * 16 { 8 } else { 4 };
        data.chunks(word_size * 2)
            .map(|c| parse_hex_le(c, word_size))
            .collect()
    }

    /// Write all GPRs + PC.
    pub fn write_regs(&mut self, regs: &[u64], word_size: usize) -> Result<(), String> {
        let hex: String = regs.iter().map(|&r| encode_le_hex(r, word_size)).collect();
        expect_ok(&self.send_recv(&format!("G{hex}"))?)
    }

    /// Read a single register by number.
    pub fn read_register(&mut self, num: usize, word_size: usize) -> Result<u64, String> {
        parse_hex_le(&self.send_recv(&format!("p{num:x}"))?, word_size)
    }

    /// Write a single register by number.
    pub fn write_register(&mut self, num: usize, val: u64, word_size: usize) -> Result<(), String> {
        expect_ok(&self.send_recv(&format!("P{num:x}={}", encode_le_hex(val, word_size)))?)
    }

    /// Write bytes to target memory.
    pub fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        expect_ok(&self.send_recv(&format!("M{addr:x},{:x}:{hex}", data.len()))?)
    }

    /// Insert a software breakpoint.
    pub fn set_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        expect_ok(&self.send_recv(&format!("Z0,{addr:x},4"))?)
    }

    /// Remove a software breakpoint.
    pub fn remove_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        expect_ok(&self.send_recv(&format!("z0,{addr:x},4"))?)
    }

    // ── Packet framing ──

    fn send_packet(&mut self, data: &str) -> Result<(), String> {
        write!(self.writer, "${data}#{:02x}", checksum(data.as_bytes()))
            .map_err(|e| format!("GDB send: {e}"))?;
        self.writer.flush().map_err(|e| format!("GDB flush: {e}"))?;
        self.recv_ack()
    }

    fn recv_packet(&mut self) -> Result<Vec<u8>, String> {
        // Skip until '$'
        loop {
            let mut b = [0u8; 1];
            self.reader
                .read_exact(&mut b)
                .map_err(|e| format!("GDB read: {e}"))?;
            if b[0] == b'$' {
                break;
            }
        }
        // Read until '#' using buffered read
        self.buf.clear();
        self.reader
            .read_until(b'#', &mut self.buf)
            .map_err(|e| format!("GDB read: {e}"))?;
        self.buf.pop(); // remove trailing '#'

        // Read 2 checksum hex chars
        let mut ck = [0u8; 2];
        self.reader
            .read_exact(&mut ck)
            .map_err(|e| format!("GDB read: {e}"))?;

        // Validate
        let expected = checksum(&self.buf);
        let received = u8::from_str_radix(
            std::str::from_utf8(&ck).map_err(|_| "GDB checksum: invalid UTF-8".to_string())?,
            16,
        )
        .map_err(|_| {
            format!(
                "GDB checksum: invalid hex '{}'",
                String::from_utf8_lossy(&ck)
            )
        })?;
        if expected != received {
            return Err(format!(
                "GDB checksum mismatch: expected {expected:02x}, got {received:02x}"
            ));
        }
        self.writer
            .write_all(b"+")
            .map_err(|e| format!("GDB ack: {e}"))?;
        Ok(self.buf.clone())
    }

    fn recv_ack(&mut self) -> Result<(), String> {
        let mut b = [0u8; 1];
        self.reader
            .read_exact(&mut b)
            .map_err(|e| format!("GDB read: {e}"))?;
        match b[0] {
            b'+' => Ok(()),
            // Response arrived without ack — data stays in BufReader for recv_packet
            b'$' => Ok(()),
            _ => Err(format!("GDB expected '+', got {:#04x}", b[0])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_le_rv64() {
        assert_eq!(parse_hex_le(b"0000008000000000", 8).unwrap(), 0x80000000);
    }

    #[test]
    fn parse_hex_le_rv32() {
        assert_eq!(parse_hex_le(b"00000080", 4).unwrap(), 0x80000000);
    }

    #[test]
    fn encode_decode_round_trip() {
        for val in [0u64, 1, 0xDEADBEEF, 0x8000_0000_0000_0000] {
            let hex = encode_le_hex(val, 8);
            assert_eq!(parse_hex_le(hex.as_bytes(), 8).unwrap(), val);
        }
    }

    #[test]
    fn read_regs_parses_rv64_response() {
        let hex: Vec<u8> = (0u64..33)
            .flat_map(|i| encode_le_hex(i * 0x100, 8).into_bytes())
            .collect();
        let word_size = if hex.len() >= 33 * 16 { 8 } else { 4 };
        let regs: Vec<u64> = hex
            .chunks(word_size * 2)
            .map(|c| parse_hex_le(c, word_size).unwrap())
            .collect();
        assert_eq!(regs.len(), 33);
        assert_eq!(regs[0], 0x000);
        assert_eq!(regs[32], 0x2000);
    }

    #[test]
    fn encode_regs_hex_len() {
        let regs = vec![0u64; 33];
        let hex: String = regs.iter().map(|&r| encode_le_hex(r, 8)).collect();
        assert_eq!(hex.len(), 33 * 16);
    }

    #[test]
    fn checksum_computation() {
        assert_eq!(checksum(b"g"), b'g');
        assert_eq!(checksum(b"OK"), b'O'.wrapping_add(b'K'));
    }
}
