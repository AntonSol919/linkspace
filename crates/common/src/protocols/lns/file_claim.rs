use std::path::{Path, PathBuf};

use anyhow::{ensure  };
use linkspace_pkt::read::read_pkt;

use super::{claim::Claim, admin::save_private_claim, name::Name};
use crate::prelude::*;


pub(crate) fn setup(lk:&Linkspace,claim: Claim,overwrite:bool) -> anyhow::Result<()>{
    let path = claim.name.file_path2()?;
    lk.env().set_files_data(path,claim.pkt.as_netpkt_bytes(),overwrite)?;
    if let Some(g) = claim.group() {
        lk.env().set_files_data(format!("by-group/{g}"), claim.name.to_string().as_bytes(), true)?;
    }
    if let Some(g) = claim.pubkey() {
        lk.env().set_files_data(format!("by-pubkey/{g}"), claim.name.to_string().as_bytes(), true)?;
    }
    save_private_claim(lk, &claim, None, &[],false)?;

    Ok(())
}



pub fn list_claims(root:&Path,name: &Name) -> anyhow::Result<Vec<anyhow::Result<Claim>>>{
    let mut path = name.file_path2()?;
    path.pop(); // remove claim.pkt
    let mut path = root.join(&path);
    let mut lst = vec![];
    traverse_claims(&mut path, &mut lst)?;
    Ok(lst)
}

fn traverse_claims(path:&mut PathBuf, claims: &mut Vec<anyhow::Result<Claim>>) -> anyhow::Result<()>{
    path.push("claim.pkt");
    if let Ok(b) = std::fs::read(&path){
        let p = try {
            let p = read_pkt(&b, true)?; 
            let claim = Claim::from(&*p)?;
            match claim.name.file_path2(){
                Err(e) => return Err(e.context(anyhow::anyhow!("{path:?} is invalid claim"))),
                Ok(c) => ensure!(path.ends_with(c),"{path:?} has a wrong claim?")
            };
            claim
        };
        claims.push(p);
    }
    path.pop();
    if let Ok(dir) = path.read_dir(){
        for entry in dir{
            let e = entry?;
            path.push(e.file_name());
            traverse_claims(path, claims)?;
            path.pop();
        }
    }
    Ok(())
}
