//! GDB Remote Serial Protocol client over TCP.

use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct GdbClient {
    stream: TcpStream,
    buf: Vec<u8>,
    pending: Option<u8>, // pushback byte for protocol alignment
}

impl GdbClient {
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr).map_err(|e| format!("GDB connect to {addr}: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| format!("set timeout: {e}"))?;
        let mut client = Self {
            stream,
            buf: Vec::with_capacity(4096),
            pending: None,
        };
        // Consume initial handshake byte(s)
        let _ = client.read_byte();
        Ok(client)
    }

    /// Send a GDB packet and receive the response payload.
    pub fn send_recv(&mut self, cmd: &str) -> Result<Vec<u8>, String> {
        self.send_packet(cmd)?;
        self.recv_packet()
    }

    // ── High-level commands ──

    pub fn step(&mut self) -> Result<(), String> {
        self.send_recv("vCont;s:p1.-1").map(|_| ())
    }

    pub fn cont(&mut self) -> Result<(), String> {
        self.send_recv("vCont;c:p1.-1").map(|_| ())
    }

    pub fn read_regs(&mut self) -> Result<Vec<u64>, String> {
        let data = self.send_recv("g")?;
        parse_gdb_regs(&data)
    }

    pub fn write_regs(&mut self, regs: &[u64], word_size: usize) -> Result<(), String> {
        let hex = encode_regs_hex(regs, word_size);
        let resp = self.send_recv(&format!("G{hex}"))?;
        expect_ok(&resp)
    }

    pub fn read_register(&mut self, num: usize, word_size: usize) -> Result<u64, String> {
        let data = self.send_recv(&format!("p{num:x}"))?;
        parse_hex_le(&data, word_size)
    }

    pub fn write_register(&mut self, num: usize, val: u64, word_size: usize) -> Result<(), String> {
        let hex = encode_le_hex(val, word_size);
        let resp = self.send_recv(&format!("P{num:x}={hex}"))?;
        expect_ok(&resp)
    }

    pub fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        let resp = self.send_recv(&format!("M{addr:x},{:x}:{hex}", data.len()))?;
        expect_ok(&resp)
    }

    pub fn set_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        let resp = self.send_recv(&format!("Z0,{addr:x},4"))?;
        expect_ok(&resp)
    }

    pub fn remove_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        let resp = self.send_recv(&format!("z0,{addr:x},4"))?;
        expect_ok(&resp)
    }

    // ── Packet framing ──

    fn send_packet(&mut self, data: &str) -> Result<(), String> {
        let cksum: u8 = data.bytes().fold(0u8, |a, b| a.wrapping_add(b));
        let pkt = format!("${data}#{cksum:02x}");
        self.stream
            .write_all(pkt.as_bytes())
            .map_err(|e| format!("GDB send: {e}"))?;
        self.stream.flush().map_err(|e| format!("GDB flush: {e}"))?;
        self.recv_ack()
    }

    fn recv_packet(&mut self) -> Result<Vec<u8>, String> {
        // Skip until '$'
        loop {
            let b = self.read_byte()?;
            if b == b'$' {
                break;
            }
        }
        // Read until '#'
        self.buf.clear();
        loop {
            let b = self.read_byte()?;
            if b == b'#' {
                break;
            }
            self.buf.push(b);
        }
        // Read 2 checksum hex chars
        let hi = self.read_byte()?;
        let lo = self.read_byte()?;

        // Validate checksum
        let expected = self.buf.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        let cksum_bytes = [hi, lo];
        let cksum_str = std::str::from_utf8(&cksum_bytes)
            .map_err(|_| format!("GDB checksum: invalid UTF-8 ({hi:#04x} {lo:#04x})"))?;
        let received = u8::from_str_radix(cksum_str, 16)
            .map_err(|_| format!("GDB checksum: invalid hex '{cksum_str}'"))?;
        if expected != received {
            return Err(format!(
                "GDB checksum mismatch: expected {expected:02x}, got {received:02x}"
            ));
        }
        // Send ACK
        self.stream
            .write_all(b"+")
            .map_err(|e| format!("GDB ack: {e}"))?;
        Ok(self.buf.clone())
    }

    fn recv_ack(&mut self) -> Result<(), String> {
        let b = self.read_byte()?;
        match b {
            b'+' => Ok(()),
            b'$' => {
                // Response arrived without ack — push back for recv_packet
                self.pending = Some(b'$');
                Ok(())
            }
            _ => Err(format!("GDB expected '+', got {:#04x}", b)),
        }
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if let Some(b) = self.pending.take() {
            return Ok(b);
        }
        let mut byte = [0u8];
        self.stream
            .read_exact(&mut byte)
            .map_err(|e| format!("GDB read: {e}"))?;
        Ok(byte[0])
    }
}

// ── Hex helpers ──

fn parse_gdb_regs(hex: &[u8]) -> Result<Vec<u64>, String> {
    // QEMU riscv: GPR[0..31] + PC = 33 registers
    let word_size = if hex.len() >= 33 * 16 { 8 } else { 4 };
    let chunk = word_size * 2;
    let n_regs = hex.len() / chunk;
    (0..n_regs)
        .map(|i| {
            let start = i * chunk;
            let end = start + chunk;
            if end > hex.len() {
                return Err("GDB register response too short".into());
            }
            parse_hex_le(&hex[start..end], word_size)
        })
        .collect()
}

fn parse_hex_le(hex: &[u8], word_size: usize) -> Result<u64, String> {
    let len = word_size * 2;
    if hex.len() < len {
        return Err(format!(
            "hex too short: expected {} chars, got {} (data: {:?})",
            len,
            hex.len(),
            String::from_utf8_lossy(hex)
        ));
    }
    let hex = &hex[..len];
    let mut val = 0u64;
    for i in (0..hex.len()).step_by(2) {
        let s = std::str::from_utf8(&hex[i..i + 2]).map_err(|e| e.to_string())?;
        let byte = u8::from_str_radix(s, 16).map_err(|e| e.to_string())?;
        val |= (byte as u64) << (i / 2 * 8);
    }
    Ok(val)
}

fn encode_le_hex(val: u64, word_size: usize) -> String {
    (0..word_size)
        .map(|i| format!("{:02x}", (val >> (i * 8)) & 0xFF))
        .collect()
}

fn encode_regs_hex(regs: &[u64], word_size: usize) -> String {
    regs.iter().map(|&r| encode_le_hex(r, word_size)).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_le_rv64() {
        // 0x0000000080000000 in little-endian hex
        let hex = b"0000008000000000";
        assert_eq!(parse_hex_le(hex, 8).unwrap(), 0x80000000);
    }

    #[test]
    fn parse_hex_le_rv32() {
        let hex = b"00000080";
        assert_eq!(parse_hex_le(hex, 4).unwrap(), 0x80000000);
    }

    #[test]
    fn encode_decode_round_trip() {
        for val in [0u64, 1, 0xDEADBEEF, 0x8000_0000_0000_0000] {
            let hex = encode_le_hex(val, 8);
            let decoded = parse_hex_le(hex.as_bytes(), 8).unwrap();
            assert_eq!(decoded, val);
        }
    }

    #[test]
    fn parse_regs_rv64() {
        // 33 registers * 16 hex chars each
        let mut hex = Vec::new();
        for i in 0u64..33 {
            hex.extend_from_slice(encode_le_hex(i * 0x100, 8).as_bytes());
        }
        let regs = parse_gdb_regs(&hex).unwrap();
        assert_eq!(regs.len(), 33);
        assert_eq!(regs[0], 0x000);
        assert_eq!(regs[32], 0x2000); // PC
    }

    #[test]
    fn encode_regs_hex_len() {
        let regs = vec![0u64; 33];
        let hex = encode_regs_hex(&regs, 8);
        assert_eq!(hex.len(), 33 * 16);
    }
}
