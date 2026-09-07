//! Reusable wheel-domain rules and vocabulary.
//!
//! `wheel-core` owns the primitive semantic identifiers.
//! This Library builds reusable wheel/loot structures on top of them.
//!
//! Neither crate owns Game runtime behavior. The Game decides how these
//! structures participate in actual gameplay.

pub use wheel_core::{GameModeId, LootTableId, OutcomeId, WheelId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelOutcome {
    id: OutcomeId,
    text: String,
}

impl WheelOutcome {
    pub fn new(id: OutcomeId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
        }
    }

    pub fn id(&self) -> &OutcomeId {
        &self.id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTable {
    id: LootTableId,
    outcomes: Vec<WheelOutcome>,
}

impl LootTable {
    pub fn new(id: LootTableId, outcomes: impl IntoIterator<Item = WheelOutcome>) -> Self {
        Self {
            id,
            outcomes: outcomes.into_iter().collect(),
        }
    }

    pub fn id(&self) -> &LootTableId {
        &self.id
    }

    pub fn outcomes(&self) -> &[WheelOutcome] {
        &self.outcomes
    }

    pub fn outcome(&self, id: &OutcomeId) -> Option<&WheelOutcome> {
        self.outcomes.iter().find(|outcome| outcome.id() == id)
    }

    pub fn add_outcome(&mut self, outcome: WheelOutcome) {
        self.outcomes.push(outcome);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelDefinition {
    id: WheelId,
    loot_table: LootTableId,
}

impl WheelDefinition {
    pub fn new(id: WheelId, loot_table: LootTableId) -> Self {
        Self { id, loot_table }
    }

    pub fn id(&self) -> &WheelId {
        &self.id
    }

    pub fn loot_table(&self) -> &LootTableId {
        &self.loot_table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_table_preserves_and_extends_outcomes() {
        let mut table = LootTable::new(
            LootTableId::new("starter"),
            [
                WheelOutcome::new(OutcomeId::new("coin"), "A coin."),
                WheelOutcome::new(OutcomeId::new("diamond"), "A diamond."),
            ],
        );

        table.add_outcome(WheelOutcome::new(OutcomeId::new("jackpot"), "Jackpot!"));

        assert_eq!(table.id().as_str(), "starter");
        assert_eq!(table.outcomes().len(), 3);

        assert_eq!(
            table.outcome(&OutcomeId::new("jackpot")).unwrap().text(),
            "Jackpot!"
        );
    }

    #[test]
    fn wheel_points_at_loot_table() {
        let wheel = WheelDefinition::new(WheelId::new("classic"), LootTableId::new("starter"));

        assert_eq!(wheel.id().as_str(), "classic");
        assert_eq!(wheel.loot_table().as_str(), "starter");
    }
}
