//! Shared domain vocabulary for the terminal wheel-game examples.
//!
//! This deliberately contains reusable Rust vocabulary only. It has no
//! Engine/Game/Mod or runtime-composition semantics of its own.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WheelId(String);

impl WheelId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WheelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutcomeId(String);

impl OutcomeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OutcomeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_preserve_authored_names() {
        let wheel = WheelId::new("classic");
        let outcome = OutcomeId::new("jackpot");

        assert_eq!(wheel.as_str(), "classic");
        assert_eq!(outcome.as_str(), "jackpot");
    }
}