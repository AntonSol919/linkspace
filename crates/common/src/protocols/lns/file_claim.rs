use super::{claim::Claim, admin::save_private_claim};
use crate::prelude::*;


pub(crate) fn setup(lk:&Linkspace,claim: Claim,overwrite:bool) -> anyhow::Result<()>{
    let path = claim.name.file_path()?;
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
