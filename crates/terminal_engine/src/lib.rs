//! Utilities for the terminal engine example.

use std::{
    io::{self, Write},
    time::{SystemTime, UNIX_EPOCH},
};

/// Minimal game definition consumed by the terminal engine example.
#[derive(Debug, Clone)]
pub struct TerminalReelGame {
    /// Display title.
    pub title: String,
    /// Short one-line description.
    pub subtitle: String,
    /// Interactive prompt prefix.
    pub prompt: String,
    /// Charge available at the start of a session.
    pub starting_charge: i32,
    /// Reel symbols.
    pub symbols: Vec<String>,
}

/// Run a terminal reel game session.
pub fn run_terminal_reel(game: TerminalReelGame) -> Result<(), String> {
    if game.symbols.is_empty() {
        return Err("terminal reel game requires at least one symbol".to_owned());
    }

    println!("{}", game.title);
    println!("  {}", game.subtitle);
    println!("  Press Enter to spin. Type `help` for commands or `quit` to return.");
    println!();

    let mut session = GameSession::new(game, seed());
    let stdin = io::stdin();
    loop {
        print!("{}> ", session.definition.prompt);
        io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush prompt: {error}"))?;

        let mut input = String::new();
        let read = stdin
            .read_line(&mut input)
            .map_err(|error| format!("failed to read input: {error}"))?;
        if read == 0 {
            println!();
            println!("terminal game session closed.");
            return Ok(());
        }

        match input.trim().to_ascii_lowercase().as_str() {
            "" | "s" | "spin" => session.spin(),
            "h" | "help" | "?" => print_help(),
            "status" => session.status(),
            "reset" => session.reset(),
            "q" | "quit" | "exit" => {
                println!("terminal game session closed.");
                return Ok(());
            }
            command => {
                println!("unknown command: {command}");
                println!("type `help` for commands");
            }
        }
    }
}

fn print_help() {
    println!("Commands");
    println!("  spin    spin the signal reels");
    println!("  status  show game state");
    println!("  reset   reset game charge and score");
    println!("  quit    return to Vapor");
}

struct GameSession {
    definition: TerminalReelGame,
    rng: Lcg,
    spin_count: u32,
    charge: i32,
    score: u32,
}

impl GameSession {
    fn new(definition: TerminalReelGame, seed: u64) -> Self {
        let charge = definition.starting_charge;
        Self {
            definition,
            rng: Lcg::new(seed),
            spin_count: 0,
            charge,
            score: 0,
        }
    }

    fn spin(&mut self) {
        if self.charge <= 0 {
            println!("charge is empty; type `reset` to restart the game loop");
            return;
        }

        self.spin_count += 1;
        self.charge -= 1;

        let reels = [self.next_symbol(), self.next_symbol(), self.next_symbol()];
        let outcome = score_reels(&reels);
        self.score += outcome.points;
        self.charge += outcome.charge;

        println!(
            "spin {:02}: [{:^6}] [{:^6}] [{:^6}]",
            self.spin_count, reels[0], reels[1], reels[2]
        );
        println!(
            "  result: {} (+{} score, {:+} charge)",
            outcome.label, outcome.points, outcome.charge
        );
        println!("  state: score {}, charge {}", self.score, self.charge);
    }

    fn status(&self) {
        println!("Game");
        println!("  spins: {}", self.spin_count);
        println!("  score: {}", self.score);
        println!("  charge: {}", self.charge);
    }

    fn reset(&mut self) {
        self.spin_count = 0;
        self.charge = self.definition.starting_charge;
        self.score = 0;
        println!("game state reset");
    }

    fn next_symbol(&mut self) -> String {
        let index = self.rng.next_index(self.definition.symbols.len());
        self.definition.symbols[index].clone()
    }
}

struct Outcome {
    label: &'static str,
    points: u32,
    charge: i32,
}

fn score_reels(reels: &[String; 3]) -> Outcome {
    if reels[0] == reels[1] && reels[1] == reels[2] {
        Outcome {
            label: "full signal lock",
            points: 30,
            charge: 4,
        }
    } else if reels[0] == reels[1] || reels[0] == reels[2] || reels[1] == reels[2] {
        Outcome {
            label: "partial signal lock",
            points: 8,
            charge: 1,
        }
    } else {
        Outcome {
            label: "signal drift",
            points: 1,
            charge: 0,
        }
    }
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_index(&mut self, len: usize) -> usize {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 32) as usize) % len
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::score_reels;

    #[test]
    fn scores_reel_matches() {
        assert_eq!(score_reels(&reels(["VAPOR", "VAPOR", "VAPOR"])).points, 30);
        assert_eq!(score_reels(&reels(["VAPOR", "CAST", "VAPOR"])).points, 8);
        assert_eq!(score_reels(&reels(["VAPOR", "CAST", "PACK"])).points, 1);
    }

    fn reels(values: [&str; 3]) -> [String; 3] {
        values.map(ToOwned::to_owned)
    }
}
