//! Shared semantic identifiers for the Hello World on Steroids wheel domain.
//!
//! This is reusable Rust vocabulary only. It has no Engine/Game/Mod runtime
//! semantics of its own.

use std::fmt;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(WheelId);
string_id!(OutcomeId);
string_id!(LootTableId);
string_id!(GameModeId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_preserve_authored_names() {
        assert_eq!(WheelId::new("classic").as_str(), "classic");
        assert_eq!(OutcomeId::new("jackpot").as_str(), "jackpot");
        assert_eq!(LootTableId::new("starter").as_str(), "starter");
        assert_eq!(GameModeId::new("classic").as_str(), "classic");
    }
}
