#[cfg(feature = "difftest")]
use crate::difftest::DiffHarness;
use crate::watchpoint::WatchManager;

/// Mutable debug session state — owns watchpoints and difftest harness.
/// Passed as a single `&mut Session` instead of threading individual params.
pub struct Session {
    pub watch: WatchManager,
    #[cfg(feature = "difftest")]
    pub loaded_path: Option<String>,
    #[cfg(feature = "difftest")]
    pub diff: Option<DiffHarness>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            watch: WatchManager::new(),
            #[cfg(feature = "difftest")]
            loaded_path: std::env::var("X_FILE").ok().filter(|s| !s.is_empty()),
            #[cfg(feature = "difftest")]
            diff: None,
        }
    }

    /// Are there active hooks that require per-step checking?
    pub fn has_hooks(&self) -> bool {
        let active = !self.watch.is_empty();
        #[cfg(feature = "difftest")]
        let active = active || self.diff.is_some();
        active
    }
}
