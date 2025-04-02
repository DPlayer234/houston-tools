use std::fmt;

use crate::buttons::prelude::*;
use crate::helper::discord::id_as_u64;

pub mod chess;
pub mod rock_paper_scissors;
pub mod tic_tac_toe;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Player {
    P1,
    P2,
}

impl Player {
    fn next(self) -> Self {
        match self {
            Self::P1 => Self::P2,
            Self::P2 => Self::P1,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PlayerState {
    #[serde(with = "id_as_u64")]
    p1: UserId,
    #[serde(with = "id_as_u64")]
    p2: UserId,
    turn: Player,
}

impl PlayerState {
    fn new(players: [UserId; 2]) -> Self {
        Self {
            p1: players[0],
            p2: players[1],
            turn: Player::P1,
        }
    }

    fn next_turn(&mut self) {
        self.turn = self.turn.next();
    }

    fn user_id(&self, player: Player) -> UserId {
        match player {
            Player::P1 => self.p1,
            Player::P2 => self.p2,
        }
    }

    fn turn_user_id(&self) -> UserId {
        self.user_id(self.turn)
    }

    fn check_turn(&self, ctx: &ButtonContext<'_>) -> Result<(), HArgError> {
        let interacting = ctx.interaction.user.id;
        let current_turn = self.turn_user_id();
        if interacting == current_turn {
            Ok(())
        } else if interacting == self.p1 || interacting == self.p2 {
            Err(HArgError::new(format!(
                "It's {}'s turn.",
                current_turn.mention()
            )))
        } else {
            Err(HArgError::new_const("You're not part of this game."))
        }
    }
}

impl fmt::Debug for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.turn == Player::P1 {
            write!(f, "([{}] vs {})", self.p1, self.p2)
        } else {
            write!(f, "({} vs [{}])", self.p1, self.p2)
        }
    }
}
