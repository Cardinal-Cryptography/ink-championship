#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod error;
mod structs;

use core::ops::RangeInclusive;
pub use error::*;
pub use structs::*;

/// The amount of players that are allowed to register for a single game.
pub const PLAYER_LIMIT: usize = 80;

/// The amount of gas we want to allocate to all players within one turn.
///
/// Should be smaller than the maximum extrinsic weight since we also need to account
/// for the overhead of the game contract itself.
pub const GAS_LIMIT_ALL_PLAYERS: u64 = 250_000_000_000;

/// Maximum number of bytes in a players name.
pub const ALLOWED_NAME_SIZES: RangeInclusive<usize> = 3..=16;
