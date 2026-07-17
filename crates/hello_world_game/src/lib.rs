//! Example game library loaded by the terminal engine example.

use terminal_engine::TerminalReelGame;

/// Build the example game definition.
pub fn game() -> TerminalReelGame {
    TerminalReelGame {
        title: "Hello World on Steroids".to_owned(),
        subtitle: "A tiny terminal game loaded from a Vapor game artifact.".to_owned(),
        prompt: "hello-world".to_owned(),
        starting_charge: 12,
        symbols: ["VAPOR", "CAST", "PACK", "MOD", "RIFT", "LOOP"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    }
}

/// Rust-native example entrypoint loaded by the matching terminal engine.
#[unsafe(no_mangle)]
pub extern "Rust" fn terminal_game_definition_v1() -> TerminalReelGame {
    game()
}
