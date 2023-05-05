struct MineWeeper { players: Vec<String>, rows: usize, mine_rate: f64, seed: [u8; 32], cols: usize, mine_rate: f64, seed: [u8; 32], grid: Vec<Vec<Cell>>, loser: Option<usize>, game_round: usize }
