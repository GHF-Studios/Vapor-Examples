//! Hello World Game: the evolving center of Vapor's
//! Hello World on Steroids techdemo.
//!
//! It now behaves as an actual tiny interactive terminal game.
//!
//! The Game owns:
//!
//! - LootTables;
//! - Wheels;
//! - GameModes;
//! - spin strategies;
//! - input interpretation;
//! - game-state resolution;
//! - Game-specific presentation.
//!
//! The Terminal Engine owns the actual terminal, input acquisition, timing,
//! ECS host, and framebuffer.

use bevy_ecs::{
    prelude::{Res, ResMut, Resource},
    schedule::{IntoScheduleConfigs, SystemSet},
};
use std::collections::BTreeMap;
use terminal_engine::{EngineControl, EngineSet, InputEvent, InputState, Key, TerminalFrame};

pub use terminal_engine::{App, Startup, Update};

pub use wheel_rules::{
    GameModeId, LootTable, LootTableId, OutcomeId, WheelDefinition, WheelId, WheelOutcome,
};

pub const HELLO_WORLD_LOOT_TABLE_ID: &str = "hello-world";
pub const HELLO_WORLD_WHEEL_ID: &str = "hello-world";
pub const CLASSIC_GAME_MODE_ID: &str = "classic";

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelGameSet {
    Register,
    Input,
    Spin,
    Resolve,
    Present,
}

#[derive(Resource, Default)]
pub struct LootTableRegistry {
    tables: BTreeMap<LootTableId, LootTable>,
}

impl LootTableRegistry {
    pub fn register(&mut self, table: LootTable) -> Option<LootTable> {
        self.tables.insert(table.id().clone(), table)
    }

    pub fn table(&self, id: &LootTableId) -> Option<&LootTable> {
        self.tables.get(id)
    }

    pub fn table_mut(&mut self, id: &LootTableId) -> Option<&mut LootTable> {
        self.tables.get_mut(id)
    }
}

#[derive(Resource, Default)]
pub struct WheelRegistry {
    wheels: BTreeMap<WheelId, WheelDefinition>,
}

impl WheelRegistry {
    pub fn register(&mut self, wheel: WheelDefinition) -> Option<WheelDefinition> {
        self.wheels.insert(wheel.id().clone(), wheel)
    }

    pub fn wheel(&self, id: &WheelId) -> Option<&WheelDefinition> {
        self.wheels.get(id)
    }
}

/// Game-owned selection contract.
///
/// A downstream Game Mod may provide a new implementation without Vapor or
/// the Terminal Engine knowing anything about that implementation.
pub trait SpinStrategy: Send + Sync + 'static {
    fn select(&self, outcomes: &[WheelOutcome], spin_index: u64) -> Option<usize>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RoundRobinSpin;

impl SpinStrategy for RoundRobinSpin {
    fn select(&self, outcomes: &[WheelOutcome], spin_index: u64) -> Option<usize> {
        if outcomes.is_empty() {
            None
        } else {
            Some(spin_index as usize % outcomes.len())
        }
    }
}

pub struct GameModeDefinition {
    id: GameModeId,
    wheel: WheelId,
    strategy: Box<dyn SpinStrategy>,
}

impl GameModeDefinition {
    pub fn new(id: GameModeId, wheel: WheelId, strategy: impl SpinStrategy) -> Self {
        Self {
            id,
            wheel,
            strategy: Box::new(strategy),
        }
    }

    pub fn id(&self) -> &GameModeId {
        &self.id
    }

    pub fn wheel(&self) -> &WheelId {
        &self.wheel
    }

    pub fn strategy(&self) -> &dyn SpinStrategy {
        self.strategy.as_ref()
    }
}

#[derive(Resource, Default)]
pub struct GameModeRegistry {
    modes: BTreeMap<GameModeId, GameModeDefinition>,
}

impl GameModeRegistry {
    pub fn register(&mut self, mode: GameModeDefinition) -> Option<GameModeDefinition> {
        self.modes.insert(mode.id().clone(), mode)
    }

    pub fn mode(&self, id: &GameModeId) -> Option<&GameModeDefinition> {
        self.modes.get(id)
    }
}

#[derive(Resource)]
pub struct ActiveGameMode {
    id: GameModeId,
}

impl ActiveGameMode {
    pub fn new(id: GameModeId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &GameModeId {
        &self.id
    }

    pub fn select(&mut self, id: GameModeId) {
        self.id = id;
    }
}

#[derive(Resource, Default)]
struct SpinRequest(bool);

impl SpinRequest {
    fn request(&mut self) {
        self.0 = true;
    }

    fn take(&mut self) -> bool {
        std::mem::take(&mut self.0)
    }
}

#[derive(Resource, Default)]
struct SpinCounter(u64);

#[derive(Resource, Default)]
pub struct SpinSelection {
    fresh: bool,
    mode: Option<GameModeId>,
    loot_table: Option<LootTableId>,
    outcome: Option<OutcomeId>,
}

impl SpinSelection {
    pub fn mode(&self) -> Option<&GameModeId> {
        self.mode.as_ref()
    }

    pub fn loot_table(&self) -> Option<&LootTableId> {
        self.loot_table.as_ref()
    }

    pub fn outcome(&self) -> Option<&OutcomeId> {
        self.outcome.as_ref()
    }
}

#[derive(Resource, Default)]
pub struct SpinResult {
    mode: Option<GameModeId>,
    outcome: Option<WheelOutcome>,
}

impl SpinResult {
    pub fn mode(&self) -> Option<&GameModeId> {
        self.mode.as_ref()
    }

    pub fn outcome(&self) -> Option<&WheelOutcome> {
        self.outcome.as_ref()
    }
}

/// Install the Hello World Game into a compatible Terminal Engine App.
///
/// This remains a Terminal-Engine-specific integration surface rather than a
/// universal Vapor Game ABI.
pub fn install(app: &mut App) {
    let mut loot_tables = LootTableRegistry::default();

    loot_tables.register(LootTable::new(
        LootTableId::new(HELLO_WORLD_LOOT_TABLE_ID),
        [
            WheelOutcome::new(OutcomeId::new("hello"), "Hello from Vapor!"),
            WheelOutcome::new(OutcomeId::new("hello-again"), "Hello again from Vapor!"),
            WheelOutcome::new(OutcomeId::new("cargo"), "Cargo says hello too."),
        ],
    ));

    let mut wheels = WheelRegistry::default();

    wheels.register(WheelDefinition::new(
        WheelId::new(HELLO_WORLD_WHEEL_ID),
        LootTableId::new(HELLO_WORLD_LOOT_TABLE_ID),
    ));

    let mut modes = GameModeRegistry::default();

    modes.register(GameModeDefinition::new(
        GameModeId::new(CLASSIC_GAME_MODE_ID),
        WheelId::new(HELLO_WORLD_WHEEL_ID),
        RoundRobinSpin,
    ));

    app.insert_resource(loot_tables)
        .insert_resource(wheels)
        .insert_resource(modes)
        .insert_resource(ActiveGameMode::new(GameModeId::new(CLASSIC_GAME_MODE_ID)))
        .init_resource::<SpinRequest>()
        .init_resource::<SpinCounter>()
        .init_resource::<SpinSelection>()
        .init_resource::<SpinResult>()
        .configure_sets(Startup, WheelGameSet::Register.in_set(EngineSet::Configure))
        .configure_sets(Update, WheelGameSet::Input.in_set(EngineSet::Input))
        .configure_sets(Update, WheelGameSet::Spin.in_set(EngineSet::Simulate))
        .configure_sets(
            Update,
            WheelGameSet::Resolve
                .in_set(EngineSet::Simulate)
                .after(WheelGameSet::Spin),
        )
        .configure_sets(Update, WheelGameSet::Present.in_set(EngineSet::Present))
        .add_systems(Update, handle_input.in_set(WheelGameSet::Input))
        .add_systems(Update, spin.in_set(WheelGameSet::Spin))
        .add_systems(Update, resolve.in_set(WheelGameSet::Resolve))
        .add_systems(Update, present.in_set(WheelGameSet::Present));
}

fn handle_input(
    input: Res<InputState>,
    mut spin: ResMut<SpinRequest>,
    mut engine: ResMut<EngineControl>,
) {
    for event in input.events() {
        let InputEvent::Key(input) = event else {
            continue;
        };

        match input.key {
            Key::Character(' ') | Key::Enter => {
                spin.request();
            }

            Key::Character('q' | 'Q') | Key::Escape => {
                engine.request_quit();
            }

            _ => {}
        }
    }
}

fn spin(
    modes: Res<GameModeRegistry>,
    active: Res<ActiveGameMode>,
    wheels: Res<WheelRegistry>,
    loot_tables: Res<LootTableRegistry>,
    mut request: ResMut<SpinRequest>,
    mut counter: ResMut<SpinCounter>,
    mut selection: ResMut<SpinSelection>,
) {
    selection.fresh = false;

    if !request.take() {
        return;
    }

    let Some(mode) = modes.mode(active.id()) else {
        return;
    };

    let Some(wheel) = wheels.wheel(mode.wheel()) else {
        return;
    };

    let Some(table) = loot_tables.table(wheel.loot_table()) else {
        return;
    };

    let selected = mode
        .strategy()
        .select(table.outcomes(), counter.0)
        .and_then(|index| table.outcomes().get(index));

    counter.0 += 1;

    let Some(outcome) = selected else {
        return;
    };

    selection.fresh = true;
    selection.mode = Some(mode.id().clone());
    selection.loot_table = Some(table.id().clone());
    selection.outcome = Some(outcome.id().clone());
}

fn resolve(
    selection: Res<SpinSelection>,
    loot_tables: Res<LootTableRegistry>,
    mut result: ResMut<SpinResult>,
) {
    if !selection.fresh {
        return;
    }

    let (Some(mode), Some(table_id), Some(outcome_id)) = (
        selection.mode(),
        selection.loot_table(),
        selection.outcome(),
    ) else {
        return;
    };

    let Some(table) = loot_tables.table(table_id) else {
        return;
    };

    let Some(outcome) = table.outcome(outcome_id) else {
        return;
    };

    result.mode = Some(mode.clone());
    result.outcome = Some(outcome.clone());
}

fn present(
    active: Res<ActiveGameMode>,
    modes: Res<GameModeRegistry>,
    wheels: Res<WheelRegistry>,
    loot_tables: Res<LootTableRegistry>,
    result: Res<SpinResult>,
    mut frame: ResMut<TerminalFrame>,
) {
    frame.draw_border();

    frame.write_centered(1, "HELLO WORLD ON STEROIDS");

    frame.write(3, 3, format!("Mode: {}", active.id()));

    let mode = modes.mode(active.id());

    let wheel = mode.and_then(|mode| wheels.wheel(mode.wheel()));

    let table = wheel.and_then(|wheel| loot_tables.table(wheel.loot_table()));

    if let Some(wheel) = wheel {
        frame.write(3, 4, format!("Wheel: {}", wheel.id()));
    }

    if let Some(table) = table {
        let labels = table
            .outcomes()
            .iter()
            .map(|outcome| outcome.id().as_str())
            .collect::<Vec<_>>()
            .join(" | ");

        frame.write_centered(7, format!("[ {labels} ]"));
    }

    let result_text = result
        .outcome()
        .map(WheelOutcome::text)
        .unwrap_or("(press SPACE to spin)");

    frame.write_centered(10, format!("Last spin: {result_text}"));

    let y = frame.height().saturating_sub(2);

    frame.write_centered(y, "[SPACE/ENTER] spin    [Q/ESC] quit");
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_engine::{InputEvent, KeyInput};

    #[test]
    fn installs_coherent_base_game_content() {
        let mut app = terminal_engine::create_app();

        install(&mut app);

        let table = app
            .world()
            .resource::<LootTableRegistry>()
            .table(&LootTableId::new(HELLO_WORLD_LOOT_TABLE_ID))
            .unwrap();

        assert_eq!(table.outcomes().len(), 3);

        let wheel = app
            .world()
            .resource::<WheelRegistry>()
            .wheel(&WheelId::new(HELLO_WORLD_WHEEL_ID))
            .unwrap();

        assert_eq!(wheel.loot_table().as_str(), HELLO_WORLD_LOOT_TABLE_ID);

        assert!(
            app.world()
                .resource::<GameModeRegistry>()
                .mode(&GameModeId::new(CLASSIC_GAME_MODE_ID,))
                .is_some()
        );
    }

    #[test]
    fn spins_only_when_input_requests_it() {
        let mut app = terminal_engine::create_app();

        install(&mut app);

        app.update();

        assert!(app.world().resource::<SpinResult>().outcome().is_none());

        app.world_mut()
            .resource_mut::<InputState>()
            .push_event(InputEvent::Key(KeyInput::plain(Key::Character(' '))));

        app.update();

        assert_eq!(
            app.world()
                .resource::<SpinResult>()
                .outcome()
                .unwrap()
                .text(),
            "Hello from Vapor!"
        );

        app.world_mut().resource_mut::<InputState>().clear();

        app.update();

        assert_eq!(
            app.world()
                .resource::<SpinResult>()
                .outcome()
                .unwrap()
                .text(),
            "Hello from Vapor!"
        );

        app.world_mut()
            .resource_mut::<InputState>()
            .push_event(InputEvent::Key(KeyInput::plain(Key::Character(' '))));

        app.update();

        assert_eq!(
            app.world()
                .resource::<SpinResult>()
                .outcome()
                .unwrap()
                .text(),
            "Hello again from Vapor!"
        );
    }
}
