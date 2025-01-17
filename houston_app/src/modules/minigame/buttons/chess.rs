//! I learned that 5x5 chess variants were a thing and have since changed the
//! starting layout to match Gardner chess:
//!
//! Martin Gardner (1991). The Unexpected Hanging and Other Mathematical
//! Diversions (Reprint ed.)
//!
//! Also see <https://en.m.wikipedia.org/wiki/Minichess>.
//!
//! The original comment is below:
//!
//! ## A chess variant with a 5x5 grid.
//!
//! Like normal chess, moving into check is illegal and the victory condition is
//! a check-mate. Castling and pawn double-move are disallowed.
//! Pawns promote into Queens with no player choice.
//!
//! This is incredibly stupid and shouldn't be taken seriously, but it can
//! probably be adapted to work with a normal-sized chessboard.

use std::{fmt, ptr};

use super::{Player, PlayerState};
use crate::buttons::prelude::*;

const N: usize = 5;

type Board = Grid<Option<Tile>>;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
struct Pos {
    x: u8,
    y: u8,
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
impl Pos {
    fn new_trunc(x: usize, y: usize) -> Self {
        Self {
            x: x as u8,
            y: y as u8,
        }
    }

    fn add_x(self, x: i8) -> Self {
        Self {
            x: self.x.wrapping_add(x as u8),
            y: self.y,
        }
    }

    fn add_y(self, y: i8) -> Self {
        Self {
            x: self.x,
            y: self.y.wrapping_add(y as u8),
        }
    }

    fn add_offset(self, offset: Offset) -> Self {
        Self {
            x: self.x.wrapping_add(offset.x as u8),
            y: self.y.wrapping_add(offset.y as u8),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Offset {
    x: i8,
    y: i8,
}

impl Offset {
    const fn new(x: i8, y: i8) -> Self {
        Self { x, y }
    }
}

#[derive(Default, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
struct Grid<T> {
    array: [[T; N]; N],
}

impl<T> Grid<T> {
    fn get(&self, pos: Pos) -> Option<&T> {
        self.array.get(usize::from(pos.x))?.get(usize::from(pos.y))
    }

    fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        self.array
            .get_mut(usize::from(pos.x))?
            .get_mut(usize::from(pos.y))
    }
}

impl<T> fmt::Debug for Grid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grid").finish_non_exhaustive()
    }
}

// Black (P2) starts at Y=0
// White (P1) starts at Y=N
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
struct Tile {
    player: Player,
    piece: Piece,
}

impl Tile {
    fn emoji(self, data: &HBotData) -> &ReactionType {
        let e = data.app_emojis();
        match (self.player, self.piece) {
            (Player::P1, Piece::Pawn) => e.chess_white_pawn(),
            (Player::P1, Piece::Rook) => e.chess_white_rook(),
            (Player::P1, Piece::Bishop) => e.chess_white_bishop(),
            (Player::P1, Piece::Knight) => e.chess_white_knight(),
            (Player::P1, Piece::Queen) => e.chess_white_queen(),
            (Player::P1, Piece::King) => e.chess_white_king(),
            (Player::P2, Piece::Pawn) => e.chess_black_pawn(),
            (Player::P2, Piece::Rook) => e.chess_black_rook(),
            (Player::P2, Piece::Bishop) => e.chess_black_bishop(),
            (Player::P2, Piece::Knight) => e.chess_black_knight(),
            (Player::P2, Piece::Queen) => e.chess_black_queen(),
            (Player::P2, Piece::King) => e.chess_black_king(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Piece {
    Pawn,
    Rook,
    Bishop,
    Knight,
    Queen,
    King,
}

impl Piece {
    fn get_move(self) -> &'static dyn Move {
        match self {
            Self::Pawn => &MovePawn,
            Self::Rook => &MoveRook,
            Self::Bishop => &MoveBishop,
            Self::Knight => &MoveKnight,
            Self::Queen => &MoveQueen,
            Self::King => &MoveKing,
        }
    }
}

trait Move {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool>;
}

struct MovePawn;

impl MovePawn {
    fn y_dir(&self, player: Player) -> i8 {
        match player {
            Player::P1 => -1,
            Player::P2 => 1,
        }
    }

    fn is_home_row(&self, pos: Pos, player: Player) -> bool {
        match player {
            Player::P1 => usize::from(pos.y) == N - 1,
            Player::P2 => pos.y == 0,
        }
    }
}

impl Move for MovePawn {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool> {
        let y_dir = self.y_dir(player);
        let mut out = Grid::default();

        let pos = origin.add_y(y_dir);
        if let Some(tile) = board.get(pos) {
            if tile.is_none() {
                // same size so this must succeed
                *out.get_mut(pos).unwrap() = true;
            }

            for cap in [pos.add_x(-1), pos.add_x(1)] {
                if let Some(cap_tile) = board.get(cap) {
                    if cap_tile.is_some_and(|p| p.player != player) {
                        // same size so this must succeed
                        *out.get_mut(cap).unwrap() = true;
                    }
                }
            }
        }

        out
    }
}

struct MoveKnight;
impl Move for MoveKnight {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool> {
        let mut out = Grid::default();

        const DIRS: &[Offset] = &[
            Offset::new(2, 1),
            Offset::new(1, 2),
            Offset::new(-2, 1),
            Offset::new(-1, 2),
            Offset::new(2, -1),
            Offset::new(1, -2),
            Offset::new(-2, -1),
            Offset::new(-1, -2),
        ];

        for &dir in DIRS {
            let pos = origin.add_offset(dir);
            if let Some(tile) = board.get(pos) {
                if tile.is_none_or(|t| t.player != player) {
                    *out.get_mut(pos).unwrap() = true;
                }
            }
        }

        out
    }
}

struct MoveKing;
impl Move for MoveKing {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool> {
        let mut out = Grid::default();

        for &dir in MoveQueen::DIRS {
            let pos = origin.add_offset(dir);
            if let Some(tile) = board.get(pos) {
                if tile.is_none_or(|t| t.player != player) {
                    *out.get_mut(pos).unwrap() = true;
                }
            }
        }

        out
    }
}

trait MoveDirs {
    const DIRS: &[Offset];
}

struct MoveRook;
impl MoveDirs for MoveRook {
    const DIRS: &[Offset] = &[
        Offset::new(1, 0),
        Offset::new(0, 1),
        Offset::new(-1, 0),
        Offset::new(0, -1),
    ];
}

struct MoveBishop;
impl MoveDirs for MoveBishop {
    const DIRS: &[Offset] = &[
        Offset::new(1, 1),
        Offset::new(1, -1),
        Offset::new(-1, 1),
        Offset::new(-1, -1),
    ];
}

struct MoveQueen;
impl MoveDirs for MoveQueen {
    const DIRS: &[Offset] = &[
        Offset::new(1, 0),
        Offset::new(0, 1),
        Offset::new(-1, 0),
        Offset::new(0, -1),
        Offset::new(1, 1),
        Offset::new(1, -1),
        Offset::new(-1, 1),
        Offset::new(-1, -1),
    ];
}

impl<D: MoveDirs> Move for D {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool> {
        let mut out = Grid::default();

        for &dir in D::DIRS {
            let mut pos = origin.add_offset(dir);
            'm: while let Some(tile) = board.get(pos) {
                match tile {
                    None => *out.get_mut(pos).unwrap() = true,
                    Some(t) if t.player != player => {
                        *out.get_mut(pos).unwrap() = true;
                        break 'm;
                    },
                    Some(_) => break 'm,
                }

                pos = pos.add_offset(dir);
            }
        }

        out
    }
}

impl Board {
    fn is_player_in_check(&self, player: Player, king_at: Pos) -> bool {
        assert!(self.get(king_at).is_some(), "invalid king_at pos");

        let opponent = player.next();
        for (pos, piece) in self.iter_pieces(opponent) {
            let targets = piece.get_move().target_mask(self, pos, opponent);
            if *targets.get(king_at).unwrap() {
                return true;
            }
        }

        false
    }

    fn is_player_in_checkmate(&self, player: Player, king_at: Pos) -> bool {
        for (src, piece) in self.iter_pieces(player) {
            let mask = piece.get_move().target_mask(self, src, player);
            for (x, row) in mask.array.into_iter().enumerate() {
                for (y, t) in row.into_iter().enumerate() {
                    if t {
                        let dst = Pos::new_trunc(x, y);
                        let mut new_board = *self;

                        let tile = new_board.get_mut(src).expect("must be in range").take();
                        *new_board.get_mut(dst).expect("must be in range") = tile;

                        // for a king move we obviously have to check differently
                        let king_at = if piece == Piece::King { dst } else { king_at };
                        if !new_board.is_player_in_check(player, king_at) {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    fn iter_pieces(&self, player: Player) -> impl Iterator<Item = (Pos, Piece)> + use<'_> {
        self.array.iter().enumerate().flat_map(move |(x, row)| {
            row.iter()
                .enumerate()
                .filter_map(move |(y, tile)| match tile {
                    Some(t) if t.player == player => Some((Pos::new_trunc(x, y), t.piece)),
                    _ => None,
                })
        })
    }

    fn king_at(&self, player: Player) -> Option<Pos> {
        for (x, row) in self.array.iter().enumerate() {
            for (y, tile) in row.iter().enumerate() {
                if tile.is_some_and(|t| t.player == player && t.piece == Piece::King) {
                    return Some(Pos::new_trunc(x, y));
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    players: PlayerState,
    board: Board,
    action: Action,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Action {
    Idle,
    Selected(Pos),
    Move(Pos, Pos),
}

#[allow(clippy::cast_possible_truncation)]
fn flat_index(pos: Pos) -> u16 {
    u16::from(pos.x) * N as u16 + u16::from(pos.y)
}

impl View {
    pub fn new(players: [UserId; 2]) -> Self {
        fn p1(piece: Piece) -> Option<Tile> {
            Some(Tile {
                player: Player::P1,
                piece,
            })
        }

        fn p2(piece: Piece) -> Option<Tile> {
            Some(Tile {
                player: Player::P2,
                piece,
            })
        }

        Self {
            players: PlayerState::new(players),
            action: Action::Idle,
            board: Grid {
                #[rustfmt::skip]
                array: [
                    [p2(Piece::Rook), p2(Piece::Pawn), None, p1(Piece::Pawn), p1(Piece::Rook)],
                    [p2(Piece::Knight), p2(Piece::Pawn), None, p1(Piece::Pawn), p1(Piece::Knight)],
                    [p2(Piece::Bishop), p2(Piece::Pawn), None, p1(Piece::Pawn), p1(Piece::Bishop)],
                    [p2(Piece::Queen), p2(Piece::Pawn), None, p1(Piece::Pawn), p1(Piece::Queen)],
                    [p2(Piece::King), p2(Piece::Pawn), None, p1(Piece::Pawn), p1(Piece::King)],
                ],
            },
        }
    }

    fn board_buttons<'a>(&mut self, data: &'a HBotData) -> Vec<CreateActionRow<'a>> {
        let mut components = Vec::with_capacity(5);

        let moves = match self.action {
            Action::Selected(pos) => self
                .board
                .get(pos)
                .copied()
                .flatten()
                .map(|t| t.piece.get_move().target_mask(&self.board, pos, t.player))
                .map(|t| (t, pos)),
            _ => None,
        };

        for y in 0..N {
            let mut row = Vec::with_capacity(5);
            for x in 0..N {
                let tile = self.board.array[x][y];
                let pos = Pos::new_trunc(x, y);

                let icon = match tile {
                    Some(t) => t.emoji(data),
                    None => data.app_emojis().empty(),
                };

                let (style, action) = match (tile, &moves) {
                    (Some(t), _) if t.player == self.players.turn => {
                        (ButtonStyle::Primary, Action::Selected(pos))
                    },
                    (Some(_), Some(m)) if m.0.get(pos) == Some(&true) => {
                        (ButtonStyle::Danger, Action::Move(m.1, pos))
                    },
                    (_, Some(m)) if m.0.get(pos) == Some(&true) => {
                        (ButtonStyle::Success, Action::Move(m.1, pos))
                    },
                    _ => (ButtonStyle::Secondary, Action::Idle),
                };

                row.push(
                    if action == Action::Idle {
                        use crate::modules::core::buttons::None;

                        let key = ptr::from_ref(&self.action) as u16;
                        let value = flat_index(pos);
                        CreateButton::new(None::new(key, value).to_custom_id()).disabled(true)
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

    fn no_act_board_buttons<'a>(&self, data: &'a HBotData) -> Vec<CreateActionRow<'a>> {
        let mut components = Vec::with_capacity(5);

        for y in 0..N {
            let mut row = Vec::with_capacity(5);
            for x in 0..N {
                let tile = self.board.array[x][y];
                let pos = Pos::new_trunc(x, y);

                let icon = match tile {
                    Some(t) => t.emoji(data),
                    None => data.app_emojis().empty(),
                };

                row.push({
                    use crate::modules::core::buttons::None;

                    let value = flat_index(pos);
                    CreateButton::new(None::new(0, value).to_custom_id())
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
                "> **⬜ <@{}>**\n-# ⬛ <@{}>",
                self.players.ids[0], self.players.ids[1]
            ),
            Player::P2 => format!(
                "-# ⬜ <@{}>\n> **⬛ <@{}>**",
                self.players.ids[0], self.players.ids[1]
            ),
        };

        let embed = CreateEmbed::new()
            .description(description)
            .color(data.config().embed_color);

        let components = self.board_buttons(data);

        CreateReply::new().embed(embed).components(components)
    }

    fn create_win_reply(self, data: &HBotData) -> CreateReply<'_> {
        let winner_id = self.players.turn_user_id();

        let description = format!(
            "## <@{winner_id}> wins!\n\
             -# ⬜ <@{p1}>\n\
             -# ⬛ <@{p2}>",
            p1 = self.players.ids[0],
            p2 = self.players.ids[1],
        );

        let embed = CreateEmbed::new()
            .description(description)
            .color(data.config().embed_color);

        let components = self.no_act_board_buttons(data);

        CreateReply::new().embed(embed).components(components)
    }
}

impl ButtonArgsReply for View {
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
                // always go for queen promotion
                if src.piece == Piece::Pawn && MovePawn.is_home_row(dst, self.players.turn.next()) {
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
