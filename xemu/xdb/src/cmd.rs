use xcore::{XResult, with_xcpu};

use crate::{expr::eval_expr, watchpoint::WatchManager};

// ── Execution ──

pub fn cmd_step(count: u64, watch_mgr: &mut WatchManager) -> XResult {
    for _ in 0..count {
        let done = with_xcpu(|cpu| -> XResult<bool> {
            cpu.step()?;
            Ok(cpu.is_terminated())
        })?;
        if done {
            break;
        }
        if let Some(hit) = check_watchpoints(watch_mgr) {
            println!("{hit}");
            return Ok(());
        }
    }
    Ok(())
}

pub fn cmd_continue(watch_mgr: &mut WatchManager) -> XResult {
    if watch_mgr.is_empty() {
        return with_xcpu(|cpu| cpu.run(u64::MAX));
    }
    loop {
        let done = with_xcpu(|cpu| -> XResult<bool> {
            cpu.step()?;
            Ok(cpu.is_terminated())
        })?;
        if done {
            break;
        }
        if let Some(hit) = check_watchpoints(watch_mgr) {
            println!("{hit}");
            return Ok(());
        }
    }
    Ok(())
}

fn check_watchpoints(watch_mgr: &mut WatchManager) -> Option<String> {
    with_xcpu(|cpu| {
        let ops = cpu.debug_ops();
        watch_mgr.check(|expr| {
            eval_expr(
                expr,
                |name| ops.read_register(name),
                |addr, sz| ops.read_memory(addr, sz).ok(),
            )
        })
    })
}

// ── Breakpoints ──

pub fn cmd_break(addr_str: &str) -> XResult {
    let addr = parse_addr(addr_str).map_err(|_| xcore::XError::BadAddress)?;
    let id = with_xcpu(|cpu| cpu.add_breakpoint(addr));
    println!("Breakpoint #{id} at {addr:#x}");
    Ok(())
}

pub fn cmd_break_delete(id: u32) -> XResult {
    let ok = with_xcpu(|cpu| cpu.remove_breakpoint(id));
    println!(
        "{}",
        if ok {
            format!("Deleted breakpoint #{id}")
        } else {
            format!("No breakpoint #{id}")
        }
    );
    Ok(())
}

pub fn cmd_break_list() -> XResult {
    with_xcpu(|cpu| {
        let bps = cpu.list_breakpoints();
        if bps.is_empty() {
            println!("No breakpoints.");
        } else {
            for bp in bps {
                println!("  #{}: {:#x}", bp.id, bp.addr);
            }
        }
    });
    Ok(())
}

// ── Examine (disassembly / memory) ──

/// All addresses are physical. Default base = current pc value.
/// In bare-metal (identity-mapped) execution, pc value == physical address.
pub fn cmd_examine(format: char, count: usize, addr: Option<String>) -> XResult {
    with_xcpu(|cpu| {
        let base = match addr {
            Some(ref s) => parse_addr(s).map_err(|_| xcore::XError::BadAddress)?,
            None => cpu.pc(), // physical address (identity-mapped in bare-metal)
        };
        let ops = cpu.debug_ops();
        match format {
            'i' => examine_inst(ops, base, count),
            'x' => examine_hex(ops, base, count),
            'b' => examine_bytes(ops, base, count),
            _ => println!("Unknown format '{format}'. Use: i(nstr), x(hex), b(yte)"),
        }
        Ok(())
    })
}

fn examine_inst(ops: &dyn xcore::DebugOps, mut pc: usize, count: usize) {
    for _ in 0..count {
        match ops.fetch_inst(pc) {
            Ok(raw) => {
                let mn = ops.disasm_raw(raw);
                let width = if raw & 0x3 != 0x3 { 2 } else { 4 };
                println!("  {pc:#010x}: {raw:08x}  {mn}");
                pc += width;
            }
            Err(_) => {
                println!("  {pc:#010x}: <not in RAM>");
                break;
            }
        }
    }
}

fn examine_hex(ops: &dyn xcore::DebugOps, base: usize, count: usize) {
    for i in 0..count {
        let a = base + i * 8;
        match ops.read_memory(a, 8) {
            Ok(val) => println!("  {a:#010x}: {val:#018x}"),
            Err(_) => {
                println!("  {a:#010x}: <not in RAM>");
                break;
            }
        }
    }
}

fn examine_bytes(ops: &dyn xcore::DebugOps, base: usize, count: usize) {
    print!("  {base:#010x}: ");
    for i in 0..count {
        match ops.read_memory(base + i, 1) {
            Ok(val) => print!("{val:02x} "),
            Err(_) => {
                print!("?? ");
                break;
            }
        }
    }
    println!();
}

// ── Print expression ──

pub fn cmd_print(expr_str: &str) -> XResult {
    with_xcpu(|cpu| {
        let ops = cpu.debug_ops();
        match eval_expr(
            expr_str,
            |name| ops.read_register(name),
            |addr, sz| ops.read_memory(addr, sz).ok(),
        ) {
            Ok(val) => println!("{val:#x} ({val})"),
            Err(e) => println!("Error: {e}"),
        }
        Ok(())
    })
}

// ── Info ──

pub fn cmd_info(what: &str, name: Option<&str>) -> XResult {
    match what {
        "reg" | "r" => with_xcpu(|cpu| {
            let ops = cpu.debug_ops();
            match name {
                Some(n) => match ops.read_register(n) {
                    Some(val) => println!("{n} = {val:#x}"),
                    None => println!("Unknown register: {n}"),
                },
                None => {
                    for (i, (name, val)) in ops.dump_registers().iter().enumerate() {
                        print!("{name:>10} = {val:#018x}");
                        if (i + 1) % 4 == 0 {
                            println!();
                        } else {
                            print!("  ");
                        }
                    }
                    println!();
                }
            }
        }),
        _ => println!("Unknown info target: {what}. Try: reg"),
    }
    Ok(())
}

// ── Watchpoints ──

pub fn cmd_watch(expr_str: &str, watch_mgr: &mut WatchManager) -> XResult {
    // Validate expression before creating watchpoint — reject syntax errors
    let result = with_xcpu(|cpu| {
        let ops = cpu.debug_ops();
        eval_expr(
            expr_str,
            |name| ops.read_register(name),
            |addr, sz| ops.read_memory(addr, sz).ok(),
        )
    });
    match result {
        Ok(val) => {
            let id = watch_mgr.add(expr_str.to_string(), Some(val));
            println!("Watchpoint #{id}: {expr_str}");
            println!("  initial value: {val:#x}");
        }
        Err(e) => {
            // Reject invalid expressions (syntax errors, unknown registers)
            println!("Error: {e}");
            return Ok(());
        }
    }
    Ok(())
}

pub fn cmd_watch_list(watch_mgr: &WatchManager) {
    let wps = watch_mgr.list();
    if wps.is_empty() {
        println!("No watchpoints.");
    } else {
        for wp in wps {
            let val = wp
                .prev_value
                .map(|v| format!("{v:#x}"))
                .unwrap_or_else(|| "???".to_string());
            println!("  #{}: {} = {}", wp.id, wp.expr_text, val);
        }
    }
}

// ── Existing ──

pub fn cmd_load(file: String) -> XResult {
    with_xcpu!(load(Some(file)).map(|_| ()))
}

pub fn cmd_reset() -> XResult {
    with_xcpu!(reset())
}

// ── Helpers ──

fn parse_addr(s: &str) -> Result<usize, String> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    usize::from_str_radix(s, 16).map_err(|e| e.to_string())
}
