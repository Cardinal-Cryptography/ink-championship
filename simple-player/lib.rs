#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use player::{Player as TestPlayer, PlayerRef as TestPlayerRef};

#[ink::contract]
mod player {
    use common::*;
    use squink_splash::GameRef;

    #[ink(storage)]
    pub struct Player {
        game_contract: AccountId,
        dimensions: (u32, u32),
        next_turn: u32,
    }

    impl Player {
        #[ink(constructor)]
        pub fn new(game_contract: AccountId, dimensions: (u32, u32), start: u32) -> Self {
            Self {
                game_contract,
                dimensions,
                next_turn: start,
            }
        }

        /// This is the function that will be called during every game round.
        ///
        /// The function returns an `(x, y)` coordinate of the pixel which you
        /// want to color.
        ///
        /// # Notes
        ///
        /// The function signature `&mut self` is so that you can retain state
        /// in the contract's storage if you want to.
        ///
        /// The function can be named as you like, but it always needs to have
        /// a defined selector of `0`.
        #[ink(message, selector = 0)]
        pub fn your_turn(&mut self) -> Option<(u32, u32)> {
            let turn = self.next_turn;
            let x = self.dimensions.0;
            self.next_turn = self.next_turn.saturating_add(1);

            let first_choice = turn.rem_euclid(x);

            let game: GameRef = ink::env::call::FromAccountId::from_account_id(self.game_contract);
            if game.field(Field { x: first_choice, y: first_choice }).is_none() {
                Some((first_choice, first_choice))
            } else {
                let new_choice = (first_choice + 1).rem_euclid(x);
                Some((new_choice, first_choice))
            }
        }
    }
}
