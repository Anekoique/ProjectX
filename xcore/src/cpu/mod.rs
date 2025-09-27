pub struct CPU {
    pc: u32,
}

impl CPU {
    pub fn new() -> Self {
        CPU { pc: 0 }
    }
}