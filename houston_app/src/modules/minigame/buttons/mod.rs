use serenity::model::id::UserId;

use crate::buttons::ButtonContext;
use crate::data::HArgError;
use crate::helper::discord::id_array_as_u64;

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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct PlayerState {
    #[serde(with = "id_array_as_u64")]
    ids: [UserId; 2],
    turn: Player,
}

impl PlayerState {
    fn new(players: [UserId; 2]) -> Self {
        Self {
            ids: players,
            turn: Player::P1,
        }
    }

    fn next_turn(&mut self) {
        self.turn = self.turn.next();
    }

    fn user_id(&self, player: Player) -> UserId {
        match player {
            Player::P1 => self.ids[0],
            Player::P2 => self.ids[1],
        }
    }

    fn turn_user_id(&self) -> UserId {
        self.user_id(self.turn)
    }

    fn check_turn(&self, ctx: &ButtonContext<'_>) -> anyhow::Result<()> {
        let current_turn = self.turn_user_id();
        if ctx.interaction.user.id != current_turn {
            anyhow::bail!(HArgError::new(format!("It's <@{current_turn}>'s turn.")));
        }

        Ok(())
    }
}
