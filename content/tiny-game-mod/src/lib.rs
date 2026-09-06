//! Tiny Game Mod used by Vapor's evolving Hello World on Steroids techdemo.
//!
//! The Mod now demonstrates three separate Game-owned extension mechanisms:
//!
//! - attaching a system to a Game-defined registration phase;
//! - extending a Game-owned LootTable through ordinary Rust API;
//! - registering and selecting an entirely new GameMode.

use bevy_ecs::{prelude::ResMut, schedule::IntoScheduleConfigs};
use hello_world_game::{
    ActiveGameMode, App, GameModeDefinition, GameModeId, GameModeRegistry,
    HELLO_WORLD_LOOT_TABLE_ID, HELLO_WORLD_WHEEL_ID, LootTableId, LootTableRegistry, OutcomeId,
    SpinStrategy, Startup, WheelGameSet, WheelId, WheelOutcome,
};

const STEROIDS_MODE_ID: &str = "on-steroids";

pub fn install(app: &mut App) {
    app.add_systems(Startup, register_steroids.in_set(WheelGameSet::Register));
}

/// Game-Mod-owned selection behavior.
///
/// This demonstrates that downstream Content may implement the Game's public
/// extension contract without Vapor knowing anything about that contract.
struct LastOutcomeSpin;

impl SpinStrategy for LastOutcomeSpin {
    fn select(&self, outcomes: &[WheelOutcome], _spin_index: u64) -> Option<usize> {
        outcomes.len().checked_sub(1)
    }
}

fn register_steroids(
    mut loot_tables: ResMut<LootTableRegistry>,
    mut modes: ResMut<GameModeRegistry>,
    mut active_mode: ResMut<ActiveGameMode>,
) {
    let table_id = LootTableId::new(HELLO_WORLD_LOOT_TABLE_ID);

    let table = loot_tables
        .table_mut(&table_id)
        .expect("Hello World Game must register its base LootTable");

    table.add_outcome(WheelOutcome::new(
        OutcomeId::new("on-steroids"),
        "Hello from Vapor! ...on steroids.",
    ));

    let mode_id = GameModeId::new(STEROIDS_MODE_ID);

    modes.register(GameModeDefinition::new(
        mode_id.clone(),
        WheelId::new(HELLO_WORLD_WHEEL_ID),
        LastOutcomeSpin,
    ));

    active_mode.select(mode_id);
}
