use super::{Player, PlayerState};
use crate::buttons::prelude::*;
use crate::helper::discord::components::{CreateComponents, components};
use crate::helper::discord::unicode_emoji;

const N: usize = 3;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    players: PlayerState,
    board: [[Option<Player>; N]; N],
}

fn icon(data: &HBotData, p: Option<Player>) -> ReactionType {
    match p {
        Some(Player::P1) => unicode_emoji("❌"),
        Some(Player::P2) => unicode_emoji("⭕"),
        None => data.app_emojis().empty().clone(),
    }
}

const fn flat_index(x: usize, y: usize) -> usize {
    x + y * N
}

#[derive(Debug, Clone, Copy)]
enum WinLine {
    Row(usize),
    Column(usize),
    DiagTopLeft,
    DiagTopRight,
}

impl WinLine {
    fn is_match(self, x: usize, y: usize) -> bool {
        match self {
            Self::Row(y_actual) => y == y_actual,
            Self::Column(x_actual) => x == x_actual,
            Self::DiagTopLeft => x == y,
            Self::DiagTopRight => (N - 1).wrapping_sub(x) == y,
        }
    }
}

impl View {
    pub fn new(players: [UserId; 2]) -> Self {
        Self {
            players: PlayerState::new(players),
            board: Default::default(),
        }
    }

    fn is_full(&self) -> bool {
        !self.board.as_flattened().contains(&None)
    }

    fn winner(&self) -> Option<(Player, WinLine)> {
        fn counts(iter: impl IntoIterator<Item = Option<Player>>) -> (usize, usize) {
            let mut counts = (0, 0);
            for item in iter {
                match item {
                    Some(Player::P1) => counts.0 += 1,
                    Some(Player::P2) => counts.1 += 1,
                    None => {},
                }
            }

            counts
        }

        macro_rules! check {
            ($e:expr, $l:expr) => {
                match $e {
                    (N, _) => return Some((Player::P1, $l)),
                    (_, N) => return Some((Player::P2, $l)),
                    _ => {},
                }
            };
        }

        // by column
        for x in 0..N {
            check!(counts(self.board[x]), WinLine::Column(x));
        }

        // by row
        for y in 0..N {
            check!(counts(self.board.iter().map(|r| r[y])), WinLine::Row(y));
        }

        // diagonals
        check!(
            counts((0..N).map(|n| self.board[n][n])),
            WinLine::DiagTopLeft
        );
        check!(
            counts((0..N).map(|n| self.board[n][N - n - 1])),
            WinLine::DiagTopRight
        );

        None
    }

    fn board_components<'a, F>(
        &mut self,
        data: &'a HBotData,
        label: String,
        current: Player,
        modify: F,
    ) -> CreateComponents<'a>
    where
        F: Fn(CreateButton<'a>, usize, usize, Option<Player>) -> CreateButton<'a>,
    {
        let mut components = CreateComponents::with_capacity(N + 2);
        components.push(label);
        components.push(CreateSeparator::new(true));

        for y in 0..N {
            let mut row = Vec::with_capacity(N);
            for x in 0..N {
                let state = self.board[x][y];

                #[expect(clippy::cast_possible_truncation)]
                let button = self
                    .new_button(
                        |s| &mut s.board[x][y],
                        Some(current),
                        |_| flat_index(x, y) as u16,
                    )
                    .emoji(icon(data, state))
                    .style(ButtonStyle::Secondary);

                row.push(modify(button, x, y, state));
            }

            components.push(CreateActionRow::buttons(row));
        }

        components
    }

    pub fn create_next_reply(mut self, data: &HBotData) -> CreateReply<'_> {
        let description = match self.players.turn {
            Player::P1 => format!(
                "> **❌ {}**\n-# ⭕ {}",
                self.players.p1.mention(),
                self.players.p2.mention(),
            ),
            Player::P2 => format!(
                "-# ❌ {}\n> **⭕ {}**",
                self.players.p1.mention(),
                self.players.p2.mention(),
            ),
        };

        let components =
            self.board_components(data, description, self.players.turn, |b, _, _, s| {
                b.disabled(s.is_some())
            });

        let components =
            components![CreateContainer::new(components).accent_color(data.config().embed_color)];

        CreateReply::new()
            .components_v2(components)
            .allowed_mentions(CreateAllowedMentions::new())
    }

    fn create_win_reply(
        mut self,
        data: &HBotData,
        winner: Player,
        win_line: WinLine,
    ) -> CreateReply<'_> {
        let winner_id = self.players.user_id(winner);

        let description = format!(
            "## {} wins!\n\
             -# ❌ {}\n\
             -# ⭕ {}",
            winner_id.mention(),
            self.players.p1.mention(),
            self.players.p2.mention(),
        );

        let components = self.board_components(data, description, Player::P1, |b, x, y, _| {
            b.disabled(true).style(if win_line.is_match(x, y) {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            })
        });

        let components =
            components![CreateContainer::new(components).accent_color(data.config().embed_color)];

        CreateReply::new()
            .components_v2(components)
            .allowed_mentions(CreateAllowedMentions::new())
    }

    fn create_draw_reply(mut self, data: &HBotData) -> CreateReply<'_> {
        let description = format!(
            "## Draw!\n\
             -# ❌ {}\n\
             -# ⭕ {}",
            self.players.p1.mention(),
            self.players.p2.mention(),
        );

        let components = self.board_components(data, description, Player::P1, |b, _, _, _| {
            b.disabled(true).style(ButtonStyle::Secondary)
        });

        let components =
            components![CreateContainer::new(components).accent_color(data.config().embed_color)];

        CreateReply::new()
            .components_v2(components)
            .allowed_mentions(CreateAllowedMentions::new())
    }
}

button_value!(View, 18);
impl ButtonReply for View {
    async fn reply(mut self, ctx: ButtonContext<'_>) -> Result {
        self.players.check_turn(&ctx)?;

        let reply = if let Some((winner, line)) = self.winner() {
            self.create_win_reply(ctx.data, winner, line)
        } else if self.is_full() {
            self.create_draw_reply(ctx.data)
        } else {
            self.players.next_turn();
            self.create_next_reply(ctx.data)
        };

        ctx.edit(reply.into()).await
    }
}
