//! Tiny Engine used by Vapor's architecture-proving vertical slice.

/// Run one statically composed application.
///
/// The concrete Game and Mod behavior is supplied by the generated Vapor App.
/// The Engine remains responsible for application lifecycle semantics.
pub fn run<Game, GameMod>(game: Game, game_mod: GameMod)
where
    Game: FnOnce(),
    GameMod: FnOnce(),
{
    println!("[Engine] starting");

    game();
    game_mod();

    println!("[Engine] shutting down");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn runs_game_then_mod() {
        let events = RefCell::new(Vec::new());

        run(
            || {
                events.borrow_mut().push("game");
            },
            || {
                events.borrow_mut().push("mod");
            },
        );

        assert_eq!(events.into_inner(), vec!["game", "mod"]);
    }
}
