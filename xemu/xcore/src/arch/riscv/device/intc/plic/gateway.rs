//! Per-source interrupt gateway FSM.
//!
//! Maps a raw line level (Level kind) or rising-edge event (Edge kind, Phase
//! 2) into core pend/clear decisions, gated by the current claim in-flight
//! state. See `docs/fix/plicGateway/01_PLAN.md:576-600` for the transition
//! tables.

use super::source::{SourceConfig, SourceKind};

/// Emitted by [`Gateway::sample`] / [`Gateway::on_complete`] to instruct the
/// PLIC core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GatewayDecision {
    /// Set the core's pending bit for this source.
    Pend,
    /// Clear the core's pending bit for this source (SiFive pre-claim-clear).
    Clear,
    /// No change to the core's pending bit.
    NoChange,
}

/// Per-source gateway state.
///
/// `kind` is construction-time (I-11: preserved across `reset`). `armed`,
/// `in_flight`, `prev_level` are runtime state cleared on reset. `prev_level`
/// is only read by the Edge FSM.
pub(super) struct Gateway {
    kind: SourceKind,
    armed: bool,
    in_flight: bool,
    prev_level: bool,
}

impl Gateway {
    pub(super) fn new(cfg: SourceConfig) -> Self {
        Self {
            kind: cfg.kind,
            armed: false,
            in_flight: false,
            prev_level: false,
        }
    }

    /// Device-line sample. Returns the core-pending decision.
    pub(super) fn sample(&mut self, level: bool) -> GatewayDecision {
        match self.kind {
            SourceKind::Level => self.sample_level(level),
            SourceKind::Edge => self.sample_edge(level),
        }
    }

    fn sample_level(&mut self, level: bool) -> GatewayDecision {
        if self.in_flight {
            // Overwrite, not OR: `armed` tracks the latest line level so
            // `on_complete` re-pends iff the line is still high at complete.
            self.armed = level;
            return GatewayDecision::NoChange;
        }
        match (self.armed, level) {
            (false, true) => {
                self.armed = true;
                GatewayDecision::Pend
            }
            (true, false) => {
                self.armed = false;
                GatewayDecision::Clear
            }
            _ => GatewayDecision::NoChange,
        }
    }

    fn sample_edge(&mut self, level: bool) -> GatewayDecision {
        let rising = level && !self.prev_level;
        self.prev_level = level;
        if !rising {
            return GatewayDecision::NoChange;
        }
        // Rising edge: arm. Pend only if this is the first rising edge and no
        // claim is in-flight; otherwise the latch coalesces silently.
        let suppress = self.in_flight || self.armed;
        self.armed = true;
        if suppress {
            GatewayDecision::NoChange
        } else {
            GatewayDecision::Pend
        }
    }

    /// Core selected this source for a claim. Enter in-flight; sampling stops
    /// generating decisions until `on_complete` closes the cycle. Edge arms a
    /// fresh latch for the next rising-edge during in-flight.
    pub(super) fn on_claim(&mut self) {
        self.in_flight = true;
        if matches!(self.kind, SourceKind::Edge) {
            self.armed = false;
        }
    }

    /// Guest wrote the complete register. Returns `Pend` iff the source
    /// re-pends (level stayed high, or edge latched during in-flight).
    pub(super) fn on_complete(&mut self) -> GatewayDecision {
        self.in_flight = false;
        if self.armed {
            GatewayDecision::Pend
        } else {
            GatewayDecision::NoChange
        }
    }

    /// Clear runtime state; preserve `kind` per I-11.
    pub(super) fn reset_runtime(&mut self) {
        self.armed = false;
        self.in_flight = false;
        self.prev_level = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level() -> Gateway {
        Gateway::new(SourceConfig::level())
    }

    fn edge() -> Gateway {
        Gateway::new(SourceConfig::edge())
    }

    #[test]
    fn level_raise_pends_then_line_low_clears() {
        let mut g = level();
        assert_eq!(g.sample(true), GatewayDecision::Pend);
        assert_eq!(g.sample(false), GatewayDecision::Clear);
    }

    #[test]
    fn level_claim_gates_repend() {
        let mut g = level();
        g.sample(true);
        g.on_claim();
        assert_eq!(g.sample(true), GatewayDecision::NoChange);
    }

    #[test]
    fn level_complete_with_line_high_repends() {
        let mut g = level();
        g.sample(true);
        g.on_claim();
        g.sample(true);
        assert_eq!(g.on_complete(), GatewayDecision::Pend);
    }

    #[test]
    fn level_complete_with_line_low_no_repend() {
        let mut g = level();
        g.sample(true);
        g.on_claim();
        g.sample(false);
        assert_eq!(g.on_complete(), GatewayDecision::NoChange);
    }

    #[test]
    fn reset_runtime_preserves_kind() {
        let mut g = level();
        g.sample(true);
        g.on_claim();
        g.reset_runtime();
        assert!(!g.armed && !g.in_flight);
        assert_eq!(g.kind, SourceKind::Level);
    }

    #[test]
    fn edge_rising_pends_coalesces_and_latches_during_in_flight() {
        let mut g = edge();
        assert_eq!(g.sample(true), GatewayDecision::Pend);
        assert_eq!(g.sample(true), GatewayDecision::NoChange); // coalesce
        g.on_claim();
        assert_eq!(g.sample(false), GatewayDecision::NoChange); // drop line
        assert_eq!(g.sample(true), GatewayDecision::NoChange); // latch rising
        assert_eq!(g.on_complete(), GatewayDecision::Pend);
    }

    #[test]
    fn edge_complete_with_no_latched_edge_no_repend() {
        let mut g = edge();
        g.sample(true);
        g.on_claim();
        assert_eq!(g.on_complete(), GatewayDecision::NoChange);
    }

    #[test]
    fn edge_sample_false_never_clears_latched_arm() {
        let mut g = edge();
        g.sample(true);
        g.on_claim();
        g.sample(false);
        g.sample(true);
        g.sample(false);
        assert_eq!(g.on_complete(), GatewayDecision::Pend);
    }

    #[test]
    fn edge_reset_preserves_kind() {
        let mut g = edge();
        g.sample(true);
        g.on_claim();
        g.reset_runtime();
        assert!(!g.armed && !g.in_flight && !g.prev_level);
        assert_eq!(g.kind, SourceKind::Edge);
    }
}
