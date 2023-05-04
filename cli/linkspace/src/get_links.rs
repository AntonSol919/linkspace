//use linkspace::{lk_watch, query::lk_hash_query, try_cb, misc::cb, lk_process_while};
use linkspace_common::cli::opts::CommonOpts;
use crate::*;


#[derive(Parser)]
pub struct GetLinks{
    #[clap(flatten)]
    pkt_in: PktIn,
    /// writedest of stdin packets
    #[clap(short, long, default_value = "stdout")]
    forward: Vec<WriteDestSpec>,
    /// writedest of linked packets
    #[clap(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    #[clap(subcommand)]
    mode: GetLinksMode
}

#[derive(Clone,Parser)]
pub enum GetLinksMode{
    Skip,
    Pause,
    Error,
    Watch
}
pub fn exec(common:&CommonOpts, cmd: GetLinks) -> anyhow::Result<()>{
    let GetLinks { forward, write,pkt_in ,mode} = cmd;
    let lk = common.runtime()?;
    let inp = common.inp_reader(&pkt_in)?;
    let mut write = common.open(&write)?;
    let mut forward = common.open(&forward)?;
    let mut buffer = vec![];
    for pkt in inp {
        let pkt = pkt?;
        if !pkt.get_links().is_empty() {
            let reader = lk.env().get_reader()?;
            for link in pkt.get_links()
            {

               // let lk = linkspace::Linkspace::from_impl(&lk);
                //lk_watch(lk, &lk_hash_query(link.ptr), try_cb(|p,_| common.write_multi_dest(&mut write, p, Some(&mut buffer))))?;

                if let Some(pkt) = reader.read(&link.ptr)?{
                    common.write_multi_dest(&mut write, &pkt.pkt, Some(&mut buffer))?;
                    continue;
                }
                match mode {
                    GetLinksMode::Skip => {eprint!("link {link} in {} not found",pkt.hash())},
                    GetLinksMode::Pause => {
                        todo!()
                        
                    },
                    GetLinksMode::Error => todo!(),
                    GetLinksMode::Watch => todo!(),
                };
            }
        }
        common.write_multi_dest(&mut forward, &**pkt, Some(&mut buffer))?;
        if !buffer.is_empty() {
            let mut out = std::io::stdout();
            out.write_all(&mut buffer)?;
            out.flush()?;
            buffer.clear();
        }
    }
    Ok(())
} 
