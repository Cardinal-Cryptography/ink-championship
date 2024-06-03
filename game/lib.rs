#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use contract::{
    SquinkSplash as Game,
    SquinkSplashRef as GameRef,
};

#[ink::contract]
mod contract {
    use ink::{
        env::{
            call::{
                build_call,
                Call,
                ExecutionInput,
                Selector,
            },
            debug_println,
            CallFlags,
            DefaultEnvironment,
        },
        prelude::{
            string::String,
            vec::Vec,
        },
        storage::{
            Lazy,
            Mapping,
        },
    };
    use common::*;

    #[ink(storage)]
    pub struct SquinkSplash {
        /// In which game phase is this contract.
        state: State,
        /// List of fields with their owner (if any).
        board: Mapping<u32, FieldEntry>,
        /// Width and height of the board.
        dimensions: Field,
        /// List of all players.
        players: Lazy<Vec<Player>>,
        /// The amount of balance that needs to be payed to join the game.
        buy_in: Balance,
        /// The amount of blocks that this game is played for once it started.
        rounds: u32,
        /// The block number the last turn was made.
        last_turn: Lazy<u32>,
        /// The opener is allowed to start the game early.
        opener: AccountId,
    }

    /// A player joined the game by calling [`register_player`].
    #[ink(event)]
    pub struct PlayerRegistered {
        /// The player contract account ID.
        player: AccountId,
    }

    /// The rounds played have increased. This is used for the client side to keep
    /// the [`TurnTaken`] events and "Blocks" UI in sync. Events are emitted before
    /// block number changes, so re-fetching [`rounds_played`] on a block change
    /// causes a brief delay in the UI.
    #[ink(event)]
    pub struct RoundIncremented {
        /// The number of rounds played.
        rounds_played: u32,
    }

    /// Someone started the game by calling [`start_game`].
    #[ink(event)]
    pub struct GameStarted {
        /// The account start called [`start_game`].
        starter: AccountId,
    }

    /// A player attempted to make a turn.
    #[ink(event)]
    pub struct TurnTaken {
        /// The player that attempted the turn.
        player: AccountId,
        /// The effect of the turn that was performed by the player.
        outcome: TurnOutcome,
    }

    /// Someone ended the game by calling [`end_game`].
    ///
    /// This event doesn't contain information about the winner because the contract still
    /// exists. Interested parties can read this information from the contract by calling
    /// [`state`] and [`player_scores`].
    #[ink(event)]
    pub struct GameEnded {
        /// The account that ended the game.
        ender: AccountId,
    }

    /// The game ended and the winner destroyed the contract.
    #[ink(event)]
    pub struct GameDestroyed {
        /// The winning player who is also the one who destroyed the contract.
        winner: Player,
    }

    impl SquinkSplash {
        /// Create a new game.
        ///
        /// - `dimensions`: (width, height) of the board.
        /// - `buy_in`: The amount of balance each player needs to submit in order to play.
        /// - `forming_rounds`: Number of blocks that needs to pass until anyone can start the game.
        /// - `rounds`: The number of blocks a game can be played for.
        /// - `score_multiplier`: The higher the more score you get per field.
        /// - `gas_per_round`: The amount of gas each player can use. Unused gas is carried over to the next round.
        #[ink(constructor)]
        pub fn new(
            dimensions: Field,
            buy_in: Balance,
            forming_rounds: u32,
            rounds: u32,
        ) -> Self {
            let mut ret = Self {
                state: State::Forming {
                    earliest_start: Self::env()
                        .block_number()
                        .saturating_add(forming_rounds),
                },
                board: Default::default(),
                dimensions,
                players: Default::default(),
                buy_in,
                rounds,
                last_turn: Default::default(),
                opener: Self::env().caller(),
            };
            ret.players.set(&Vec::new());
            ret
        }

        /// When the game is in finished the contract can be deleted by the winner.
        #[ink(message)]
        pub fn destroy(&mut self) -> Result<(), GameError> {
            if let State::Finished { winner } = self.state {
                winner
                    .eq(&Self::env().caller())
                    .then_some(())
                    .ok_or(GameError::OnlyWinnerIsAllowedToDestroyTheContract)?;

                let winner = {
                    let players = self.players();
                    let winning_idx = Self::find_player(&winner, &players)
                        .map_err(|_| GameError::TheWinnerIsNotAPlayer)?;

                    players.into_iter().nth(winning_idx).unwrap()
                };
                let winner_id = winner.id;
                Self::env().emit_event(GameDestroyed { winner });
                Self::env().terminate_contract(winner_id);
            } else {
                return Err(GameError::OnlyFinishedGamesCanBeDestroyed);
            }
        }

        /// Anyone can start the game when `earliest_start` is reached.
        #[ink(message)]
        pub fn start_game(&mut self) -> Result<(), GameError> {
            if Self::env().caller() != self.opener {
                return Err(GameError::OnlyAdminCanStartTheGame);
            }

            if let State::Forming { earliest_start } = self.state {
                (Self::env().block_number() >= earliest_start)
                    .then_some(())
                    .ok_or(GameError::GameCantBeStartedYet)?;
            } else {
                return Err(GameError::GameAlreadyStarted);
            };
            let players = self.players();

            let res = !players.is_empty();
            res.then_some(())
                .ok_or(GameError::YouNeedAtLeastOnePlayer)?;

            self.state = State::Running { rounds_played: 0 };

            // We pretend that there was already a turn in this block so that no
            // turns can be submitted in the same block as when the game is started.
            self.last_turn.set(&Self::env().block_number());
            Self::env().emit_event(GameStarted {
                starter: Self::env().caller(),
            });

            Ok(())
        }

        /// When enough time has passed, no new turns can be submitted.
        /// Then anybody may call this function to end the game and
        /// trigger the payout to the winner.
        #[ink(message)]
        pub fn end_game(&mut self) -> Result<(), GameError> {
            let res = !self.is_running();
            res.then_some(())
                .ok_or(GameError::GameCantBeEndedOrHasAlreadyEnded)?;

            let players = self.players();
            let winner = players
                .iter()
                .min_by_key(|p| p.scoring_order())
                .ok_or(GameError::WeOnlyAllowStartingTheGameWithAtLeastOnePlayer)?
                .id;

            // Give the pot to the winner
            Self::env().transfer(
                winner,
                Balance::from(players.len() as u32).saturating_mul(self.buy_in),
            )?;

            self.state = State::Finished { winner };
            Self::env().emit_event(GameEnded {
                ender: Self::env().caller(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn reset_game(&mut self) -> Result<(), GameError> {
            match self.state {
                State::Finished { .. } => {
                    self.state = State::Forming {
                        earliest_start: Self::env().block_number(),
                    };
                    for x in 0..self.dimensions.x {
                        for y in 0..self.dimensions.y {
                            self.board.remove(self.idx(&Field { x, y }).unwrap());
                        }
                    }
                    self.players.set(&Vec::new());
                    self.last_turn.set(&0);
                    Ok(())
                }
                _ => Err(GameError::OnlyFinishedGameCanBeReset),
            }
        }

        /// Add a new player to the game. Only allowed while the game has not started.
        #[ink(message, payable)]
        pub fn register_player(
            &mut self,
            id: AccountId,
            name: String,
        ) -> Result<(), GameError> {
            matches!(self.state, State::Forming { .. })
                .then_some(())
                .ok_or(GameError::PlayersCanOnlyBeRegisteredInTheFormingPhase)?;

            ALLOWED_NAME_SIZES
                .contains(&name.len())
                .then_some(())
                .ok_or(GameError::InvalidLengthForName)?;

            self.buy_in
                .eq(&Self::env().transferred_value())
                .then_some(())
                .ok_or(GameError::WrongBuyIn)?;

            let mut players = self.players();

            players
                .len()
                .lt(&PLAYER_LIMIT)
                .then_some(())
                .ok_or(GameError::MaximumPlayerCountReached)?;

            match Self::find_player(&id, &players) {
                Err(idx) => {
                    let res = !players.iter().any(|p| p.name == name);
                    res.then_some(()).ok_or(GameError::ThisNameIsAlreadyTaken)?;

                    players.insert(
                        idx,
                        Player {
                            id,
                            name,
                            gas_used: 0,
                            score: 0,
                        },
                    );
                    self.players.set(&players);
                    Self::env().emit_event(PlayerRegistered { player: id });
                }
                Ok(_) => {
                    return Err(GameError::PlayerAlreadyRegistered);
                }
            }
            Ok(())
        }

        /// This is the actual game loop.
        ///
        /// It can be called by anyone and triggers at most one turn
        /// of the game per block.
        #[ink(message)]
        pub fn submit_turn(&mut self) -> Result<(), GameError> {
            self.is_running()
                .then_some(())
                .ok_or(GameError::GameCannotBeEndedOrHasAlreadyEnded)?;

            let mut players = self.players();

            let State::Running { rounds_played } = &mut self.state else {
                return Err(GameError::ThisGameDoesNotAcceptTurnsRightNow);
            };

            // Only one turn per block
            // We need to write this to storage because of reentrancy: The called contract
            // could call this function again and do another turn in the same block.
            let current_block = Self::env().block_number();
            let last_turn = self
                .last_turn
                .get()
                .ok_or(GameError::ValueWasNotSetWhenStartingTheGame)?;

            last_turn
                .lt(&current_block)
                .then_some(())
                .ok_or(GameError::TurnWasAlreadySubmittedForThisBlock)?;

            self.last_turn.set(&current_block);

            // We need to cache this as we can't accessed players in the loop.
            let num_players = players.len();

            // Batching is needed so we don't call all the players every round
            // (because of the gas limit).
            let current_round = *rounds_played;
            *rounds_played = rounds_played.saturating_add(1);
            let num_batches = Self::calc_num_batches(num_players);
            let current_batch = current_round.rem_euclid(num_batches);

            // Information about the game is passed to players.
            let mut game_info = GameInfo {
                rounds_played: current_round,
                gas_left: 0,
                player_scores: players
                    .iter()
                    .map(|player| (player.name.clone(), player.score))
                    .collect(),
            };

            for (idx, player) in players.iter_mut().enumerate() {
                if (idx as u32).rem_euclid(num_batches) != current_batch {
                    continue;
                }

                // Stop calling a contract that has no gas left.
                let gas_limit = Self::calc_gas_limit(num_players);
                let gas_left = Self::calc_gas_budget(gas_limit, self.rounds)
                    .saturating_sub(player.gas_used);
                if gas_left == 0 {
                    Self::env().emit_event(TurnTaken {
                        player: player.id,
                        outcome: TurnOutcome::BudgetExhausted,
                    });
                    continue;
                }
                game_info.gas_left = gas_left;

                // We need to call with reentrancy enabled to allow those
                // contracts to query us.
                let call = build_call::<DefaultEnvironment>()
                    .call_type(Call::new(player.id))
                    .gas_limit(gas_limit)
                    .exec_input(
                        ExecutionInput::new(Selector::from([0x00; 4]))
                            .push_arg(&game_info),
                    )
                    .call_flags(CallFlags::default().set_allow_reentry(true))
                    .returns::<Option<Field>>();

                let gas_before = Self::env().gas_left();
                let turn = call.try_invoke();
                let gas_used = gas_before.saturating_sub(Self::env().gas_left());

                // We continue even if the contract call fails. If the contract
                // doesn't conform it is the players fault. No second tries.
                let outcome = match turn {
                    Ok(Ok(Some(turn))) if self.idx(&turn).is_some() => {
                        let idx = self.idx(&turn).unwrap();
                        // Player tried to make a turn: charge gas.
                        player.gas_used = player.gas_used.saturating_add(gas_used);
                        if !self.is_valid_coord(&turn) {
                            TurnOutcome::OutOfBounds { turn }
                        } else if let Some(entry) = self.board.get(idx) {
                            TurnOutcome::Occupied {
                                turn,
                                player: entry.owner,
                            }
                        } else {
                            self.board.insert(
                                idx,
                                &FieldEntry {
                                    owner: player.id,
                                    claimed_at: current_round,
                                },
                            );
                            player.score = player.score.saturating_add(u64::from(
                                current_round.saturating_add(1),
                            ));
                            TurnOutcome::Success { turn }
                        }
                    }
                    Ok(Ok(None)) => TurnOutcome::NoTurn,
                    _err => {
                        // Player gets charged gas for failing.
                        player.gas_used = player.gas_used.saturating_add(gas_used);
                        debug_println!("Contract failed to make a turn: {:?}", _err);
                        TurnOutcome::BrokenPlayer
                    }
                };

                Self::env().emit_event(TurnTaken {
                    player: player.id,
                    outcome,
                });
            }

            Self::env().emit_event(RoundIncremented {
                rounds_played: current_round.saturating_add(1),
            });

            self.players.set(&players);
            Ok(())
        }

        /// The buy-in amount to register a player.
        #[ink(message)]
        pub fn buy_in_amount(&self) -> Balance {
            self.buy_in
        }

        /// The total amount of rounds this game is to be played for.
        #[ink(message)]
        pub fn total_rounds(&self) -> u32 {
            self.rounds
        }

        /// How much gas each player is allowed to use per round.
        #[ink(message)]
        pub fn gas_limit(&self) -> u64 {
            Self::calc_gas_limit(self.players().len())
        }

        /// Describes into many groups the players should be partitioned.
        ///
        /// How often [`submit_turn`] needs to be called until all players
        /// made a turn.
        #[ink(message)]
        pub fn num_batches(&self) -> u32 {
            Self::calc_num_batches(self.players().len())
        }

        /// How much gas each player is allowed to consume for the whole game.
        #[ink(message)]
        pub fn gas_budget(&self) -> u64 {
            Self::calc_gas_budget(self.gas_limit(), self.rounds)
        }

        /// The current game state.
        #[ink(message)]
        pub fn state(&self) -> State {
            self.state.clone()
        }

        /// Returns `true` if the game is running.
        #[ink(message)]
        pub fn is_running(&self) -> bool {
            if let State::Running { rounds_played, .. } = self.state {
                rounds_played < self.rounds
            } else {
                false
            }
        }

        /// List of all players sorted by score and gas costs.
        #[ink(message)]
        pub fn players_sorted(&self) -> Vec<Player> {
            let mut players = self.players();
            players.sort_unstable_by_key(|player| player.scoring_order());
            players
        }

        /// Returns the dimensions of the board.
        #[ink(message)]
        pub fn dimensions(&self) -> Field {
            self.dimensions
        }

        /// Returns the value (owner) of the supplied field.
        #[ink(message)]
        pub fn field(&self, coord: Field) -> Option<FieldEntry> {
            self.idx(&coord).and_then(|idx| self.board.get(idx))
        }

        /// Returns the complete board.
        ///
        /// The index into the vector is calculated as `x + y * width`.
        #[ink(message)]
        pub fn board(&self) -> Vec<Option<FieldEntry>> {
            self.board_iter().collect()
        }

        fn calc_gas_limit(num_players: usize) -> u64 {
            (GAS_LIMIT_ALL_PLAYERS
                .saturating_mul(u64::from(Self::calc_num_batches(num_players))))
                .checked_div(num_players as u64)
                .unwrap_or(0)
        }

        fn calc_num_batches(num_players: usize) -> u32 {
            if num_players > 30 {
                2
            } else {
                1
            }
        }

        fn calc_gas_budget(gas_limit: u64, num_rounds: u32) -> u64 {
            gas_limit.saturating_mul(u64::from(num_rounds).saturating_div(4))
        }

        fn players(&self) -> Vec<Player> {
            self.players
                .get()
                .expect("Initial value is set in constructor.")
        }

        fn board_iter(&self) -> impl Iterator<Item=Option<FieldEntry>> + '_ {
            (0..self.dimensions.y).flat_map(move |y| {
                (0..self.dimensions.x).map(move |x| self.field(Field { x, y }))
            })
        }

        fn find_player(id: &AccountId, players: &[Player]) -> Result<usize, usize> {
            players.binary_search_by_key(id, |player| player.id)
        }

        fn idx(&self, coord: &Field) -> Option<u32> {
            coord
                .y
                .checked_mul(self.dimensions.x)
                .and_then(|val| val.checked_add(coord.x))
        }

        fn is_valid_coord(&self, coord: &Field) -> bool {
            self.idx(coord)
                .map(|val| val < self.dimensions.len())
                .unwrap_or(false)
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod tests {
    use ink_e2e::{
        alice,
        ContractsBackend,
    };
    use common::Field;
    use simple_player::TestPlayerRef;

    #[ink_e2e::test(additional_contracts = "../simple-player/Cargo.toml")]
    async fn e2e_game<Client: E2EBackend>(mut client: Client) {
        let alice = alice();
        let dimensions = Field { x: 10, y: 10 };
        let forming_rounds = 0;
        let rounds = 20;
        let buy_in = 0;

        let player_alex = client
            .instantiate(
                "simple-player",
                &alice,
                TestPlayerRef::new((dimensions.x, dimensions.y), 7),
                0,
                None,
            )
            .await
            .unwrap();

        let player_bob = client
            .instantiate(
                "simple-player",
                &alice,
                TestPlayerRef::new((dimensions.x, dimensions.y), 3),
                0,
                None,
            )
            .await
            .unwrap();

        let game = client
            .instantiate(
                "squink_splash",
                &alice,
                GameRef::new(dimensions, buy_in, forming_rounds, rounds),
                0,
                None,
            )
            .await
            .unwrap();

        let mut game_call = game.call::<Game>();

        client
            .call(
                &alice,
                &game_call.register_player(player_alex.account_id, "Alex".into()),
                0,
                None,
                None,
            )
            .await
            .unwrap();

        client
            .call(
                &alice,
                &game_call.register_player(player_bob.account_id, "Bob".into()),
                0,
                None,
                None,
            )
            .await
            .unwrap();

        client
            .call(&alice, &game_call.start_game(), 0, None, None)
            .await
            .unwrap();

        let state = client
            .call(&alice, &game_call.state(), 0, None, None)
            .await
            .unwrap()
            .return_value();
        println!("state: {:?}", state);

        for _ in 0..rounds {
            client
                .call(&alice, &game_call.submit_turn(), 0, None, None)
                .await
                .unwrap();

            let players = client
                .call(&alice, &game_call.players_sorted(), 0, None, None)
                .await
                .unwrap()
                .return_value();

            println!("players: {:?}", players);
        }

        let state = client
            .call(&alice, &game_call.state(), 0, None, None)
            .await
            .unwrap()
            .return_value();
        println!("state: {:?}", state);

        client
            .call(&alice, &game_call.end_game(), 0, None, None)
            .await
            .unwrap();
    }
}
