pub struct Watchpoint {
    pub id: u32,
    pub expr_text: String,
    pub prev_value: Option<u64>,
}

pub struct WatchManager {
    wps: Vec<Watchpoint>,
    next_id: u32,
}

impl WatchManager {
    pub fn new() -> Self {
        Self {
            wps: Vec::new(),
            next_id: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.wps.is_empty()
    }

    pub fn add(&mut self, expr: String, init: Option<u64>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.wps.push(Watchpoint {
            id,
            expr_text: expr,
            prev_value: init,
        });
        id
    }

    pub fn remove(&mut self, id: u32) -> bool {
        self.wps
            .iter()
            .position(|w| w.id == id)
            .map(|pos| self.wps.remove(pos))
            .is_some()
    }

    pub fn list(&self) -> &[Watchpoint] {
        &self.wps
    }

    /// Check all watchpoints against current state.
    /// `eval` evaluates an expression and returns Ok(value) or Err.
    /// Only triggers on actual value changes; eval errors are non-triggers.
    pub fn check(&mut self, eval: impl Fn(&str) -> Result<u64, String>) -> Option<String> {
        for wp in &mut self.wps {
            match eval(&wp.expr_text) {
                Ok(new_val) => {
                    let changed = match wp.prev_value {
                        Some(old) => old != new_val,
                        None => false, // first successful eval — record, don't trigger
                    };
                    if changed {
                        let old = wp.prev_value.unwrap_or(0);
                        wp.prev_value = Some(new_val);
                        return Some(format!(
                            "Watchpoint #{}: {} changed {old:#x} → {new_val:#x}",
                            wp.id, wp.expr_text
                        ));
                    }
                    wp.prev_value = Some(new_val); // always update
                }
                Err(_) => {
                    // Eval error is a non-trigger — don't fire the watchpoint.
                    // This handles temporarily unreadable expressions
                    // gracefully.
                }
            }
        }
        None
    }
}
