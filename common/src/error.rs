use ink::env::Error;
use scale::{Decode, Encode};
use ink::prelude::string::String;
use ink::prelude::format;

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum GameError {
    OnlyAdminCanStartTheGame,
    OnlyFinishedGamesCanBeDestroyed,
    GameAlreadyStarted,
    PlayerAlreadyRegistered,
    PlayersCanOnlyBeRegisteredInTheFormingPhase,
    InvalidLengthForName,
    WrongBuyIn,
    MaximumPlayerCountReached,
    ThisNameIsAlreadyTaken,
    GameCannotBeEndedOrHasAlreadyEnded,
    ThisGameDoesNotAcceptTurnsRightNow,
    TurnWasAlreadySubmittedForThisBlock,
    GameCantBeStartedYet,
    YouNeedAtLeastOnePlayer,
    GameCantBeEndedOrHasAlreadyEnded,
    OnlyWinnerIsAllowedToDestroyTheContract,
    OnlyFinishedGameCanBeReset,
    TheWinnerIsNotAPlayer,
    WeOnlyAllowStartingTheGameWithAtLeastOnePlayer,
    InkEnvError(String),
    ValueWasNotSetWhenStartingTheGame,
}

impl From<Error> for GameError {
    fn from(why: Error) -> Self {
        Self::InkEnvError(format!("{:?}", why))
    }
}
