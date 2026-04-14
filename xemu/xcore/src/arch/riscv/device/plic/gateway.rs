//! Per-source interrupt gateway FSM.
//!
//! Converts raw line-level samples into core pend/clear decisions, gated by
//! the current claim in-flight state. Level-triggered with the SiFive
//! pre-claim-clear variant (plicGateway I-8) — the only triggering
//! discipline needed by the in-tree device set.

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

/// Per-source gateway state. All fields are runtime state cleared on reset.
pub(super) struct Gateway {
    armed: bool,
    in_flight: bool,
}

impl Gateway {
    pub(super) fn new() -> Self {
        Self {
            armed: false,
            in_flight: false,
        }
    }

    /// Sample the device line level. Returns the core-pending decision.
    pub(super) fn sample(&mut self, level: bool) -> GatewayDecision {
        if self.in_flight {
            // Track latest level so `on_complete` re-pends iff the line is
            // still high at complete.
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

    /// Core selected this source for a claim. Enter in-flight; `sample` stops
    /// generating decisions until `on_complete` closes the cycle.
    pub(super) fn on_claim(&mut self) {
        self.in_flight = true;
    }

    /// Guest wrote the complete register. Returns `Pend` iff the line was
    /// still high when the cycle closed.
    pub(super) fn on_complete(&mut self) -> GatewayDecision {
        self.in_flight = false;
        if self.armed {
            GatewayDecision::Pend
        } else {
            GatewayDecision::NoChange
        }
    }

    /// Clear runtime state.
    pub(super) fn reset_runtime(&mut self) {
        self.armed = false;
        self.in_flight = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raise_pends_then_line_low_clears() {
        let mut g = Gateway::new();
        assert_eq!(g.sample(true), GatewayDecision::Pend);
        assert_eq!(g.sample(false), GatewayDecision::Clear);
    }

    #[test]
    fn claim_gates_repend() {
        let mut g = Gateway::new();
        g.sample(true);
        g.on_claim();
        assert_eq!(g.sample(true), GatewayDecision::NoChange);
    }

    #[test]
    fn complete_with_line_high_repends() {
        let mut g = Gateway::new();
        g.sample(true);
        g.on_claim();
        g.sample(true);
        assert_eq!(g.on_complete(), GatewayDecision::Pend);
    }

    #[test]
    fn complete_with_line_low_no_repend() {
        let mut g = Gateway::new();
        g.sample(true);
        g.on_claim();
        g.sample(false);
        assert_eq!(g.on_complete(), GatewayDecision::NoChange);
    }

    #[test]
    fn reset_runtime_clears_state() {
        let mut g = Gateway::new();
        g.sample(true);
        g.on_claim();
        g.reset_runtime();
        assert!(!g.armed && !g.in_flight);
    }
}
