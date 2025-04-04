use std::io::{self, Write};
use std::time::{Instant, Duration};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Black,
    White,
}

impl Piece {
    fn opponent(self) -> Piece {
        match self {
            Piece::Black => Piece::White,
            Piece::White => Piece::Black,
        }
    }
}

#[derive(Clone)]
struct Board {
    cells: [[Option<Piece>; 8]; 8],
}

impl Board {
    fn new() -> Self {
        let mut board = Board { cells: [[None; 8]; 8] };
        // 初期配置（中央の4マス）
        board.cells[3][3] = Some(Piece::White);
        board.cells[3][4] = Some(Piece::Black);
        board.cells[4][3] = Some(Piece::Black);
        board.cells[4][4] = Some(Piece::White);
        board
    }

    // ANSIエスケープシーケンスを用いて盤面を表示する
    fn print(&self) {
        // 列ラベル
        print!("  ");
        for col in 0..8 {
            print!(" {}  ", (b'A' + col as u8) as char);
        }
        println!();
        for row in 0..8 {
            // 行番号
            print!("{}  ", row + 1);
            for col in 0..8 {
                // 緑の背景
                let bg = "\x1b[42m";
                let reset = "\x1b[0m";
                let cell_str = match self.cells[row][col] {
                    Some(Piece::Black) => format!("⚫️"),
                    Some(Piece::White) => format!("⚪️"),
                    None => "⚪︎".to_string(),
                };
                // 背景を残すため、背景コードを前に付けて出力
                print!("{} {} {}", bg, cell_str, reset);
            }
            println!();
        }
    }

    // 座標が盤内かどうか
    fn in_bounds(row: i32, col: i32) -> bool {
        row >= 0 && row < 8 && col >= 0 && col < 8
    }

    // 指定したプレイヤーの合法手一覧を返す
    fn valid_moves(&self, piece: Piece) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        for row in 0..8 {
            for col in 0..8 {
                if self.cells[row][col].is_none() && self.is_valid_move(piece, row, col) {
                    moves.push((row, col));
                }
            }
        }
        moves
    }

    // (row, col)にpieceを置くことが合法かどうか
    fn is_valid_move(&self, piece: Piece, row: usize, col: usize) -> bool {
        if self.cells[row][col].is_some() {
            return false;
        }
        let directions = [(-1, -1), (-1, 0), (-1, 1),
                          (0, -1),           (0, 1),
                          (1, -1),  (1, 0),  (1, 1)];
        for &(dx, dy) in directions.iter() {
            let mut r = row as i32 + dx;
            let mut c = col as i32 + dy;
            let mut found_opponent = false;
            while Board::in_bounds(r, c) {
                match self.cells[r as usize][c as usize] {
                    Some(p) if p == piece.opponent() => {
                        found_opponent = true;
                    }
                    Some(p) if p == piece => {
                        if found_opponent {
                            return true;
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
                r += dx;
                c += dy;
            }
        }
        false
    }

    // 指定した手を適用し、挟んだ相手の石を反転する
    // 合法手でなければfalseを返す
    fn apply_move(&mut self, piece: Piece, row: usize, col: usize) -> bool {
        if !self.is_valid_move(piece, row, col) {
            return false;
        }
        self.cells[row][col] = Some(piece);
        let directions = [(-1, -1), (-1, 0), (-1, 1),
                          (0, -1),           (0, 1),
                          (1, -1),  (1, 0),  (1, 1)];
        for &(dx, dy) in directions.iter() {
            let mut r = row as i32 + dx;
            let mut c = col as i32 + dy;
            let mut pieces_to_flip = Vec::new();
            while Board::in_bounds(r, c) {
                match self.cells[r as usize][c as usize] {
                    Some(p) if p == piece.opponent() => {
                        pieces_to_flip.push((r as usize, c as usize));
                    }
                    Some(p) if p == piece => {
                        if !pieces_to_flip.is_empty() {
                            for (fr, fc) in pieces_to_flip {
                                self.cells[fr][fc] = Some(piece);
                            }
                        }
                        break;
                    }
                    _ => break,
                }
                r += dx;
                c += dy;
            }
        }
        true
    }

    // 指定したプレイヤーの石の個数を返す
    fn count(&self, piece: Piece) -> usize {
        self.cells
            .iter()
            .flatten()
            .filter(|&&p| p == Some(piece))
            .count()
    }

    // 両者とも合法手がない場合、ゲーム終了とする
    fn is_game_over(&self) -> bool {
        self.valid_moves(Piece::Black).is_empty() && self.valid_moves(Piece::White).is_empty()
    }

    // 評価関数：石の個数差に加え、角の獲得にボーナスを与える
    fn evaluate(&self, piece: Piece) -> i32 {
        let mut score = 0;
        let corner_bonus = 25;
        for row in 0..8 {
            for col in 0..8 {
                match self.cells[row][col] {
                    Some(p) if p == piece => {
                        score += 10;
                        if (row == 0 && col == 0)
                            || (row == 0 && col == 7)
                            || (row == 7 && col == 0)
                            || (row == 7 && col == 7)
                        {
                            score += corner_bonus;
                        }
                    }
                    Some(p) if p == piece.opponent() => {
                        score -= 10;
                        if (row == 0 && col == 0)
                            || (row == 0 && col == 7)
                            || (row == 7 && col == 0)
                            || (row == 7 && col == 7)
                        {
                            score -= corner_bonus;
                        }
                    }
                    _ => {}
                }
            }
        }
        score
    }
}

// Minimax（α–β法）による探索
// 時間制限内に探索を打ち切るため、開始時刻と許容時間を渡す
fn minimax(
    board: &Board,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    maximizing: bool,
    piece: Piece,
    start: Instant,
    time_limit: Duration,
) -> i32 {
    if depth == 0 || board.is_game_over() || start.elapsed() >= time_limit {
        return board.evaluate(piece);
    }
    let moves = board.valid_moves(if maximizing { piece } else { piece.opponent() });
    if moves.is_empty() {
        // 合法手がない場合はパスして相手に手番を渡す
        return minimax(board, depth, alpha, beta, !maximizing, piece, start, time_limit);
    }
    if maximizing {
        let mut max_eval = i32::MIN;
        for (r, c) in moves {
            let mut new_board = board.clone();
            new_board.apply_move(piece, r, c);
            let eval = minimax(&new_board, depth - 1, alpha, beta, false, piece, start, time_limit);
            max_eval = max_eval.max(eval);
            alpha = alpha.max(eval);
            if beta <= alpha {
                break;
            }
        }
        max_eval
    } else {
        let mut min_eval = i32::MAX;
        for (r, c) in moves {
            let mut new_board = board.clone();
            new_board.apply_move(piece.opponent(), r, c);
            let eval = minimax(&new_board, depth - 1, alpha, beta, true, piece, start, time_limit);
            min_eval = min_eval.min(eval);
            beta = beta.min(eval);
            if beta <= alpha {
                break;
            }
        }
        min_eval
    }
}

// 反復深化により、指定の時間内で最善手を求める
fn get_best_move(
    board: &Board,
    piece: Piece,
    time_limit: Duration,
    max_depth: u32,
) -> Option<(usize, usize)> {
    let start = Instant::now();
    let mut best_move = None;
    let mut best_score = i32::MIN;
    let moves = board.valid_moves(piece);
    if moves.is_empty() {
        return None;
    }
    for depth in 1..=max_depth {
        for &(r, c) in moves.iter() {
            let mut new_board = board.clone();
            new_board.apply_move(piece, r, c);
            let score = minimax(&new_board, depth - 1, i32::MIN, i32::MAX, false, piece, start, time_limit);
            if score > best_score {
                best_score = score;
                best_move = Some((r, c));
            }
            if start.elapsed() >= time_limit {
                break;
            }
        }
        if start.elapsed() >= time_limit {
            break;
        }
    }
    best_move
}

// 入力例 "A1" や "C3" から盤面上の座標 (row, col) に変換する
fn parse_input(input: &str) -> Option<(usize, usize)> {
    let input = input.trim().to_uppercase();
    if input.len() < 2 {
        return None;
    }
    let col_char = input.chars().next()?;
    let row_str = &input[1..];
    let col = (col_char as u8).wrapping_sub(b'A') as usize;
    let row = row_str.parse::<usize>().ok()? - 1;
    if row < 8 && col < 8 {
        Some((row, col))
    } else {
        None
    }
}

fn main() {
    let mut board = Board::new();
    let player_piece = Piece::Black; // 先手は黒
    let ai_piece = Piece::White;
    // AIの思考時間上限（約5秒）
    let ai_time_limit = Duration::from_secs(5);
    // 反復深化の最大探索深度（11手程度読む）
    let max_search_depth = 11;

    loop {
        println!("\nCurrent board:");
        board.print();
        if board.is_game_over() {
            break;
        }
        // プレイヤーのターン
        let player_moves = board.valid_moves(player_piece);
        if !player_moves.is_empty() {
            println!("Your turn. Enter your move (e.g., A1): ");
            let mut input = String::new();
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            if let Some((r, c)) = parse_input(&input) {
                if board.apply_move(player_piece, r, c) {
                    println!("You placed at {}{}", (b'A' + c as u8) as char, r + 1);
                } else {
                    println!("Invalid move. Try again.");
                    continue;
                }
            } else {
                println!("Invalid input. Please use format like A1.");
                continue;
            }
        } else {
            println!("No valid moves for you. Passing turn.");
        }

        if board.is_game_over() {
            break;
        }

        // コンピュータ（AI）のターン
        let ai_moves = board.valid_moves(ai_piece);
        if !ai_moves.is_empty() {
            println!("AI is thinking...");
            if let Some((r, c)) = get_best_move(&board, ai_piece, ai_time_limit, max_search_depth) {
                board.apply_move(ai_piece, r, c);
                println!("AI placed at {}{}", (b'A' + c as u8) as char, r + 1);
            } else {
                println!("AI has no valid moves. Passing turn.");
            }
        } else {
            println!("AI has no valid moves. Passing turn.");
        }
    }
    println!("Game over!");
    board.print();
    let player_count = board.count(player_piece);
    let ai_count = board.count(ai_piece);
    println!("Your pieces: {}", player_count);
    println!("AI pieces: {}", ai_count);
    if player_count > ai_count {
        println!("You win!");
    } else if ai_count > player_count {
        println!("AI wins!");
    } else {
        println!("It's a tie!");
    }
}