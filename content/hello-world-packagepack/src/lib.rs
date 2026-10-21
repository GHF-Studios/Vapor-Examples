//! Runtime entrypoint composition for the Workshop wheel example.

/// Install the selected Game and its selected Mod into the Engine application.
pub fn install_composition(app: &mut terminal_engine::App) {
    hello_world_game::install(app);
    tiny_game_mod::install(app);
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::World;
    use hello_world_game::WheelGame;

    #[test]
    fn packagepack_installs_game_and_selected_mod_into_engine() {
        let mut app = terminal_engine::create_app();
        super::install_composition(&mut app);
        app.update();

        let world: &World = app.world();
        let game = world.resource::<WheelGame>();
        assert_eq!(game.sector_count(), 9);
        assert_eq!(game.sectors().last().unwrap().id().as_str(), "on-steroids");
    }
}
