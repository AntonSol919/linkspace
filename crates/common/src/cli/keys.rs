// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::prelude::*;
use abe::TypedABE;
use anyhow::{anyhow, bail, Context};
use clap::Parser;
use linkspace_argon2_identity::{decrypt, encrypt, pubkey, INSECURE_COST};

use super::opts::CommonOpts;

#[derive(Parser, Clone, Debug)]
pub struct KeyOpts {
    /// (abe) password argon2 encrypted signing key
    #[clap(long, alias = "pass", env = "LINKSPACE_PASS")]
    password: Option<String>,
    /// print password
    #[clap(long)]
    display_pass: bool,
    /// use utf8 encoding for password instead of ABE
    #[clap(short, long)]
    utf8_password: bool,
    // todo should accept 'colon:separated:ids'
    #[clap(short, long, env = "LINKSPACE_KEY", default_value = "me")]
    name: String,
    /// use specific enckey str - will not lookup / generate
    #[clap(short, long)]
    enckey: Option<String>,
    #[clap(skip)]
    signing_key: std::sync::OnceLock<SigningKey>,
}
impl KeyOpts {
    pub fn identity<'i>(
        &'i self,
        common: &CommonOpts,
        prompt: bool,
    ) -> anyhow::Result<&'i SigningKey> {
        self.signing_key.get_or_try_init(|| {
            let password_bytes = self.password_bytes(common, prompt)?;
            match &self.enckey {
                Some(enckey) => Ok(crate::identity::decrypt(enckey, &password_bytes)?),
                None => {
                    let e =
                        String::new() + "{local:{:" + &self.name + "}::*=:enckey/readhash:data}";
                    let enckey = String::from_utf8(
                        common
                            .eval(&e)?
                            .into_exact_bytes()
                            .map_err(|_| anyhow!("bad enckey data"))?,
                    )?;
                    Ok(crate::identity::decrypt(&enckey, &password_bytes)?)
                }
            }
        })
    }
    pub fn password_bytes(&self, common: &CommonOpts, prompt: bool) -> anyhow::Result<Vec<u8>> {
        let pass_str = match self.password.as_deref() {
            Some(v) => v.to_string(),
            None if prompt => rpassword::prompt_password("password: ")?,
            None => anyhow::bail!("missing password"),
        };
        if self.display_pass {
            println!("input: {pass_str}")
        };
        if self.utf8_password {
            Ok(pass_str.as_bytes().to_owned())
        } else {
            let txt = pass_str.parse::<TypedABE<Vec<u8>>>()?;
            if self.display_pass {
                println!("abe: {txt}")
            };
            let bytes = txt.eval(&common.eval_ctx())?;
            if self.display_pass {
                println!("bytes: {}", AB(bytes.as_slice()))
            };
            Ok(bytes)
        }
    }
}

#[derive(Parser, Clone, Debug)]
pub struct KeyGenOpts {
    /// overwrite existing entry
    #[clap(long)]
    overwrite: bool,
    /// do not test password if only reading key
    #[clap(long)]
    no_check: bool,
    /// if a key already exists return an error
    #[clap(long)]
    error_existing: bool,
    /// do not create a local lns entry
    #[clap(long)]
    no_local_claim: bool,
    #[clap(flatten)]
    key: KeyOpts,
    /// supress enckey string output
    #[clap(long)]
    no_enckey: bool,
    /// supress pubkey string output
    #[clap(long)]
    no_pubkey: bool,
    /// Use insecure encryption paramaters to speed up unlocking
    #[clap(long)]
    insecure: bool,

    /// optional notes for lns entry
    #[clap(long)]
    notes: Option<String>,
}

pub fn keygen(common: &CommonOpts, opts: KeyGenOpts) -> anyhow::Result<()> {
    let rt = common.runtime()?;
    use crate::protocols::lns::local;
    let KeyGenOpts {
        insecure,
        overwrite,
        mut no_check,
        no_local_claim,
        key,
        notes,
        error_existing,
        no_enckey,
        no_pubkey,
    } = opts;

    let path = spath_buf(&[key.name.as_bytes()]);

    let user_enckey_input = key.enckey.is_some();
    let enckey = match key.enckey.clone() {
        Some(k) => Some(k),
        None => {
            // Equivelant to {local:+name+::l>:enckey::data}
            let r = rt.env().get_reader()?;
            match local::get_local_claim(&r, &path)? {
                None => None,
                Some(pkt) => {
                    let link = pkt
                        .get_links()
                        .iter()
                        .find(|v| v.tag.ends_with(b"enckey"))
                        .context("missing 'enckey' tag in claim")?;
                    let st = r
                        .read(&link.ptr)?
                        .context("missing 'enckey' pkt")?
                        .get_data_str()
                        .context("bad enckey format")?
                        .to_owned();
                    if error_existing {
                        bail!("already exists")
                    }
                    Some(st)
                }
            }
        }
    };
    use std::env::{args, current_dir, current_exe};
    let notes = notes.unwrap_or_else(|| {
        format!(
            "exec:{:?}\ndir:{:?}\nargs:{:?}",
            current_exe(),
            current_dir(),
            args()
        )
    });
    let rt = &common.runtime()?;

    let mut generate = |password: &[u8]| {
        let key = SigningKey::generate();
        no_check = true;
        encrypt(
            &key,
            &password,
            if insecure { Some(INSECURE_COST) } else { None },
        )
    };

    if overwrite {
        let enckey = match user_enckey_input {
            true => {
                let enckey = enckey.unwrap();
                if !no_check {
                    let password = key.password_bytes(common, true)?;
                    decrypt(&enckey, &password)?;
                }
                enckey
            }
            false => {
                let password = key.password_bytes(common, true)?;
                generate(&password)
            }
        };

        let pubkey = if !no_local_claim {
            local::setup_local_key(&rt, &key.name, &enckey, notes.as_bytes())?
        } else {
            B64(pubkey(&enckey)?)
        };
        if !no_enckey {
            println!("{enckey}")
        };
        if !no_pubkey {
            println!("{pubkey}")
        };
        return Ok(());
    }

    if user_enckey_input {
        bail!("missing --overwrite to setup custom enckey")
    }
    match enckey {
        Some(enckey) => {
            if !no_check {
                let password = key.password_bytes(common, true)?;
                decrypt(&enckey, &password)?;
            }
            let pubkey = B64(pubkey(&enckey)?);
            if !no_enckey {
                println!("{enckey}")
            };
            if !no_pubkey {
                println!("{pubkey}")
            };
        }
        None => {
            let password = key.password_bytes(common, true)?;
            let enckey = generate(&password);
            let pubkey = if !no_local_claim {
                local::setup_local_key(&rt, &key.name, &enckey, notes.as_bytes())?
            } else {
                B64(pubkey(&enckey)?)
            };
            if !no_enckey {
                println!("{enckey}")
            };
            if !no_pubkey {
                println!("{pubkey}")
            };
        }
    }
    Ok(())
}
