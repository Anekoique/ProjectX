//! Per-source configuration for the PLIC gateway.
//!
//! `SourceKind` selects the gateway FSM variant: `Level` follows the SiFive
//! pre-claim-clear variant (I-8); `Edge` latches on rising edges (I-3).

/// Trigger discipline for a single PLIC source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceKind {
    /// Level-triggered (SiFive variant): pend while line is high, clear when
    /// the line drops before claim.
    #[default]
    Level,
    /// Edge-triggered: pend on rising edge, latch during in-flight claim.
    #[allow(dead_code)] // Public API; exercised by tests and future board configs.
    Edge,
}

/// Construction-time configuration for a PLIC source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceConfig {
    pub kind: SourceKind,
}

impl SourceConfig {
    pub const fn level() -> Self {
        Self {
            kind: SourceKind::Level,
        }
    }

    #[cfg(test)]
    pub const fn edge() -> Self {
        Self {
            kind: SourceKind::Edge,
        }
    }
}
