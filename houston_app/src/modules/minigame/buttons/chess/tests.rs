use super::game::*;
use super::*;

type BitBoard = Grid<bool>;

macro_rules! bit_board {
    ($($t:tt)*) => {
        BitBoard::new([
            [$(($t & 0b10000) != 0),*],
            [$(($t & 0b01000) != 0),*],
            [$(($t & 0b00100) != 0),*],
            [$(($t & 0b00010) != 0),*],
            [$(($t & 0b00001) != 0),*],
        ])
    };
}

#[test]
fn checkmate_no_moves() {
    let board = new_board!(
        [- -, - -, - -, - -, b K]
        [- -, - -, w Q, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [w K, - -, - -, w R, - -]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        board.is_player_in_checkmate(Player::P2, king_at),
        "must be checkmated"
    )
}

#[test]
fn checkmate_discovery() {
    let board = new_board!(
        [- -, - -, - -, w R, - -]
        [w R, - -, - -, - -, - -]
        [- -, - -, - -, - -, b K]
        [- -, - -, - -, b p, - -]
        [w K, - -, w B, - -, w R]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        board.is_player_in_checkmate(Player::P2, king_at),
        "must be checkmated"
    )
}

#[test]
fn not_checkmate_capture() {
    let board = new_board!(
        [- -, - -, - -, - -, b K]
        [- -, - -, w Q, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, b p, - -]
        [w K, - -, - -, - -, w R]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        !board.is_player_in_checkmate(Player::P2, king_at),
        "must not be checkmated"
    )
}

#[test]
fn not_checkmate_avoid() {
    let board = new_board!(
        [- -, - -, - -, - -, b K]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [w K, - -, - -, - -, w R]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        !board.is_player_in_checkmate(Player::P2, king_at),
        "must not be checkmated"
    )
}

#[test]
fn in_check() {
    let board = new_board!(
        [- -, - -, - -, - -, b K]
        [- -, - -, - -, - -, - -]
        [- -, - -, b Q, - -, - -]
        [- -, - -, - -, - -, - -]
        [w K, - -, - -, - -, w R]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        board.is_player_in_check(Player::P2, king_at),
        "must be in check"
    )
}

#[test]
fn not_in_check() {
    let board = new_board!(
        [- -, - -, - -, b K, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, b Q, - -, - -]
        [- -, - -, - -, - -, - -]
        [w K, - -, - -, - -, w R]
    );

    let king_at = board.king_at(Player::P2).expect("king present");
    assert!(
        !board.is_player_in_check(Player::P2, king_at),
        "must not be in check"
    )
}

#[test]
fn white_pawn_moves() {
    let board = new_board!(
        [b R, b k, b B, b Q, b K]
        [b p, b p, b p, b p, b p]
        [- -, - -, - -, w p, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, - -, - -]
    );

    let from_c4 = bit_board!(
        0b00000
        0b00000
        0b00100
        0b00000
        0b00000
    );

    let from_c3 = bit_board!(
        0b00000
        0b01010
        0b00000
        0b00000
        0b00000
    );

    let from_d4 = BitBoard::default();

    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(2, 3), Player::P1),
        from_c4,
    );
    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(2, 2), Player::P1),
        from_c3,
    );
    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(3, 3), Player::P1),
        from_d4,
    );
}

#[test]
fn black_pawn_moves() {
    let board = new_board!(
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, b p, - -]
        [w p, w p, w p, w p, w p]
        [w R, w k, w B, w Q, w K]
    );

    let from_c2 = bit_board!(
        0b00000
        0b00000
        0b00100
        0b00000
        0b00000
    );

    let from_c3 = bit_board!(
        0b00000
        0b00000
        0b00000
        0b01010
        0b00000
    );

    let from_d2 = BitBoard::default();

    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(2, 1), Player::P2),
        from_c2,
    );
    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(2, 2), Player::P2),
        from_c3,
    );
    assert_eq!(
        Piece::Pawn
            .get_move()
            .target_mask(&board, Pos::new(3, 1), Player::P2),
        from_d2,
    );
}

#[test]
fn queen_moves() {
    let board1 = new_board!(
        [- -, w p, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, b p, - -, w Q, - -]
        [- -, - -, - -, w p, - -]
        [- -, - -, - -, - -, - -]
    );

    let mask1 = bit_board!(
        0b00010
        0b00111
        0b01101
        0b00101
        0b01000
    );

    let board2 = new_board!(
        [- -, - -, - -, - -, - -]
        [- -, w p, - -, - -, - -]
        [- -, w Q, - -, b p, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, w p, - -]
    );

    let mask2 = bit_board!(
        0b00010
        0b10100
        0b10110
        0b11100
        0b01000
    );

    assert_eq!(
        Piece::Queen
            .get_move()
            .target_mask(&board1, Pos::new(3, 2), Player::P1),
        mask1
    );

    assert_eq!(
        Piece::Queen
            .get_move()
            .target_mask(&board2, Pos::new(1, 2), Player::P1),
        mask2
    );
}

#[test]
fn king_moves() {
    let board1 = new_board!(
        [- -, w p, - -, - -, - -]
        [- -, - -, - -, - -, - -]
        [- -, b p, - -, w K, - -]
        [- -, - -, - -, w p, - -]
        [- -, - -, - -, - -, - -]
    );

    let mask1 = bit_board!(
        0b00000
        0b00111
        0b00101
        0b00101
        0b00000
    );

    let board2 = new_board!(
        [- -, - -, - -, - -, - -]
        [- -, w p, - -, - -, - -]
        [- -, w K, - -, b p, - -]
        [- -, - -, - -, - -, - -]
        [- -, - -, - -, w p, - -]
    );

    let mask2 = bit_board!(
        0b00000
        0b10100
        0b10100
        0b11100
        0b00000
    );

    assert_eq!(
        Piece::King
            .get_move()
            .target_mask(&board1, Pos::new(3, 2), Player::P1),
        mask1
    );

    assert_eq!(
        Piece::King
            .get_move()
            .target_mask(&board2, Pos::new(1, 2), Player::P1),
        mask2
    );
}

#[test]
fn knight_moves() {
    let board1 = new_board!(
        [- -, - -, b p, - -, - -]
        [- -, w p, - -, - -, - -]
        [- -, - -, - -, w k, - -]
        [- -, - -, w p, w p, - -]
        [- -, - -, - -, - -, - -]
    );

    let mask1 = bit_board!(
        0b00101
        0b00000
        0b00000
        0b01000
        0b00101
    );

    let board2 = new_board!(
        [- -, - -, - -, - -, - -]
        [- -, w p, w p, - -, - -]
        [- -, w k, - -, - -, - -]
        [- -, - -, - -, w p, - -]
        [- -, - -, b p, - -, - -]
    );

    let mask2 = bit_board!(
        0b10100
        0b00010
        0b00000
        0b00000
        0b10100
    );

    assert_eq!(
        Piece::Knight
            .get_move()
            .target_mask(&board1, Pos::new(3, 2), Player::P1),
        mask1
    );

    assert_eq!(
        Piece::Knight
            .get_move()
            .target_mask(&board2, Pos::new(1, 2), Player::P1),
        mask2
    );
}
