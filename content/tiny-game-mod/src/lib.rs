//! A tiny mod that adds one sector to the composed wheel.

use bevy_ecs::{prelude::ResMut, schedule::IntoScheduleConfigs};
use hello_world_game::{App, OutcomeId, Startup, WheelGame, WheelGameSet, WheelOutcome};

pub fn install(app: &mut App) {
    app.add_systems(Startup, add_bonus_sector.in_set(WheelGameSet::Register));
}

fn add_bonus_sector(mut game: ResMut<WheelGame>) {
    game.add_outcome(WheelOutcome::new(
        OutcomeId::new("on-steroids"),
        "Mod sector: +2 salvage",
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use hello_world_game::{WheelGame, create_app, install as install_game};

    #[test]
    fn composed_mod_adds_its_sector_during_registration() {
        let mut app = create_app();
        install_game(&mut app);
        install(&mut app);
        app.update();

        let game = app.world().resource::<WheelGame>();
        assert_eq!(game.sector_count(), 9);
        assert_eq!(game.sectors().last().unwrap().id().as_str(), "on-steroids");
    }
}
