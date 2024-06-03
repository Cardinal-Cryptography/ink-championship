use core::cmp::Reverse;

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

#[derive(scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(Debug, scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GameInfo {
    pub rounds_played: u32,
    pub gas_left: u64,
    pub player_scores: Vec<(String, u64)>,
}

/// The game can be in different states over its lifetime.
#[derive(scale::Decode, scale::Encode, Clone)]
#[cfg_attr(
    feature = "std",
    derive(Debug, scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum State {
    /// The initial state of the game.
    ///
    /// The game is in this state right after instantiation of the contract. This is
    /// the only state in which players can be registered. No turns can be submitted
    /// in this state.
    Forming {
        /// When this block is reached everybody can can call `start_game` in order
        /// to progress the state to `Running`.
        earliest_start: u32,
    },
    /// This is the actual playing phase which is entered after calling `start_game`.
    ///
    /// No new players can be registered in this phase.
    Running {
        /// The number of rounds that are already played in the current game.
        rounds_played: u32,
    },
    /// The game is finished an the pot has been payed out to the `winner`.
    Finished {
        /// The player with the highest score when the game ended.
        ///
        /// This player is also the one which is allowed to call `destroy` to remove
        /// the contract. This means that the winner will also collect the storage
        /// deposits put down by all players as an additional price.
        winner: AccountId,
    },
}

#[derive(scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(Debug, scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Player {
    pub id: AccountId,
    pub name: String,
    pub gas_used: u64,
    pub score: u64,
}

impl Player {
    /// Return the key to sort by (winner is min value by this order)
    pub fn scoring_order(&self) -> impl Ord {
        (Reverse(self.score), self.gas_used)
    }
}

/// Describing either a single point in the field or its dimensions.
#[derive(scale::Decode, scale::Encode, Clone, Copy, Debug)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Field {
    /// The width component.
    pub x: u32,
    /// The height component.
    pub y: u32,
}

impl Field {
    pub fn len(&self) -> u32 {
        self.x.saturating_mul(self.y)
    }
}

/// Info for each occupied board entry.
#[derive(scale::Decode, scale::Encode, Debug)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FieldEntry {
    /// Player to claimed the field.
    pub owner: AccountId,
    /// The round in which the field was claimed.
    pub claimed_at: u32,
}

/// The different effects resulting from a player making a turn.
///
/// Please note that these are only the failures that don't make the transaction fail
/// and hence cause an actual state change. For example, trying to do multiple turns
/// per block or submitting a turn for an unregistered player are not covered.
#[derive(scale::Decode, scale::Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TurnOutcome {
    /// A field was painted.
    Success {
        /// The field that was painted.
        turn: Field,
    },
    /// The contract's turn lies outside of the playing field.
    OutOfBounds {
        /// The turn that lies outside the playing field.
        turn: Field,
    },
    /// Someone else already painted the field and hence it can't be painted.
    Occupied {
        /// The turn that tried to paint.
        turn: Field,
        /// The player that occupies the field that was tried to be painted by `turn`.
        player: AccountId,
    },
    /// Player contract failed to return a result. This happens if it
    /// panicked, ran out of gas, returns garbage or is not even a contract.
    BrokenPlayer,
    /// Player decided to not make a turn and hence was charged no gas.
    NoTurn,
    /// Contract doesn't have any budget left and isn't called anymore.
    BudgetExhausted,
}
