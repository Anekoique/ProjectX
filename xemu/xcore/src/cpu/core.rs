use std::sync::{Arc, Mutex};

use memory_addr::VirtAddr;

use crate::{config::Word, device::bus::Bus, error::XResult};

pub trait CoreOps {
    fn pc(&self) -> VirtAddr;
    fn bus(&self) -> &Arc<Mutex<Bus>>;
    fn reset(&mut self) -> XResult;
    fn step(&mut self) -> XResult;
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;
}
