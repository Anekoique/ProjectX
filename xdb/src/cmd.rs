use xcore::XCPU;

pub fn cmd_continue() {
    cmd_step(u32::MAX)
}

pub fn cmd_step(count: u32) {
    XCPU.lock()
        .map_err(|e| {
            panic!("Failed to lock CPU mutex: {}", e);
        })
        .and_then(|mut cpu| cpu.run(count))
        .unwrap_or_else(|e| eprintln!("Error: {}", e));
}

pub fn cmd_load(file: String) {
    XCPU.lock()
        .map_err(|e| {
            panic!("Failed to lock CPU mutex: {}", e);
        })
        .and_then(|mut cpu| cpu.load(file))
        .unwrap_or_else(|e| eprintln!("Error: {}", e));
}
