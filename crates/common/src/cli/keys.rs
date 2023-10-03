// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{prelude::*, protocols::lns::{name::{NameExpr }, self }};
use abe::{TypedABE };
use anyhow::{ Context, bail };
use clap::Parser;

use super::{opts::CommonOpts };

#[derive(Parser, Clone, Debug)]
pub struct KeyOpts {
    /// (abe) password argon2 encrypted signing key
    #[arg(long, alias = "pass", env = "LK_PASS")]
    password: Option<String>,
    /// print password - is always in abtxt format
    #[arg(long)]
    display_pass: bool,
    /// use utf8 encoding for password instead of ABE
    #[arg(short, long)]
    utf8_password: bool,
    /// local key name - e.g. my:home:local
    #[arg(short, long, env = "LK_KEYNAME",default_value="me:local")]
    key: NameExpr,
    /// use specific enckey instead of looking for key. can be $argon str or (claim) hash
    #[arg(short, long)]
    enckey: Option<String>,
    #[clap(skip)]
    signing_key: std::sync::OnceLock<SigningKey>,
}
impl Default for KeyOpts {
    fn default() -> Self {
        Self { password: Default::default(),
               display_pass: Default::default(),
               utf8_password: Default::default(),
               key: "me:local".parse().unwrap(),
               enckey: Default::default(),
               signing_key: Default::default()
        }
    }
}
impl KeyOpts {

    pub fn enckey(&self, _common:&CommonOpts) ->anyhow::Result<Option<(PubKey,String)>>{
        let enckey = match &self.enckey {
            None => return Ok(None),
            Some(k) => k,
        };
        if enckey.starts_with('$'){
            let pubkey = crate::identity::pubkey(enckey)?.into();
            return Ok(Some((pubkey,enckey.to_string())))
        };
        todo!()
    }

    pub fn identity<'i>(
        &'i self,
        common: &CommonOpts,
        prompt: bool,
    ) -> anyhow::Result<&'i SigningKey> {
        self.signing_key.get_or_try_init(|| {
            match &self.enckey {
                Some(enckey) => {
                    let password_bytes = self.password_bytes(common, prompt)?;
                    Ok(crate::identity::decrypt(enckey, &password_bytes)?)
                },
                None => {
                    let name = self.key.eval(&common.eval_scope())?;
                    let (_,enckey)= lns::lookup_enckey(&common.runtime()?, &name)?.context("no such enckey")?;
                    let password_bytes = self.password_bytes(common, prompt)?;
                    Ok(crate::identity::decrypt(&enckey, &password_bytes)?)
                }
            }
        })
    }
    pub fn password_bytes(&self, common: &CommonOpts, prompt: bool) -> anyhow::Result<Vec<u8>> {
        self.password_bytes_prompt(common, prompt,"password: ")
    }
    pub fn password_bytes_prompt(&self, common: &CommonOpts, prompt: bool,st:&str) -> anyhow::Result<Vec<u8>> {
        let pass_str = match self.password.as_deref() {
            Some(v) => v.to_string(),
            None if prompt => rpassword::prompt_password(st)?,
            None => anyhow::bail!("missing password"),
        };
        let bytes = if self.utf8_password {
            pass_str.as_bytes().to_owned()
        } else {
            let txt = pass_str.parse::<TypedABE<Vec<u8>>>()?;
            txt.eval(&common.eval_scope())?
        };

        if self.display_pass {
            println!("{}",AB(&bytes))
        };
        Ok(bytes)
    }
}


#[derive(Parser, Clone, Debug)]
/**
**/
pub struct KeyGenOpts {
    /// overwrite existing entry
    #[arg(long)]
    overwrite: bool,
    /// do not test password if only reading key
    #[arg(long)]
    no_check: bool,
    /// if a key already exists return an error
    #[arg(long,alias="create-new")]
    error_some: bool,
    /// if a key does not exists return an error
    #[arg(long,alias="open")]
    error_none: bool,

    /// do not use a linkspace instance. Won't save or get.
    #[arg(long,conflicts_with_all(["error_some","error_none","overwrite","key"]))]
    no_lk: bool,
    #[command(flatten)]
    key: KeyOpts,
    /// supress enckey string output
    #[arg(long)]
    no_enckey: bool,
    /// supress pubkey string output
    #[arg(long)]
    no_pubkey: bool,
    /// Set to 0 to use insecure encryption paramaters to speed up unlocking
    #[arg(long,default_value_t=1)]
    decrypt_cost: usize,

    /// after reading a argon2id, first re-encode it with a new password
    #[arg(long)]
    new_pass: bool,
    /// new password -- implies new_pass
    #[arg(long)]
    new_pass_str: Option<String>,
}


pub fn keygen(common: &CommonOpts, opts: KeyGenOpts) -> anyhow::Result<()> {
    use linkspace_argon2_identity::{decrypt, encrypt, pubkey};
    let KeyGenOpts {
        decrypt_cost,
        overwrite,
        mut no_check,
        no_lk,
        mut key,
        error_some,
        no_enckey,
        no_pubkey,
        new_pass,
        new_pass_str,
        error_none,
    } = opts;

    let with_rt = if no_lk { None} else{ Some(common.runtime()?)};
    use linkspace_argon2_identity::{INSECURE_COST, DEFAULT_COST, EXPENSIVE_COST};
    let cost = Some(match decrypt_cost{
        0 => INSECURE_COST,
        1 => DEFAULT_COST,
        _ => EXPENSIVE_COST
    });

    let mut generate = |password: &[u8]| {
        let key = SigningKey::generate();
        no_check = true;
        encrypt(
            &key,
            password,
            cost
        )
    };
    let print = |enckey,pubkey|{
        if !no_enckey {println!("{enckey}")};
        if !no_pubkey {println!("{pubkey}")};
    };
    let name = key.key.eval(&common.eval_scope())?;
    //let name = key.key.clone().map(|e| e.eval(&common.eval_scope())).transpose()?.unwrap_or_else(Name::local);
    //ensure!(name.local_branch_authority(),"key names must end in :local - use lns to create public identities");
    let user_enckey_input = key.enckey.is_some();

    let mut enckey = match (key.enckey.clone(),&with_rt) {
        (Some(k),_) => Some(k),
        (None,None)=> None ,
        (None,Some(rt)) => match lns::lookup_enckey(rt, &name)?{
            None if error_none => bail!("no key found"),
            None => None,
            Some(_) if error_some => anyhow::bail!("already exists"),
            Some((_,e)) => Some(e)
        }
    };
    tracing::debug!(?enckey);
    if new_pass || new_pass_str.is_some(){
        let keystr = enckey.context("new_pass but no --enckey found")?;
        let old_password = key.password_bytes_prompt(common, true,"old password: ")?;
        let skey = decrypt(&keystr, &old_password)?;
        key.password = new_pass_str;
        let new_password =key.password_bytes_prompt(common, true,"new password: ")?;
        key.password = Some(abtxt::as_abtxt(&new_password).to_string());
        key.utf8_password = false;
        enckey = Some(encrypt(&skey, &new_password, cost))
    }

    if overwrite {
        let enckey = match user_enckey_input {
            true => {
                let enckey = enckey.unwrap();
                if !no_check {
                    let password = key.password_bytes_prompt(common, true,"decrypting - password>")?;
                    decrypt(&enckey, &password)?;
                }
                enckey
            }
            false => {
                let password = key.password_bytes_prompt(common, true,"generating new - password>")?;
                generate(&password)
            }
        };

        let pubkey = match &with_rt {
            Some(rt) => lns::setup_special_keyclaim(rt, name, &enckey,true)?,
            None => B64(pubkey(&enckey)?),
        };
        print(enckey,pubkey);
        return Ok(());
    }

    if user_enckey_input && !no_lk{
        bail!("missing --overwrite to setup new enckey")
    }
    tracing::debug!(?enckey);
    match enckey {
        Some(enckey) => {
            if !no_check {
                let password = key.password_bytes_prompt(common, true,"decrypting - password>")?;
                decrypt(&enckey, &password)?;
            }
            let pubkey = B64(pubkey(&enckey)?);
            print(enckey,pubkey);
        }
        None => {
            let password = key.password_bytes_prompt(common, true,"generating new - password>")?;
            let enckey = generate(&password);
            let pubkey = match &with_rt {
                None => B64(pubkey(&enckey)?),
                Some(rt) => lns::setup_special_keyclaim(rt, name, &enckey,false)?
            };
            print(enckey,pubkey)
        }
    }
    Ok(())
}
