fn main(){println!("TODO")}
/*
#![feature(try_blocks,iterator_try_collect)]

use std::{env::args, cell::{ RefCell, Cell}, rc::Rc };

use anyhow::Context;
use linkspace::{prelude::*, lk_open, lk_key, lk_get, query::lk_hash_query, lk_encode, lk_query_parse, lk_query };
use mineweeper_rs::*;
use serde::{Serialize, Deserialize};


#[derive(Serialize,Deserialize,Debug)]
pub struct GameConf {
    pub players: Vec<(String,String)>,
    pub rows: u32,
    pub columns: u32,
    pub mine_rate: f64
}


fn main() -> anyhow::Result<()> {

    let game_hash : LkHash= args().nth(1).context("missing argv[1] hash")?.parse()?;
    let lk = lk_open(None, false)?;
    let key = lk_key(&lk,None, None, false)?;
    let game_pkt = lk_get(&lk, &lk_hash_query(game_hash))?.context("game_pkt not found")?;

    let GameConf { players,rows,columns,mine_rate} = serde_json::from_slice(game_pkt.data())?;
    println!("Starting Game");
    println!("Group : {}",lk_encode(game_pkt.get_group(), "#/@/b"));

    let game_path = ipath_buf(&[b"game",&*game_hash]);
    let common_q = lk_query_parse(
        lk_query(&Q),
        &["domain:=:mineweeper","group:=:[group]","path:=:/game/[hash]"],
        &game_pkt as &dyn NetPkt)?;

    let pullq = lk_query_push(lk_query(&common_q), "", "qid", &*game_hash)?;
    lk_pull(&lk, &pullq)?;

    // Using lk_keypoint_ref we could avoid an allocation
    // but we'll make the bold assumption keyboard input is the limiting factor in this process.
    let new_keypoint = |data:&[u8],links:&[Link]| lk_keypoint(ab(b"mineweeper"), *game_pkt.get_group(), &game_path, links, data, None, &key);

    let mut p_keys : Vec<PubKey> = vec![];
    let mut p_names = vec![];
    for (i,(name,key)) in players.into_iter().enumerate(){
        println!("{i} {name} : {key}");
        p_keys.push(key.parse()?);
        p_names.push(name);
    }


    let game = MineWeeper::new(
        p_names,
        rows,
        columns,
        mine_rate,
        *game_hash
    );
    let game = Rc::new(RefCell::new(game));
    let prev_turn = Rc::new(Cell::new(Link::new(b"prev",game_pkt.hash())));

    
    let stdin = std::io::stdin();
    let mut lines =stdin.lines();
    while print_game_state(&game.borrow()){
        let pid = game.borrow().current_player();
        let pubkey = p_keys[pid];
        let q = lk_query_push(common_q.clone(), "pubkey","=", &*pubkey)?;
        let _q = lk_query_push(q, "","qid", b"move")?;

        //lk_watch(&lk, &q, find_and_do_next_move());
        let line = lines.next().context("no more input")??;
        let e : anyhow::Result<()> = try{
            let (r,c) = line.split_once([';','|',' ',',',':']).context("missing col")?;
            clear_screen();
        };
        if let Err(e) = e{
            println!("{e}")
        }
    }
    Ok(())
}
*/
