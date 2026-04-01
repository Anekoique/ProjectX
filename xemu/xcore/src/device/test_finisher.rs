//! SiFive Test Finisher — halt/reboot device used by OpenSBI for `shutdown`.

use super::{Device, mmio_regs};
use crate::{
    config::Word,
    error::{XError, XResult},
};

mmio_regs! { enum Reg { Finisher = 0x0000 } }

/// SiFive test finisher device.
pub struct TestFinisher;

impl TestFinisher {
    /// Create a new test finisher.
    pub fn new() -> Self {
        Self
    }
}

impl Device for TestFinisher {
    fn read(&mut self, _offset: usize, _size: usize) -> XResult<Word> {
        Ok(0)
    }

    #[allow(clippy::unnecessary_cast)]
    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        if let Some(Reg::Finisher) = Reg::decode(offset) {
            match val as u32 & 0xFFFF {
                0x5555 => return Err(XError::ProgramExit(0)),
                0x3333 => return Err(XError::ProgramExit((val as u32) >> 16)),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_exit() {
        let mut tf = TestFinisher::new();
        assert!(matches!(
            tf.write(0, 4, 0x5555),
            Err(XError::ProgramExit(0))
        ));
    }

    #[test]
    fn fail_exit_with_code() {
        let mut tf = TestFinisher::new();
        let val = (1u32 << 16) | 0x3333;
        assert!(matches!(
            tf.write(0, 4, val as Word),
            Err(XError::ProgramExit(1))
        ));
    }

    #[test]
    fn read_returns_zero() {
        let mut tf = TestFinisher::new();
        assert_eq!(tf.read(0, 4).unwrap(), 0);
    }

    #[test]
    fn unknown_value_no_exit() {
        let mut tf = TestFinisher::new();
        assert!(tf.write(0, 4, 0x1234).is_ok());
    }
}
