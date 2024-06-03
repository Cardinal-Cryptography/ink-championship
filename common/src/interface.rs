use ink::env::{DefaultEnvironment, Environment};
use ink::prelude::vec::Vec;
use crate::{ Field, FieldEntry, Player, State};

#[ink::trait_definition]
pub trait IGame {
    /// The buy-in amount to register a player.
    #[ink(message)]
    fn buy_in_amount(&self) -> <DefaultEnvironment as Environment>::Balance;

    /// The total amount of rounds this game is to be played for.
    #[ink(message)]
    fn total_rounds(&self) -> u32;

    /// How much gas each player is allowed to use per round.
    #[ink(message)]
    fn gas_limit(&self) -> u64;

    /// Describes into many groups the players should be partitioned.
    ///
    /// How often [`submit_turn`] needs to be called until all players
    /// made a turn.
    #[ink(message)]
    fn num_batches(&self) -> u32;

    /// How much gas each player is allowed to consume for the whole game.
    #[ink(message)]
    fn gas_budget(&self) -> u64;

    /// The current game state.
    #[ink(message)]
    fn state(&self) -> State;

    /// Returns `true` if the game is running.
    #[ink(message)]
    fn is_running(&self) -> bool;

    /// List of all players sorted by score and gas costs.
    #[ink(message)]
    fn players_sorted(&self) -> Vec<Player>;

    /// Returns the dimensions of the board.
    #[ink(message)]
    fn dimensions(&self) -> Field;

    /// Returns the value (owner) of the supplied field.
    #[ink(message)]
    fn field(&self, coord: Field) -> Option<FieldEntry>;

    /// Returns the complete board.
    ///
    /// The index into the vector is calculated as `x + y * width`.
    #[ink(message)]
    fn board(&self) -> Vec<Option<FieldEntry>>;
}