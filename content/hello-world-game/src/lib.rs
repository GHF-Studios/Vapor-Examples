//! A small wheel-driven salvage roguelike used to prove Vapor's terminal game
//! composition and game-mod extension path.
//!
//! The Game owns the wheel, run rules, and presentation. The Terminal Engine
//! owns terminal input, timing, the ECS host, and the framebuffer.

use bevy_ecs::{
    prelude::{Res, ResMut, Resource},
    schedule::{IntoScheduleConfigs, SystemSet},
};
use std::time::{SystemTime, UNIX_EPOCH};
use terminal_engine::{EngineControl, EngineSet, InputEvent, InputState, Key, TerminalFrame};

pub use terminal_engine::{App, Startup, Update, create_app};
pub use wheel_rules::{LootTable, LootTableId, OutcomeId, WheelDefinition, WheelId, WheelOutcome};

pub const HELLO_WORLD_LOOT_TABLE_ID: &str = "hello-world";
pub const HELLO_WORLD_WHEEL_ID: &str = "hello-world";
const SPIN_FRAME_COUNT: u8 = 36;
const STARTING_HULL: u8 = 3;

/// Stable phases that game mods can join to extend the wheel.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelGameSet {
    Register,
    Input,
    Spin,
    Present,
}

/// The single wheel and its current run state.
///
/// A game mod can add a wheel sector during [`WheelGameSet::Register`] without
/// introducing another registry or runtime service.
#[derive(Debug, Resource)]
pub struct WheelGame {
    wheel: WheelDefinition,
    table: LootTable,
    seed: u64,
    spin_count: u64,
    frames_remaining: u8,
    preview_sector: usize,
    result: Option<WheelOutcome>,
    run: Expedition,
}

impl WheelGame {
    /// Create a wheel with a deterministic seed, useful for tests and demos.
    pub fn new(seed: u64) -> Self {
        let table_id = LootTableId::new(HELLO_WORLD_LOOT_TABLE_ID);
        let table = LootTable::new(
            table_id.clone(),
            [
                WheelOutcome::new(OutcomeId::new("salvage"), "Loose salvage"),
                WheelOutcome::new(OutcomeId::new("salvage"), "Loose salvage"),
                WheelOutcome::new(OutcomeId::new("salvage"), "Loose salvage"),
                WheelOutcome::new(OutcomeId::new("salvage"), "Loose salvage"),
                WheelOutcome::new(OutcomeId::new("hazard"), "Debris strike"),
                WheelOutcome::new(OutcomeId::new("hazard"), "Debris strike"),
                WheelOutcome::new(OutcomeId::new("cache"), "Sealed cache"),
                WheelOutcome::new(OutcomeId::new("repair"), "Repair bay"),
            ],
        );

        Self {
            wheel: WheelDefinition::new(WheelId::new(HELLO_WORLD_WHEEL_ID), table_id),
            table,
            seed,
            spin_count: 0,
            frames_remaining: 0,
            preview_sector: 0,
            result: None,
            run: Expedition::default(),
        }
    }

    /// Add a sector from a game mod or other composed content.
    pub fn add_outcome(&mut self, outcome: WheelOutcome) {
        self.table.add_outcome(outcome);
    }

    /// Current sectors, in wheel order.
    pub fn sectors(&self) -> &[WheelOutcome] {
        self.table.outcomes()
    }

    /// The semantic wheel descriptor used by this game.
    pub fn wheel(&self) -> &WheelDefinition {
        &self.wheel
    }

    /// Number of sectors currently installed on the wheel.
    pub fn sector_count(&self) -> usize {
        self.table.outcomes().len()
    }

    fn is_spinning(&self) -> bool {
        self.frames_remaining > 0
    }

    fn request_spin(&mut self) {
        if !self.run.finished && !self.is_spinning() {
            self.frames_remaining = SPIN_FRAME_COUNT;
            self.result = None;
        }
    }

    fn advance_spin(&mut self) {
        if !self.is_spinning() {
            return;
        }

        let animation_seed = self.seed ^ self.spin_count.wrapping_mul(0xA076_1D64_78BD_642F);
        self.preview_sector = (mix64(animation_seed ^ u64::from(self.frames_remaining)) as usize)
            % self.sector_count();
        self.frames_remaining -= 1;

        if self.frames_remaining > 0 {
            return;
        }

        self.preview_sector =
            (mix64(self.seed.wrapping_add(self.spin_count)) as usize) % self.sector_count();
        self.spin_count = self.spin_count.saturating_add(1);
        let outcome = self.table.outcomes()[self.preview_sector].clone();
        self.run.resolve(outcome.id());
        self.result = Some(outcome);
    }

    fn extract(&mut self) {
        if !self.is_spinning() {
            self.run.extract();
        }
    }

    fn restart(&mut self) {
        self.frames_remaining = 0;
        self.result = None;
        self.run.restart();
    }
}

#[derive(Debug)]
struct Expedition {
    depth: u32,
    hull: u8,
    max_hull: u8,
    salvage: u32,
    banked_salvage: u32,
    best_depth: u32,
    finished: bool,
    last_event: String,
}

impl Default for Expedition {
    fn default() -> Self {
        Self {
            depth: 1,
            hull: STARTING_HULL,
            max_hull: STARTING_HULL,
            salvage: 0,
            banked_salvage: 0,
            best_depth: 1,
            finished: false,
            last_event: "Spin to descend. Extract to keep your haul.".to_owned(),
        }
    }
}

impl Expedition {
    fn resolve(&mut self, outcome: &OutcomeId) {
        if self.finished {
            return;
        }

        self.last_event = match outcome.as_str() {
            "salvage" => {
                self.salvage += 1;
                "Loose salvage: +1".to_owned()
            }
            "cache" => {
                self.salvage += 3;
                "Sealed cache: +3 salvage".to_owned()
            }
            "repair" if self.hull < self.max_hull => {
                self.hull += 1;
                "Repair bay: +1 hull".to_owned()
            }
            "repair" => {
                self.salvage += 1;
                "Hull full: repair converted to +1 salvage".to_owned()
            }
            "hazard" => {
                let damage = 1 + self.depth.saturating_sub(1) / 3;
                self.hull = self
                    .hull
                    .saturating_sub(u8::try_from(damage).unwrap_or(u8::MAX));
                format!("Debris strike: -{damage} hull")
            }
            // A mod can add a sector without changing the base game's rules.
            _ => {
                self.salvage += 2;
                "Bonus sector: +2 salvage".to_owned()
            }
        };

        if self.hull == 0 {
            self.best_depth = self.best_depth.max(self.depth);
            self.finished = true;
            self.last_event.push_str(" — run lost");
            return;
        }

        self.depth += 1;
        self.best_depth = self.best_depth.max(self.depth);
        if self.depth.is_multiple_of(3) {
            self.max_hull += 1;
            self.hull += 1;
            self.last_event.push_str("; plating reinforced (+1 hull)");
        }
    }

    fn extract(&mut self) {
        if self.finished || self.depth == 1 {
            return;
        }

        self.finished = true;
        self.banked_salvage += self.salvage;
        self.last_event = format!("Extracted safely: banked {} salvage", self.salvage);
    }

    fn restart(&mut self) {
        let banked_salvage = self.banked_salvage;
        let best_depth = self.best_depth;
        *self = Self::default();
        self.banked_salvage = banked_salvage;
        self.best_depth = best_depth;
        self.last_event = "New run. The wheel is ready.".to_owned();
    }
}

fn mix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

/// Install the game into a compatible Terminal Engine application.
pub fn install(app: &mut App) {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default();
    install_with_seed(app, seed);
}

/// Install the game with a stable wheel seed for deterministic tests or demos.
pub fn install_with_seed(app: &mut App, seed: u64) {
    app.insert_resource(WheelGame::new(seed))
        .configure_sets(Startup, WheelGameSet::Register.in_set(EngineSet::Configure))
        .configure_sets(Update, WheelGameSet::Input.in_set(EngineSet::Input))
        .configure_sets(Update, WheelGameSet::Spin.in_set(EngineSet::Simulate))
        .configure_sets(Update, WheelGameSet::Present.in_set(EngineSet::Present))
        .add_systems(Update, handle_input.in_set(WheelGameSet::Input))
        .add_systems(Update, advance_spin.in_set(WheelGameSet::Spin))
        .add_systems(Update, present.in_set(WheelGameSet::Present));
}

fn handle_input(
    input: Res<InputState>,
    mut game: ResMut<WheelGame>,
    mut engine: ResMut<EngineControl>,
) {
    for event in input.events() {
        let InputEvent::Key(input) = event else {
            continue;
        };

        match input.key {
            Key::Character(' ') | Key::Enter => game.request_spin(),
            Key::Character('e' | 'E') => game.extract(),
            Key::Character('r' | 'R') => game.restart(),
            Key::Character('q' | 'Q') | Key::Escape => engine.request_quit(),
            _ => {}
        }
    }
}

fn advance_spin(mut game: ResMut<WheelGame>) {
    game.advance_spin();
}

fn present(game: Res<WheelGame>, mut frame: ResMut<TerminalFrame>) {
    frame.draw_border();
    frame.write_centered(1, "WHEEL DESCENT");
    frame.write_centered(
        3,
        format!(
            "DEPTH {:02}   HULL {}/{}   HAUL {}   BANKED {}",
            game.run.depth,
            game.run.hull,
            game.run.max_hull,
            game.run.salvage,
            game.run.banked_salvage,
        ),
    );
    frame.write_centered(4, format!("WHEEL: {}", game.wheel.id()));

    let sectors = game
        .sectors()
        .iter()
        .map(|outcome| outcome.id().as_str().to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("  ");
    frame.write_centered(6, format!("[ {sectors} ]"));

    if game.is_spinning() {
        frame.write_centered(8, format!("---  {}  ---", game.preview_sector + 1));
        frame.write_centered(10, "The wheel is rattling through the dark...");
    } else if let Some(outcome) = &game.result {
        frame.write_centered(8, format!("---  {}  ---", game.preview_sector + 1));
        frame.write_centered(10, outcome.text());
    } else {
        frame.write_centered(8, "---  THE WHEEL WAITS  ---");
        frame.write_centered(10, "Spin to descend. Extract to keep your haul.");
    }
    frame.write_centered(12, &game.run.last_event);
    frame.write_centered(
        14,
        format!(
            "BEST DEPTH {}   |   SESSION BANK {}",
            game.run.best_depth, game.run.banked_salvage
        ),
    );

    let controls = if game.run.finished {
        "[R] new run   [Q/ESC] quit"
    } else if game.is_spinning() {
        "Wheel spinning..."
    } else {
        "[SPACE/ENTER] spin   [E] extract   [Q/ESC] quit"
    };
    let controls_y = frame.height().saturating_sub(2);
    frame.write_centered(controls_y, controls);
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_engine::{InputEvent, KeyInput};

    #[test]
    fn a_mod_can_add_a_sector_without_a_new_registry() {
        let mut game = WheelGame::new(17);
        game.add_outcome(WheelOutcome::new(OutcomeId::new("bonus"), "Bonus sector"));

        assert_eq!(game.sector_count(), 9);
        assert_eq!(game.sectors().last().unwrap().text(), "Bonus sector");
    }

    #[test]
    fn installed_wheel_has_eight_risk_and_reward_sectors() {
        let game = WheelGame::new(17);
        assert_eq!(game.sector_count(), 8);
        assert_eq!(
            game.wheel().loot_table().as_str(),
            HELLO_WORLD_LOOT_TABLE_ID
        );
    }

    #[test]
    fn salvage_advances_depth_and_deep_rooms_reinforce_hull() {
        let mut run = Expedition::default();
        run.resolve(&OutcomeId::new("salvage"));
        assert_eq!(run.salvage, 1);
        assert_eq!(run.depth, 2);
        assert_eq!(run.hull, 3);

        run.resolve(&OutcomeId::new("salvage"));
        assert_eq!(run.depth, 3);
        assert_eq!(run.max_hull, 4);
        assert_eq!(run.hull, 4);
    }

    #[test]
    fn deep_hazard_can_end_a_run() {
        let mut run = Expedition {
            depth: 4,
            hull: 2,
            ..Expedition::default()
        };

        run.resolve(&OutcomeId::new("hazard"));

        assert!(run.finished);
        assert_eq!(run.hull, 0);
        assert_eq!(run.best_depth, 4);
    }

    #[test]
    fn extraction_banks_salvage_and_restart_keeps_the_bank() {
        let mut run = Expedition::default();
        run.resolve(&OutcomeId::new("salvage"));
        run.resolve(&OutcomeId::new("cache"));

        run.extract();
        assert_eq!(run.banked_salvage, 4);
        assert!(run.finished);

        run.restart();
        assert_eq!(run.banked_salvage, 4);
        assert_eq!(run.depth, 1);
        assert_eq!(run.hull, STARTING_HULL);
    }

    #[test]
    fn spin_requires_input_then_animates_before_resolving() {
        let mut app = terminal_engine::create_app();
        install_with_seed(&mut app, 17);
        app.update();
        assert!(!app.world().resource::<WheelGame>().is_spinning());
        assert!(app.world().resource::<WheelGame>().result.is_none());

        app.world_mut()
            .resource_mut::<InputState>()
            .push_event(InputEvent::Key(KeyInput::plain(Key::Character(' '))));
        app.update();
        app.world_mut().resource_mut::<InputState>().clear();
        assert!(app.world().resource::<WheelGame>().is_spinning());
        assert!(app.world().resource::<WheelGame>().result.is_none());

        for _ in 0..SPIN_FRAME_COUNT {
            app.update();
        }

        let game = app.world().resource::<WheelGame>();
        assert!(!game.is_spinning());
        assert!(game.result.is_some());
        assert_eq!(game.run.depth, 2);
        assert_eq!(
            game.preview_sector + 1,
            (mix64(17) as usize % game.sector_count()) + 1
        );
    }

    #[test]
    fn extraction_and_restart_controls_preserve_the_saved_haul() {
        let mut app = terminal_engine::create_app();
        install_with_seed(&mut app, 17);
        app.world_mut()
            .resource_mut::<WheelGame>()
            .run
            .resolve(&OutcomeId::new("salvage"));
        app.world_mut()
            .resource_mut::<InputState>()
            .push_event(InputEvent::Key(KeyInput::plain(Key::Character('e'))));
        app.update();
        app.world_mut().resource_mut::<InputState>().clear();
        assert!(app.world().resource::<WheelGame>().run.finished);
        assert_eq!(app.world().resource::<WheelGame>().run.banked_salvage, 1);

        app.world_mut()
            .resource_mut::<InputState>()
            .push_event(InputEvent::Key(KeyInput::plain(Key::Character('r'))));
        app.update();
        app.world_mut().resource_mut::<InputState>().clear();
        let game = app.world().resource::<WheelGame>();
        assert!(!game.run.finished);
        assert_eq!(game.run.depth, 1);
        assert_eq!(game.run.banked_salvage, 1);
    }
}
