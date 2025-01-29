//! Model and actual core game logic like allowed moves etc.

use super::Player;
use crate::buttons::prelude::*;

pub const N: usize = 5;

pub type Board = Grid<Option<Tile>>;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pos {
    pub x: u8,
    pub y: u8,
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
impl Pos {
    pub fn new_trunc(x: usize, y: usize) -> Self {
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
pub struct Offset {
    x: i8,
    y: i8,
}

impl Offset {
    const fn new(x: i8, y: i8) -> Self {
        Self { x, y }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Grid<T> {
    array: [[T; N]; N],
}

impl<T> Grid<T> {
    pub const fn new(array: [[T; N]; N]) -> Self {
        Self { array }
    }

    pub fn get(&self, pos: Pos) -> Option<&T> {
        self.array.get(usize::from(pos.x))?.get(usize::from(pos.y))
    }

    pub fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        self.array
            .get_mut(usize::from(pos.x))?
            .get_mut(usize::from(pos.y))
    }

    fn iter_grid(&self) -> impl Iterator<Item = (Pos, &T)> + use<'_, T> {
        self.array.iter().enumerate().flat_map(|(x, row)| {
            row.iter()
                .enumerate()
                .map(move |(y, tile)| (Pos::new_trunc(x, y), tile))
        })
    }
}

// Black (P2) starts at Y=0
// White (P1) starts at Y=N
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tile {
    pub player: Player,
    pub piece: Piece,
}

impl Tile {
    pub const fn new(player: Player, piece: Piece) -> Self {
        Self { player, piece }
    }

    pub fn emoji(self, data: &HBotData) -> &ReactionType {
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
pub enum Piece {
    Pawn,
    Rook,
    Bishop,
    Knight,
    Queen,
    King,
}

impl Piece {
    pub fn get_move(self) -> &'static dyn Move {
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

pub fn is_home_row(pos: Pos, player: Player) -> bool {
    match player {
        Player::P1 => usize::from(pos.y) == N - 1,
        Player::P2 => pos.y == 0,
    }
}

pub trait Move {
    fn target_mask(&self, board: &Board, origin: Pos, player: Player) -> Grid<bool>;
}

pub struct MovePawn;

impl MovePawn {
    fn y_dir(&self, player: Player) -> i8 {
        match player {
            Player::P1 => -1,
            Player::P2 => 1,
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

impl Grid<bool> {
    fn iter_true(&self) -> impl Iterator<Item = Pos> + use<'_> {
        self.iter_grid().filter(|t| *t.1).map(|t| t.0)
    }
}

impl Board {
    pub fn is_player_in_check(&self, player: Player, king_at: Pos) -> bool {
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

    pub fn is_player_in_checkmate(&self, player: Player, king_at: Pos) -> bool {
        for (src, piece) in self.iter_pieces(player) {
            let mask = piece.get_move().target_mask(self, src, player);
            for dst in mask.iter_true() {
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

    pub fn king_at(&self, player: Player) -> Option<Pos> {
        self.iter_pieces(player)
            .find(|t| t.1 == Piece::King)
            .map(|t| t.0)
    }
}

/// Macro to construct boards in a way that's more human-readable.
#[rustfmt::skip]
macro_rules! new_board {
    (@player w) => {Player::P1};
    (@player b) => {Player::P2};
    (@piece p) => {Piece::Pawn};
    (@piece R) => {Piece::Rook};
    (@piece B) => {Piece::Bishop};
    (@piece k) => {Piece::Knight};
    (@piece Q) => {Piece::Queen};
    (@piece K) => {Piece::King};
    (@tile - -) => {None};
    (@tile $player:tt $piece:tt) => {
        Some(Tile::new(new_board!(@player $player), new_board!(@piece $piece)))
    };

    // main macro entry point
    (
        [$($pl1:tt $pi1:tt),*]
        [$($pl2:tt $pi2:tt),*]
        [$($pl3:tt $pi3:tt),*]
        [$($pl4:tt $pi4:tt),*]
        [$($pl5:tt $pi5:tt),*]
    ) => {
        const {
            use $crate::modules::minigame::buttons::Player;
            use $crate::modules::minigame::buttons::chess::game::{Board, Piece, Tile};
            Board::new([
                $([
                    new_board!(@tile $pl1 $pi1),
                    new_board!(@tile $pl2 $pi2),
                    new_board!(@tile $pl3 $pi3),
                    new_board!(@tile $pl4 $pi4),
                    new_board!(@tile $pl5 $pi5),
                ]),*
            ])
        }
    };
}

pub(super) use new_board;
