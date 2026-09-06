//! Reusable wheel-definition vocabulary.
//!
//! The `wheel_core` crate is intentionally supplied through a Vapor semantic
//! dependency rather than being manually authored into this package's initial
//! Cargo manifest.

use wheel_core::{OutcomeId, WheelId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelDefinition {
    id: WheelId,
    outcomes: Vec<OutcomeId>,
}

impl WheelDefinition {
    pub fn new(
        id: WheelId,
        outcomes: impl IntoIterator<Item = OutcomeId>,
    ) -> Self {
        Self {
            id,
            outcomes: outcomes.into_iter().collect(),
        }
    }

    pub fn id(&self) -> &WheelId {
        &self.id
    }

    pub fn outcomes(&self) -> &[OutcomeId] {
        &self.outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_registered_outcomes() {
        let wheel = WheelDefinition::new(
            WheelId::new("classic"),
            [
                OutcomeId::new("coin"),
                OutcomeId::new("diamond"),
                OutcomeId::new("jackpot"),
            ],
        );

        assert_eq!(wheel.id().as_str(), "classic");
        assert_eq!(wheel.outcomes().len(), 3);
        assert_eq!(wheel.outcomes()[2].as_str(), "jackpot");
    }
}