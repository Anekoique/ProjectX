//! Recursive-descent expression evaluator for the debugger.
//!
//! Grammar:
//!
//! ```text
//!   expr    = compare
//!   compare = arith (("==" | "!=") arith)?
//!   arith   = term (('+' | '-') term)*
//!   term    = unary (('*' | '/' | '%') unary)*
//!   unary   = '*' unary | '-' unary | atom
//!   atom    = '$' NAME | "0x" HEX | DECIMAL | '(' expr ')'
//! ```
struct Parser<'a, R: Fn(&str) -> Option<u64>, M: Fn(usize, usize) -> Option<u64>> {
    input: &'a [u8],
    pos: usize,
    read_reg: &'a R,
    read_mem: &'a M,
}

impl<'a, R: Fn(&str) -> Option<u64>, M: Fn(usize, usize) -> Option<u64>> Parser<'a, R, M> {
    fn new(input: &'a str, read_reg: &'a R, read_mem: &'a M) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            read_reg,
            read_mem,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> u8 {
        let ch = self.input[self.pos];
        self.pos += 1;
        ch
    }

    fn skip_ws(&mut self) {
        self.pos += self.input[self.pos..]
            .iter()
            .take_while(|b| b.is_ascii_whitespace())
            .count();
    }

    fn expect(&mut self, ch: u8) -> Result<(), String> {
        self.skip_ws();
        if self.peek() == Some(ch) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected '{}' at pos {}", ch as char, self.pos))
        }
    }

    fn parse_expr(&mut self) -> Result<u64, String> {
        self.parse_compare()
    }

    fn parse_compare(&mut self) -> Result<u64, String> {
        let lhs = self.parse_arith()?;
        self.skip_ws();
        if self.pos + 1 < self.input.len() {
            match &self.input[self.pos..self.pos + 2] {
                b"==" => {
                    self.pos += 2;
                    return Ok(u64::from(lhs == self.parse_arith()?));
                }
                b"!=" => {
                    self.pos += 2;
                    return Ok(u64::from(lhs != self.parse_arith()?));
                }
                _ => {}
            }
        }
        Ok(lhs)
    }

    fn parse_arith(&mut self) -> Result<u64, String> {
        let mut val = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.advance();
                    val = val.wrapping_add(self.parse_term()?);
                }
                Some(b'-') => {
                    self.advance();
                    val = val.wrapping_sub(self.parse_term()?);
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_term(&mut self) -> Result<u64, String> {
        let mut val = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                // '*' in infix position is multiplication (unary '*' is handled in parse_unary)
                Some(b'*') => {
                    self.advance();
                    val = val.wrapping_mul(self.parse_unary()?);
                }
                Some(b'/') => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    if rhs == 0 {
                        return Err("division by zero".into());
                    }
                    val = val.wrapping_div(rhs);
                }
                Some(b'%') => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    if rhs == 0 {
                        return Err("modulo by zero".into());
                    }
                    val = val.wrapping_rem(rhs);
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_unary(&mut self) -> Result<u64, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'*') => {
                self.advance();
                let addr = self.parse_unary()? as usize;
                (self.read_mem)(addr, 8).ok_or_else(|| format!("cannot read memory at {addr:#x}"))
            }
            Some(b'-') => {
                self.advance();
                Ok(0u64.wrapping_sub(self.parse_unary()?))
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> Result<u64, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'$') => {
                self.advance();
                let name = self.read_name();
                (self.read_reg)(&name).ok_or_else(|| format!("unknown register: {name}"))
            }
            Some(b'(') => {
                self.advance();
                let val = self.parse_expr()?;
                self.expect(b')')?;
                Ok(val)
            }
            Some(b'0') if self.input.get(self.pos + 1) == Some(&b'x') => {
                self.pos += 2;
                self.read_hex()
            }
            Some(c) if c.is_ascii_digit() => self.read_decimal(),
            Some(c) => Err(format!("unexpected '{}' at pos {}", c as char, self.pos)),
            None => Err("unexpected end of expression".into()),
        }
    }

    /// Consume bytes while `pred` holds, return the consumed slice as `&str`.
    fn consume_while(&mut self, pred: fn(u8) -> bool) -> &str {
        let start = self.pos;
        self.pos += self.input[self.pos..]
            .iter()
            .take_while(|&&b| pred(b))
            .count();
        std::str::from_utf8(&self.input[start..self.pos]).unwrap()
    }

    fn read_name(&mut self) -> String {
        self.consume_while(|b| b.is_ascii_alphanumeric() || b == b'_')
            .to_string()
    }

    fn read_hex(&mut self) -> Result<u64, String> {
        let s = self.consume_while(|b| b.is_ascii_hexdigit());
        if s.is_empty() {
            return Err("expected hex digits after 0x".into());
        }
        u64::from_str_radix(s, 16).map_err(|e| e.to_string())
    }

    fn read_decimal(&mut self) -> Result<u64, String> {
        self.consume_while(|b| b.is_ascii_digit())
            .parse::<u64>()
            .map_err(|e| e.to_string())
    }
}

/// Evaluate expression with register and memory read callbacks.
///
/// Register references: `$name` (e.g., `$a0`, `$pc`).
/// Memory dereference: `*addr` (reads 8 bytes at physical address).
pub fn eval_expr(
    input: &str,
    read_reg: impl Fn(&str) -> Option<u64>,
    read_mem: impl Fn(usize, usize) -> Option<u64>,
) -> Result<u64, String> {
    let mut parser = Parser::new(input, &read_reg, &read_mem);
    let val = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        Err(format!("trailing characters at pos {}", parser.pos))
    } else {
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_reg(_: &str) -> Option<u64> {
        None
    }
    fn no_mem(_: usize, _: usize) -> Option<u64> {
        None
    }

    #[test]
    fn literals() {
        assert_eq!(eval_expr("42", no_reg, no_mem).unwrap(), 42);
        assert_eq!(eval_expr("0xff", no_reg, no_mem).unwrap(), 255);
        assert_eq!(eval_expr("0x80000000", no_reg, no_mem).unwrap(), 0x80000000);
    }

    #[test]
    fn arithmetic() {
        assert_eq!(eval_expr("2 + 3", no_reg, no_mem).unwrap(), 5);
        assert_eq!(eval_expr("10 - 3", no_reg, no_mem).unwrap(), 7);
        assert_eq!(eval_expr("4 + 3 * 2", no_reg, no_mem).unwrap(), 10);
        assert_eq!(eval_expr("(4 + 3) * 2", no_reg, no_mem).unwrap(), 14);
    }

    #[test]
    fn register_in_expression() {
        let read_reg = |name: &str| -> Option<u64> {
            match name {
                "a0" => Some(100),
                "pc" => Some(0x80000000),
                "a1" => Some(200),
                _ => None,
            }
        };
        assert_eq!(eval_expr("$a0", read_reg, no_mem).unwrap(), 100);
        assert_eq!(eval_expr("$a0 + 1", read_reg, no_mem).unwrap(), 101);
        assert_eq!(eval_expr("$a0 + $a1", read_reg, no_mem).unwrap(), 300);
        assert_eq!(
            eval_expr("$pc + 4 * 2", read_reg, no_mem).unwrap(),
            0x80000008
        );
        assert_eq!(eval_expr("$a0 == $a1", read_reg, no_mem).unwrap(), 0);
        assert_eq!(eval_expr("$a0 == 100", read_reg, no_mem).unwrap(), 1);
        assert!(eval_expr("$unknown", read_reg, no_mem).is_err());
    }

    #[test]
    fn memory_deref() {
        let read_mem = |addr: usize, _: usize| -> Option<u64> {
            if addr == 0x1000 { Some(0xDEAD) } else { None }
        };
        assert_eq!(eval_expr("*0x1000", no_reg, read_mem).unwrap(), 0xDEAD);
        assert!(eval_expr("*0x2000", no_reg, read_mem).is_err());
        // Deref + arithmetic
        assert_eq!(eval_expr("*0x1000 + 1", no_reg, read_mem).unwrap(), 0xDEAE);
    }

    #[test]
    fn compare() {
        assert_eq!(eval_expr("5 == 5", no_reg, no_mem).unwrap(), 1);
        assert_eq!(eval_expr("5 != 5", no_reg, no_mem).unwrap(), 0);
        assert_eq!(eval_expr("5 != 3", no_reg, no_mem).unwrap(), 1);
    }

    #[test]
    fn errors() {
        assert!(eval_expr("", no_reg, no_mem).is_err());
        assert!(eval_expr("5 / 0", no_reg, no_mem).is_err());
        assert!(eval_expr("abc", no_reg, no_mem).is_err());
    }
}
