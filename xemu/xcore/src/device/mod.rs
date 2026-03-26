pub mod bus;
pub mod ram;

use crate::{config::Word, error::XResult};

pub trait Device: Send {
    fn read(&self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
}
