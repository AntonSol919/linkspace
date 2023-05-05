use std::{fmt::{Display, Write}};
use anyhow::{Context, ensure};

pub fn clear_screen(){println!("{}","\n".repeat(80))}

struct Cell{
    pub mine:bool,
    pub revealed: bool,
    pub neighbour_mines : usize
}
impl Display for Cell{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits = b"0123456789";
        let ch = if !self.revealed { b'.'}else if self.mine {b'X'} else {digits[self.neighbour_mines]};
        f.write_char(ch as char)
    }
}
pub struct MineWeeper{
    pub players: Vec<String>,
    pub rows: u32,
    pub cols:u32,
    pub mine_rate:f64,
    pub seed : [u8;32],
    grid: Vec<Vec<Cell>>,
    pub loser : Option<usize>,
    pub game_round:usize
}

fn is_mine(seed:[u8;32],row:u32,col:u32,mine_rate:f64) -> bool {
    let bytes = [&seed as &[u8],&row.to_be_bytes(),&col.to_be_bytes()].concat();
    let rand_bytes = linkspace::misc::blake3_hash(&bytes);
    let random = linkspace::misc::bytes2uniform(&*rand_bytes);
    random < mine_rate
}

impl MineWeeper {
    pub fn new(players:Vec<String>,rows:u32,cols:u32,mine_rate:f64,seed:[u8;32]) -> Self {
        let mines :Vec<Vec<bool>>= (0..rows).map(|row| (0..cols).map(|col| is_mine(seed,row,col,mine_rate)).collect()).collect();
        let is_mine = |r:usize,c:usize| *mines.get(r).and_then(|row|row.get(c)).unwrap_or(&false);
        const N : [(isize,isize);8] = [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)];

        let new_cell = |row:usize,col:usize| Cell{
            mine: is_mine(row,col),
            revealed:false,
            neighbour_mines: N.iter().filter(|(dr,dc)| is_mine(row.wrapping_add_signed(*dr),col.wrapping_add_signed(*dc))).count()
        };
        let grid = (0..rows).map(|row| (0..cols).map(|col| new_cell(row as usize ,col as usize)).collect()).collect();
        MineWeeper {players,rows,cols,mine_rate,seed,grid, loser:None,game_round :0}
    }
    fn get_cell_mut(&mut self,row:u32,col:u32) ->Option<&mut Cell>{
        self.grid.get_mut(row as usize)?.get_mut(col as usize)
    }
    
    pub fn current_player(&self) -> usize {
        self.game_round % self.players.len()
    }
    pub fn count_revealable(&self) -> usize {
        self.grid.iter().map(|row|row.iter().filter(|v| !v.revealed).count()).sum()
    }
    pub fn reveal(&mut self,row:u32,col:u32) -> anyhow::Result<()>{
        let cell = self.get_cell_mut(row, col).context("Cell not in grid")?;
        ensure!(!cell.revealed, "Cell already revealed");
        cell.revealed = true;
        if cell.mine {
            self.loser = Some(self.current_player())
        }else {
            self.game_round += 1;
        }
        Ok(())
    }
}

pub fn print_game_state(mw:&MineWeeper) -> bool{
    print_grid(mw);
    println!("Round {}",mw.game_round);
    if let Some(loser) = mw.loser {
        println!("Game Finished!");
        println!("The loser is {} ({loser})!",mw.players[loser]);
        return false
    }

    if mw.count_revealable() == 0{
        println!("Everybody survived!");
        println!("For now ...");
        return false
    }
    println!("{} options to not lose",mw.count_revealable());
    let pid = mw.current_player();
    let name = &mw.players[pid];
    println!("Player {name} ({pid})");
    true
}

pub fn print_grid(mw:&MineWeeper){
    print!("[##]");
    (0..mw.cols).for_each(|c| print!(" [{c:>2}]"));
    println!("");
    for (r,row) in mw.grid.iter().enumerate(){
        print!("[{r:>2}]");
        row.iter().for_each(|cell| print!(" {cell:>2}"));
        println!("")
    }
}
