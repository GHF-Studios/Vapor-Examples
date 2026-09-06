//! Reusable wheel-rule vocabulary for Hello World on Steroids.
//!
//! This remains a Vapor Library: it provides reusable Rust implementation/API
//! without owning Engine/Game/Mod runtime-composition semantics.

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

/// A collection of outcomes which a Wheel may resolve against.
///
/// Selection policy deliberately does not live here.
///
/// Which outcome is chosen is Game/runtime behavior and may later be extended
/// by Mods or capabilities supplied by other Content.
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

    pub fn add_outcome(&mut self, outcome: WheelOutcome) -> bool {
        if self.outcome(outcome.id()).is_some() {
            return false;
        }

        self.outcomes.push(outcome);

        true
    }
}

/// A Wheel identifies the LootTable against which it operates.
///
/// This deliberately does not encode a selection algorithm.
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
    fn loot_table_rejects_duplicate_outcome_ids() {
        let mut table = LootTable::new(
            LootTableId::new("starter"),
            [WheelOutcome::new(OutcomeId::new("hello"), "Hello")],
        );

        assert!(!table.add_outcome(WheelOutcome::new(
            OutcomeId::new("hello"),
            "Different Hello",
        )));

        assert_eq!(table.outcomes().len(), 1);
    }

    #[test]
    fn wheel_references_its_loot_table() {
        let wheel = WheelDefinition::new(WheelId::new("classic"), LootTableId::new("starter"));

        assert_eq!(wheel.id().as_str(), "classic");
        assert_eq!(wheel.loot_table().as_str(), "starter");
    }
}
