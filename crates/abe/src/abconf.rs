use std::fmt::{Display, Debug};

use anyhow::{ensure, bail};

use crate::{eval::{ABList }, ast::{Ctr }};


#[derive(Default,Clone)]
/** A generic serialize-deserialize format for an ablist. 
e.g.
```
hello:world/thing
:some:other:options:\f\f\f
/ok
```
**/
pub struct ABConf(pub Vec<ABList>);

pub fn parse_ablist_b(st:&[u8]) -> anyhow::Result<crate::eval::ABList>{
    let abe = crate::ast::parse_abe_strict_b(st)?;
    abe.as_slice().try_into().map_err(|e| anyhow::anyhow!("expr not supported - {e}"))
}

impl std::ops::Deref for ABConf{
    type Target = [ABList];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}
impl From<Vec<ABList>> for ABConf {
    fn from(value: Vec<ABList>) -> Self {
        ABConf::new(value)
    }
}
impl ABConf{
    pub const DEFAULT :Self= ABConf(vec![]);
    
    pub const fn new(values:Vec<ABList>) -> Self {
        ABConf(values)
    }
    pub fn extend(&mut self, tail: Vec<ABList> ){
        self.0.extend(tail)
    }
    pub fn push(&mut self, entry: ABList){
        self.0.push(entry)
    }
    pub fn get(&self,b:&[u8]) -> Option<Result<&ABList,&ABList>>{
        self.get_checked(&[b]).next()
    }
    
    /** used to get the first match and return its optional value.
    e.g. get_value(S0:S1:?val)
    
    S0:S1 => Some(None)
    S0:S1:val => Some(val)

    S0 => next
    S0:S1/ => Err
    S0:S1:val: => Err

    */
    pub fn has_optional_value(&self,starts_with:&[&[u8]]) -> Option<Result<Option<&[u8]>,&ABList>>{
        match &self.get_checked(starts_with).next()?{
            Ok(lst)  => match &lst[starts_with.len()-1..]{
                [(Some(Ctr::Colon),bytes) ] => Some(Ok(Some(bytes.as_slice()))),
                [] => Some(Ok(None)),
                _ => Some(Err(lst))
            }
            Err(v) => Some(Err(v)),
        }
    }
    /// Iterate over all matches starting with S0:S1:S2:val. Returns Err() if starts_with contains '/'
    pub fn get_checked<'a:'b,'b>(&'a self,starts_with:&'b [&[u8]]) -> impl Iterator<Item=Result<&'a ABList,&'a ABList>> + 'b{
        self.get_inner(starts_with)
            .map(|o| if o.iter().take(starts_with.len()).all(|c| matches!(c.0,None|Some(Ctr::Colon))) { Ok(o)} else {Err(o)})
    }
    pub fn get_inner<'a:'b,'b>(&'a self,starts_with:&'b [&[u8]]) -> impl Iterator<Item=&'a ABList> + 'b{
        self.0.iter().filter(move |a| {
            if a.len() < starts_with.len() { return false;}
            let ok = starts_with.iter().zip(a.iter_bytes()).all(|(b,opt)| b == &opt);
            ok
        })
    }
    pub fn try_from_txt(b:&[u8]) -> anyhow::Result<Self>{
        Self::try_from(b,false,Some(ABConfFmt::ABCTxt))
    }
    pub fn try_from(b:&[u8],header:bool,format:Option<ABConfFmt>) -> anyhow::Result<Self>{
        let (b,fmt) = match (header,format) {
            (true, _ ) => {
                ensure!(b.len() >= 8 ,"missing header");
                let (h,b) = b.split_at(8);
                let fmt = ABConfFmt::try_from(h.try_into().unwrap())?;
                if let Some(argf) = format{
                    ensure!(fmt == argf,"header is {fmt}, expected {argf}");
                }
                (b,fmt)
            },
            (false, None) => bail!("requires a header or a preset format"),
            (false, Some(f)) => (b,f),
        };
        let abc = match fmt {
            ABConfFmt::ABCTxt => {
                b.split(|c|*c==b'\n')
                    .map(|v|parse_ablist_b(v).map_err(|v| anyhow::anyhow!("data contains expr {v:?}")))
                    .try_collect()?
            },
        };
        Ok(ABConf(abc))
    }
    pub fn to_vec(&self) -> Vec<u8>{
        self.to_string().into_bytes()
    }

    pub fn serialize(&self, header:bool , fmt: ABConfFmt, write: &mut dyn std::io::Write) -> std::io::Result<()>{
        if header { write!(write,"{fmt}")?};
        match fmt {
            ABConfFmt::ABCTxt => {
                for abl in &self.0{
                    writeln!(write,"{abl}")?;
                }
                Ok(())
            },
        }
    }
    
}
impl Display for ABConf{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for abl in &self.0{
            writeln!(f,"{abl}")?;
        }
        Ok(())
    }
}
impl Debug for ABConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}




#[derive(Copy,Clone,Default,PartialEq)]
pub enum ABConfFmt{
    #[default]
    ABCTxt 
}
impl ABConfFmt {
    pub fn try_from(head: &[u8;8]) -> anyhow::Result<Self>{
        match head {
            b"#abctxt\n" => Ok(ABConfFmt::ABCTxt),
            e => bail!("unknown fmt {}",String::from_utf8_lossy(e))
        }
    }
}
impl Display for ABConfFmt{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ABConfFmt::ABCTxt => f.write_str("#abctxt\n"),
        }
    }
}

