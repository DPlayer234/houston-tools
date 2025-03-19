use super::{Player, PlayerState};
use crate::buttons::prelude::*;
use crate::helper::discord::unicode_emoji;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    players: PlayerState,
    board: [[Option<Player>; 3]; 3],
}

fn icon(data: &HBotData, p: Option<Player>) -> ReactionType {
    match p {
        Some(Player::P1) => unicode_emoji("❌"),
        Some(Player::P2) => unicode_emoji("⭕"),
        None => data.app_emojis().empty().clone(),
    }
}

const fn flat_index(x: usize, y: usize) -> usize {
    x + y * 3
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
            Self::DiagTopRight => 2usize.wrapping_sub(x) == y,
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
            ($e:expr, $l:expr) => {{
                let (p1, p2) = $e;
                if p1 == 3 {
                    return Some((Player::P1, $l));
                }
                if p2 == 3 {
                    return Some((Player::P2, $l));
                }
            }};
        }

        // by column
        for x in 0..3 {
            check!(counts(self.board[x]), WinLine::Column(x));
        }

        // by row
        for y in 0..3 {
            check!(
                counts([self.board[0][y], self.board[1][y], self.board[2][y]]),
                WinLine::Row(y)
            );
        }

        // diagonals
        check!(
            counts([self.board[0][0], self.board[1][1], self.board[2][2]]),
            WinLine::DiagTopLeft
        );
        check!(
            counts([self.board[2][0], self.board[1][1], self.board[0][2]]),
            WinLine::DiagTopRight
        );

        None
    }

    fn board_buttons<'a, F>(
        &mut self,
        data: &'a HBotData,
        current: Player,
        modify: F,
    ) -> Vec<CreateActionRow<'a>>
    where
        F: Fn(CreateButton<'a>, usize, usize, Option<Player>) -> CreateButton<'a>,
    {
        let mut components = Vec::with_capacity(3);

        for y in 0..3 {
            let mut row = Vec::with_capacity(3);
            for x in 0..3 {
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
                "> **❌ <@{}>**\n-# ⭕ <@{}>",
                self.players.p1, self.players.p2
            ),
            Player::P2 => format!(
                "-# ❌ <@{}>\n> **⭕ <@{}>**",
                self.players.p1, self.players.p2
            ),
        };

        let embed = CreateEmbed::new()
            .description(description)
            .color(data.config().embed_color);

        let components = self.board_buttons(data, self.players.turn, |b, _, _, s| {
            b.disabled(s.is_some())
        });

        CreateReply::new().embed(embed).components(components)
    }

    fn create_win_reply(
        mut self,
        data: &HBotData,
        winner: Player,
        win_line: WinLine,
    ) -> CreateReply<'_> {
        let winner_id = self.players.user_id(winner);

        let description = format!(
            "## <@{winner_id}> wins!\n\
             -# ❌ <@{p1}>\n\
             -# ⭕ <@{p2}>",
            p1 = self.players.p1,
            p2 = self.players.p2,
        );

        let embed = CreateEmbed::new()
            .description(description)
            .color(data.config().embed_color);

        let components = self.board_buttons(data, Player::P1, |b, x, y, _| {
            b.disabled(true).style(if win_line.is_match(x, y) {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            })
        });

        CreateReply::new().embed(embed).components(components)
    }

    fn create_draw_reply(mut self, data: &HBotData) -> CreateReply<'_> {
        let embed = format!(
            "## Draw!\n\
             -# ❌ <@{p1}>\n\
             -# ⭕ <@{p2}>",
            p1 = self.players.p1,
            p2 = self.players.p2,
        );

        let description = CreateEmbed::new()
            .description(embed)
            .color(data.config().embed_color);

        let components = self.board_buttons(data, Player::P1, |b, _, _, _| {
            b.disabled(true).style(ButtonStyle::Danger)
        });

        CreateReply::new().embed(description).components(components)
    }
}

impl ButtonArgsReply for View {
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
