//! I learned that 5x5 chess variants were a thing and have since changed the
//! starting layout to match Gardner chess:
//!
//! Martin Gardner (1991). The Unexpected Hanging and Other Mathematical
//! Diversions (Reprint ed.)
//!
//! Also see <https://en.m.wikipedia.org/wiki/Minichess>.
//!
//! ## A chess variant with a 5x5 grid.
//!
//! Like normal chess, moving into check is illegal and the victory condition is
//! a check-mate. Castling and pawn double-move are disallowed.
//! Pawns promote into Queens with no player choice.
//!
//! This is incredibly stupid and shouldn't be taken seriously, but it can
//! probably be adapted to work with a normal-sized chessboard.

use std::ptr;

use super::{Player, PlayerState};
use crate::buttons::prelude::*;

mod game;
#[cfg(test)]
mod tests;

use game::{Board, N, N_U8, Piece, Pos, new_board};

#[derive(Clone, Serialize, Deserialize)]
pub struct View {
    players: PlayerState,
    board: Board,
    action: Action,
}

utils::impl_debug!(struct View: { players, action, .. });

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Action {
    Idle,
    Selected(Pos),
    Move(Pos, Pos),
}

fn flat_index(pos: Pos) -> u16 {
    u16::from(pos.x) * u16::from(N_U8) + u16::from(pos.y)
}

impl View {
    pub fn new(players: [UserId; 2]) -> Self {
        Self {
            players: PlayerState::new(players),
            action: Action::Idle,
            board: new_board!(
                [b R, b k, b B, b Q, b K]
                [b p, b p, b p, b p, b p]
                [- -, - -, - -, - -, - -]
                [w p, w p, w p, w p, w p]
                [w R, w k, w B, w Q, w K]
            ),
        }
    }

    fn board_components<'a>(&mut self, data: &'a HBotData, label: String) -> CreateComponents<'a> {
        let mut components = CreateComponents::with_capacity(N + 2);
        components.push(CreateTextDisplay::new(label));
        components.push(CreateSeparator::new(true));

        let moves = match self.action {
            Action::Selected(pos) => self.board.get(pos).copied().flatten().map(|t| {
                (
                    t.piece.get_move().target_mask(&self.board, pos, t.player),
                    pos,
                )
            }),
            _ => None,
        };

        for y in 0..N_U8 {
            let mut row = Vec::with_capacity(N);
            for x in 0..N_U8 {
                let pos = Pos::new(x, y);
                let tile = self.board.get(pos).expect("must be in range");

                let icon = match tile {
                    Some(t) => t.emoji(data),
                    None => data.app_emojis().empty(),
                };

                let (style, action) = match (tile, &moves) {
                    (Some(t), _) if t.player == self.players.turn => {
                        (ButtonStyle::Primary, Action::Selected(pos))
                    },
                    (t, Some((mask, src))) if mask.get(pos) == Some(&true) => (
                        t.map_or(ButtonStyle::Success, |_| ButtonStyle::Danger),
                        Action::Move(*src, pos),
                    ),
                    _ => (ButtonStyle::Secondary, Action::Idle),
                };

                row.push(
                    if action == Action::Idle {
                        use crate::modules::core::buttons::Noop;

                        #[expect(clippy::cast_possible_truncation)]
                        let key = ptr::from_ref(&self.action).addr() as u16;
                        let value = flat_index(pos);
                        CreateButton::new(Noop::new(key, value).to_custom_id()).disabled(true)
                    } else {
                        self.new_button(|s| &mut s.action, action, |_| flat_index(pos))
                    }
                    .emoji(icon.clone())
                    .style(style),
                );
            }

            components.push(CreateActionRow::buttons(row));
        }

        components
    }

    fn no_act_board_components<'a>(
        &self,
        data: &'a HBotData,
        label: String,
    ) -> CreateComponents<'a> {
        let mut components = CreateComponents::with_capacity(N + 2);
        components.push(CreateTextDisplay::new(label));
        components.push(CreateSeparator::new(true));

        for y in 0..N_U8 {
            let mut row = Vec::with_capacity(N);
            for x in 0..N_U8 {
                let pos = Pos::new(x, y);
                let tile = self.board.get(pos).expect("must be in range");

                let icon = match tile {
                    Some(t) => t.emoji(data),
                    None => data.app_emojis().empty(),
                };

                row.push({
                    use crate::modules::core::buttons::Noop;

                    let value = flat_index(pos);
                    CreateButton::new(Noop::new(0, value).to_custom_id())
                        .disabled(true)
                        .emoji(icon.clone())
                        .style(ButtonStyle::Secondary)
                });
            }

            components.push(CreateActionRow::buttons(row));
        }

        components
    }

    fn is_active_player_in_check(&self) -> bool {
        let player = self.players.turn;
        self.board
            .king_at(player)
            .is_some_and(|king_at| self.board.is_player_in_check(player, king_at))
    }

    fn is_inactive_player_in_checkmate(&self) -> bool {
        let player = self.players.turn.next();
        self.board
            .king_at(player)
            .is_none_or(|king_at| self.board.is_player_in_checkmate(player, king_at))
    }

    pub fn create_next_reply(mut self, data: &HBotData) -> CreateReply<'_> {
        let description = match self.players.turn {
            Player::P1 => format!(
                "> **⬜ {}**\n-# ⬛ {}",
                self.players.p1.mention(),
                self.players.p2.mention(),
            ),
            Player::P2 => format!(
                "-# ⬜ {}\n> **⬛ {}**",
                self.players.p1.mention(),
                self.players.p2.mention(),
            ),
        };

        let components = self.board_components(data, description);
        let container = CreateContainer::new(components).accent_color(data.config().embed_color);

        CreateReply::new()
            .components_v2(components![container])
            .allowed_mentions(CreateAllowedMentions::new())
    }

    fn create_win_reply(self, data: &HBotData) -> CreateReply<'_> {
        let winner_id = self.players.turn_user_id();

        let description = format!(
            "## {} wins!\n\
             -# ⬜ {}\n\
             -# ⬛ {}",
            winner_id.mention(),
            self.players.p1.mention(),
            self.players.p2.mention(),
        );

        let components = self.no_act_board_components(data, description);
        let container = CreateContainer::new(components).accent_color(data.config().embed_color);

        CreateReply::new()
            .components_v2(components![container])
            .allowed_mentions(CreateAllowedMentions::new())
    }
}

button_value!(View, 20);
impl ButtonReply for View {
    async fn reply(mut self, ctx: ButtonContext<'_>) -> Result {
        self.players.check_turn(&ctx)?;

        if let Action::Move(src, dst) = self.action {
            // take the piece in the source slot
            let mut src = self
                .board
                .get_mut(src)
                .context("invalid move src pos")?
                .take();

            // check whether this is a pawn that has reached the enemy home row
            if let Some(src) = &mut src {
                anyhow::ensure!(src.player == self.players.turn, "should select own piece");

                // always go for queen promotion
                if src.piece == Piece::Pawn && game::is_home_row(dst, self.players.turn.next()) {
                    src.piece = Piece::Queen;
                }
            }

            // place the new piece down
            *self.board.get_mut(dst).context("invalid move dst pos")? = src;

            // check for invalid moves
            if self.is_active_player_in_check() {
                anyhow::bail!(HArgError::new("That move would put you in check."));
            }

            // check for checkmate
            if self.is_inactive_player_in_checkmate() {
                let reply = self.create_win_reply(ctx.data);
                return ctx.edit(reply.into()).await;
            }

            self.action = Action::Idle;
            self.players.next_turn();
        }

        let reply = self.create_next_reply(ctx.data);
        ctx.edit(reply.into()).await
    }
}
