#![feature(try_blocks)]

use anyhow::Context;
use linkspace::misc::*;
use mineweeper_rs::*;

fn main() -> anyhow::Result<()> {
    let mut game = MineWeeper::new(vec!["Alice".into(),"Bob".into()],10,10,0.2,*blake3_hash(b"22"));
    let stdin = std::io::stdin();
    let mut lines =stdin.lines();
    
    while print_game_state(&game){
        let line = lines.next().context("no more input")??;
        let e : anyhow::Result<()> = try{
            let (r,c) = line.split_once([';','|',' ',',',':']).context("missing col")?;
            clear_screen();
            game.reveal(r.parse()?, c.parse()?)?;
        };
        if let Err(e) = e{
            println!("{e}")
        }
    }
    Ok(())
}
