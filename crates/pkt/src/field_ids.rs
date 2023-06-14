// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{
    netpkt::{NetFlags, NetPktHeader},
    *,
};
use byte_fmt::{
    abe::{print_abe, ToABE}, 
};
use core::str::FromStr;
use std::fmt::Debug;

pub trait FieldG {
    const PKTS: PointTypeFlags;
}
pub trait NamedField
where
    Self: Sized + FieldG,
{
    const TOKEN: u8;
    const NAME: &'static str;
    const ENUM: FieldEnum;
    fn info() -> FieldInfo {
        as_info::<Self>()
    }
}
pub struct FieldInfo {
    pub token: u8,
    pub name: &'static str,
    pub pkts: PointTypeFlags,
}
const fn as_info<I: NamedField>() -> FieldInfo {
    FieldInfo {
        token: I::TOKEN,
        name: I::NAME,
        pkts: I::PKTS,
    }
}

macro_rules! fid  {
    ([$(($fname:ident,$token:expr,$name:expr,$pkt_types:expr)),*]) => {
        #[macro_export]
        macro_rules! fixed_fields_arr {
            ( $foreach:tt) => {
                [$($foreach!($fname,$name)),*]
            };
        }
        $(
            #[derive(Copy,Clone,Debug)]
            pub struct $fname;
            impl FieldG for $fname {
                const PKTS : PointTypeFlags= $pkt_types;
            }
            impl NamedField for $fname{
                const TOKEN: u8 = $token;
                const NAME : &'static str = $name;
                const ENUM : FieldEnum = FieldEnum::$fname;
            }
            impl $fname {
                pub const fn id(self) -> &'static [u8]{
                    $name.as_bytes()
                }
            }
        )*
        #[derive(Debug,Copy,Clone,Eq,PartialEq)]
        #[repr(u8)]
        /// An enum that provides access to the fields in a [NetPkt]
        pub enum FieldEnum {
            $( $fname = $token ),*
        }
        impl FieldEnum {
            pub const LIST : [FieldEnum;29]= [$(FieldEnum::$fname,)*];
            pub fn try_from_id(id:&[u8]) -> Option<Self> {
                $( if id == &[$token] || id == $name.as_bytes() { return Some(FieldEnum::$fname);})*
                    None
            }
            pub const fn info(self) -> FieldInfo{
                match self {
                    $(FieldEnum::$fname => as_info::<$fname>() ),*
                }
            }
        }
    };
}

impl FieldEnum {
    pub fn try_to_abe(self, abl: abe::eval::ABList) -> Option<Vec<abe::ABE>> {
        match self {
            FieldEnum::PathLenF => U8::try_from(abl).ok().map(|v| v.to_abe()),

            FieldEnum::PktTypeF | FieldEnum::VarNetFlagsF => {
                U8::try_from(abl).ok().map(|v| v.abe_bits())
            }
            FieldEnum::CreateF | FieldEnum::VarStampF => {
                Stamp::try_from(abl).ok().map(|v| v.to_abe())
            }
            FieldEnum::VarHopF
            | FieldEnum::VarUBits0F
            | FieldEnum::VarUBits1F
            | FieldEnum::VarUBits2F
            | FieldEnum::VarUBits3F => U32::try_from(abl).ok().map(|v| v.to_abe()),
            FieldEnum::PubKeyF | FieldEnum::GroupIDF | FieldEnum::PktHashF => {
                LkHash::try_from(abl).ok().map(|v| v.to_abe())
            }
            FieldEnum::LinksLenF | FieldEnum::DataSizeF | FieldEnum::PointSizeF => {
                U16::try_from(abl).ok().map(|v| v.to_abe())
            }
            FieldEnum::DomainF => Domain::try_from(abl).ok().map(|v| v.to_abe()),
            FieldEnum::PathComp0F
            | FieldEnum::PathComp1F
            | FieldEnum::PathComp2F
            | FieldEnum::PathComp3F
            | FieldEnum::PathComp4F
            | FieldEnum::PathComp5F
            | FieldEnum::PathComp6F
            | FieldEnum::PathComp7F
            | FieldEnum::PathF => SPathBuf::try_from(abl).ok().map(|v| v.to_abe()),

            FieldEnum::IPathF => IPathBuf::try_from(abl).ok().map(|v| v.to_abe()),
            FieldEnum::SignatureF => Signature::try_from(abl).ok().map(|v| v.to_abe()),
            FieldEnum::DataF => Some(abl.into()),
        }
    }
}
impl std::fmt::Display for FieldEnum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.info().name)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FieldEnumErr {
    #[error("No Such field")]
    NoSuchField,
}
impl FromStr for FieldEnum {
    type Err = FieldEnumErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FieldEnum::try_from_id(s.as_bytes()).ok_or(FieldEnumErr::NoSuchField)
    }
}

pub const VAR_FIELDS: [FieldEnum; 7] = [
    FieldEnum::VarNetFlagsF,
    FieldEnum::VarHopF,
    FieldEnum::VarStampF,
    FieldEnum::VarUBits0F,
    FieldEnum::VarUBits1F,
    FieldEnum::VarUBits2F,
    FieldEnum::VarUBits3F,
];

fid! {[
    (VarNetFlagsF,b'f',"netflags", PointTypeFlags::DATA),
    (VarHopF,b'j',"hop" , PointTypeFlags::DATA),
    (VarStampF,b's',"stamp" , PointTypeFlags::DATA),
    (VarUBits0F,b'q',"ubits0" , PointTypeFlags::DATA),
    (VarUBits1F,b'Q',"ubits1" , PointTypeFlags::DATA),
    (VarUBits2F,b'w',"ubits2" , PointTypeFlags::DATA),
    (VarUBits3F,b'W',"ubits3" , PointTypeFlags::DATA),
    (PktHashF,b'h',"hash" , PointTypeFlags::DATA),
    (PktTypeF,b'y',"type", PointTypeFlags::DATA),
    (PointSizeF,b'o',"point_size", PointTypeFlags::DATA ),
    (PubKeyF,b'k',"pubkey",PointTypeFlags::SIGNATURE),
    (SignatureF,b'v',"signature",PointTypeFlags::SIGNATURE),
    (GroupIDF,b'g',"group",PointTypeFlags::LINK),
    (DomainF,b'd',"domain",PointTypeFlags::LINK),
    (CreateF,b'c',"create",PointTypeFlags::LINK),
    (PathLenF,b'x',"path_len",PointTypeFlags::LINK),
    (LinksLenF,b'l',"links_len",PointTypeFlags::LINK),
    (DataSizeF,b'B',"data_size",PointTypeFlags::DATA),
    (PathF,b'p',"path",PointTypeFlags::LINK),
    (IPathF,b'P',"ipath",PointTypeFlags::LINK),
    (PathComp0F,b'0',"path0",PointTypeFlags::LINK),
    (PathComp1F,b'1',"path1",PointTypeFlags::LINK),
    (PathComp2F,b'2',"path2",PointTypeFlags::LINK),
    (PathComp3F,b'3',"path3",PointTypeFlags::LINK),
    (PathComp4F,b'4',"path4",PointTypeFlags::LINK),
    (PathComp5F,b'5',"path5",PointTypeFlags::LINK),
    (PathComp6F,b'6',"path6",PointTypeFlags::LINK),
    (PathComp7F,b'7',"path7",PointTypeFlags::LINK),
    (DataF,b'b',"data",PointTypeFlags::DATA)
]}

impl FieldEnum {
    pub fn mut_route(self, header: &mut NetPktHeader) -> Option<&mut [u8]> {
        Some(match self {
            FieldEnum::VarNetFlagsF => std::slice::from_mut(header.mut_flags_u8()),
            FieldEnum::VarHopF => &mut header.hop.0,
            FieldEnum::VarStampF => &mut header.stamp.0,
            FieldEnum::VarUBits0F => &mut header.ubits[0].0,
            FieldEnum::VarUBits1F => &mut header.ubits[1].0,
            FieldEnum::VarUBits2F => &mut header.ubits[2].0,
            FieldEnum::VarUBits3F => &mut header.ubits[3].0,
            _ => return None,
        })
    }
}

pub trait SFieldVal<Out>
where
    Self: FieldG + Copy + 'static,
{
    #[allow(clippy::needless_lifetimes)]
    fn get_val<'o, T: NetPkt + ?Sized>(pkt: &'o T) -> Out;
}
pub trait SFieldPtr<Out>
where
    Self: FieldG + Copy + 'static,
    Out: ?Sized,
{
    #[allow(clippy::needless_lifetimes)]
    fn get_ptr<'o, T: NetPkt + ?Sized>(pkt: &'o T) -> &'o Out;
}
macro_rules! field_val {
    ([$( ( $fname:ident,$out:ty, $getter:expr )),*]) => {
        $(
            impl SFieldVal<$out> for $fname {
                fn get_val<'o, T:NetPkt+?Sized>(pkt : &'o T) -> $out { $getter(pkt)}
            }
        )*
    };
}
macro_rules! field_ptr{
    ([$( ( $fname:ident,$out:ty, $getter:expr )),*]) => {
        $(
            impl SFieldPtr<$out> for $fname {
                fn get_ptr<'o, T:NetPkt+?Sized>(pkt : &'o T) -> &'o $out { $getter(pkt)}
            }
        )*
    };
}

field_val!([
    (VarNetFlagsF, u8, |pkt: &'o T| *pkt.net_header().flags_u8()),
    (VarHopF, u32, |pkt: &'o T| pkt.net_header().hop.get()),
    (VarStampF, u64, |pkt: &'o T| pkt.net_header().stamp.get()),
    (VarUBits0F, u32, |pkt: &'o T| pkt.net_header().ubits[0]
        .get()),
    (VarUBits1F, u32, |pkt: &'o T| pkt.net_header().ubits[1]
        .get()),
    (VarUBits2F, u32, |pkt: &'o T| pkt.net_header().ubits[2]
        .get()),
    (VarUBits3F, u32, |pkt: &'o T| pkt.net_header().ubits[3]
        .get()),
    (PktHashF, U256, |pkt: &'o T| pkt.hash().into()),
    (PktTypeF, u8, |pkt: &'o T| (pkt
        .as_point()
        .point_header()
        .point_type
        .bits)),
    (PointSizeF, u16, |pkt: &'o T| (pkt
        .as_point()
        .point_header()
        .point_size
        .get())),
    (DataSizeF, u16, |pkt: &'o T| pkt.as_point().data().len().try_into().expect("bug: impossible tail")),
    (
        LinksLenF,
        u16,
        |pkt: &'o T| pkt.as_point().get_links().len().try_into().expect("bug: impossible links")
    ),
    (CreateF, u64, |pkt: &'o T| pkt
        .as_point()
        .get_create_stamp()
        .get()),
    (GroupIDF, U256, |pkt: &'o T| (*pkt.as_point().get_group())
        .into()),
    (DomainF, u128, |pkt: &'o T| (*pkt.as_point().get_domain())
        .into()),
    (PubKeyF, U256, |pkt: &'o T| (*pkt.as_point().get_pubkey())
        .into()),
    (SignatureF, U512, |pkt: &'o T| (*pkt
        .as_point()
        .get_signature())
    .into()),
    (PathLenF, u8, |pkt: &'o T| *pkt.as_point().get_path_len())
]);

field_ptr!([
    (IPathF, IPath, |pkt: &'o T| pkt.as_point().get_ipath()),
    (PathF, SPath, |pkt: &'o T| pkt.as_point().get_path()),
    (PathComp0F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path0()),
    (PathComp1F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path1()),
    (PathComp2F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path2()),
    (PathComp3F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path3()),
    (PathComp4F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path4()),
    (PathComp5F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path5()),
    (PathComp6F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path6()),
    (PathComp7F, [u8], |pkt: &'o T| pkt
        .as_point()
        .get_ipath()
        .path7()),
    (VarNetFlagsF, NetFlags, |pkt: &'o T| &pkt
        .net_header_ref()
        .flags),
    (VarHopF, U32, |pkt: &'o T| &pkt.net_header_ref().hop),
    (VarStampF, Stamp, |pkt: &'o T| &pkt.net_header_ref().stamp),
    (VarUBits0F, U32, |pkt: &'o T| &pkt.net_header_ref().ubits[0]),
    (VarUBits1F, U32, |pkt: &'o T| &pkt.net_header_ref().ubits[1]),
    (VarUBits2F, U32, |pkt: &'o T| &pkt.net_header_ref().ubits[2]),
    (VarUBits3F, U32, |pkt: &'o T| &pkt.net_header_ref().ubits[3]),
    (PktHashF, LkHash, |pkt: &'o T| pkt.hash_ref()),
    (PktTypeF, PointTypeFlags, |pkt: &'o T| &pkt
        .as_point()
        .point_header_ref()
        .point_type),
    (PointSizeF, U16, |pkt: &'o T| &pkt
        .as_point()
        .point_header_ref()
        .point_size),
    (CreateF, Stamp, |pkt: &'o T| pkt
        .as_point()
        .get_create_stamp()),
    (GroupIDF, GroupID, |pkt: &'o T| pkt.as_point().get_group()),
    (DomainF, Domain, |pkt: &'o T| pkt.as_point().get_domain()),
    (PubKeyF, PubKey, |pkt: &'o T| pkt.as_point().get_pubkey()),
    (SignatureF, Signature, |pkt: &'o T| pkt
        .as_point()
        .get_signature()),
    (PathLenF, u8, |pkt: &'o T| pkt.as_point().get_path_len())
]);

impl FieldEnum {
    pub fn fixed_size(self) -> Option<usize> {
        let v = match self {
            FieldEnum::PathLenF | FieldEnum::PktTypeF | FieldEnum::VarNetFlagsF => 1,
            FieldEnum::LinksLenF | FieldEnum::DataSizeF | FieldEnum::PointSizeF => 2,
            FieldEnum::VarHopF
            | FieldEnum::VarUBits0F
            | FieldEnum::VarUBits1F
            | FieldEnum::VarUBits2F
            | FieldEnum::VarUBits3F => 4,
            FieldEnum::CreateF | FieldEnum::VarStampF => 8,
            FieldEnum::DomainF => 16,
            FieldEnum::GroupIDF | FieldEnum::PubKeyF | FieldEnum::PktHashF => 32,
            FieldEnum::SignatureF => 64,
            FieldEnum::DataF
            | FieldEnum::PathComp0F
            | FieldEnum::PathComp1F
            | FieldEnum::PathComp2F
            | FieldEnum::PathComp3F
            | FieldEnum::PathComp4F
            | FieldEnum::PathComp5F
            | FieldEnum::PathComp6F
            | FieldEnum::PathComp7F
            | FieldEnum::IPathF
            | FieldEnum::PathF => return None,
        };
        Some(v)
    }
    /// This always returns something even if the field doesn't exists for the specific pkt
    pub fn bytes(self, pkt: &dyn NetPkt, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            FieldEnum::VarNetFlagsF => {
                out.write_all(std::slice::from_ref(pkt.net_header().flags_u8()))
            }
            FieldEnum::VarHopF => out.write_all(&VarHopF::get_ptr(pkt).0),
            FieldEnum::VarStampF => out.write_all(&VarStampF::get_ptr(pkt).0),
            FieldEnum::VarUBits0F => out.write_all(&VarUBits0F::get_ptr(pkt).0),
            FieldEnum::VarUBits1F => out.write_all(&VarUBits1F::get_ptr(pkt).0),
            FieldEnum::VarUBits2F => out.write_all(&VarUBits2F::get_ptr(pkt).0),
            FieldEnum::VarUBits3F => out.write_all(&VarUBits3F::get_ptr(pkt).0),
            FieldEnum::PktHashF => out.write_all(&PktHashF::get_ptr(pkt).0),
            FieldEnum::PktTypeF => out.write_all(std::slice::from_ref(
                &pkt.as_point().point_header_ref().point_type.bits,
            )),
            FieldEnum::PointSizeF => out.write_all(&PointSizeF::get_ptr(pkt).0),
            FieldEnum::DataSizeF => out.write_all(&DataSizeF::get_val(pkt).to_be_bytes()),
            FieldEnum::LinksLenF => out.write_all(&LinksLenF::get_val(pkt).to_be_bytes()),
            FieldEnum::DomainF => out.write_all(&DomainF::get_ptr(pkt).0),
            FieldEnum::PathF => out.write_all(PathF::get_ptr(pkt).spath_bytes()),
            FieldEnum::IPathF => out.write_all(IPathF::get_ptr(pkt).ipath_bytes()),
            FieldEnum::PathLenF => {
                out.write_all(std::slice::from_ref(pkt.as_point().get_path_len()))
            }
            FieldEnum::PathComp0F => out.write_all(PathComp0F::get_ptr(pkt)),
            FieldEnum::PathComp1F => out.write_all(PathComp1F::get_ptr(pkt)),
            FieldEnum::PathComp2F => out.write_all(PathComp2F::get_ptr(pkt)),
            FieldEnum::PathComp3F => out.write_all(PathComp3F::get_ptr(pkt)),
            FieldEnum::PathComp4F => out.write_all(PathComp4F::get_ptr(pkt)),
            FieldEnum::PathComp5F => out.write_all(PathComp5F::get_ptr(pkt)),
            FieldEnum::PathComp6F => out.write_all(PathComp6F::get_ptr(pkt)),
            FieldEnum::PathComp7F => out.write_all(PathComp7F::get_ptr(pkt)),
            FieldEnum::GroupIDF => out.write_all(&GroupIDF::get_ptr(pkt).0),
            FieldEnum::CreateF => out.write_all(&CreateF::get_ptr(pkt).0),
            FieldEnum::PubKeyF => out.write_all(&PubKeyF::get_ptr(pkt).0),
            FieldEnum::SignatureF => out.write_all(&SignatureF::get_ptr(pkt).0),
            FieldEnum::DataF => out.write_all(pkt.as_point().data()),
        }
    }
    pub fn display(self, pkt: &dyn NetPkt, mut out: impl std::io::Write) -> anyhow::Result<()> {
        match self {
            FieldEnum::VarNetFlagsF => write!(out, "{:?}", VarNetFlagsF::get_ptr(pkt)),
            FieldEnum::VarHopF => write!(out, "{}", VarHopF::get_ptr(pkt)),
            FieldEnum::VarStampF => write!(out, "{}", VarStampF::get_ptr(pkt)),
            FieldEnum::VarUBits0F => write!(out, "{}", VarUBits0F::get_ptr(pkt)),
            FieldEnum::VarUBits1F => write!(out, "{}", VarUBits1F::get_ptr(pkt)),
            FieldEnum::VarUBits2F => write!(out, "{}", VarUBits2F::get_ptr(pkt)),
            FieldEnum::VarUBits3F => write!(out, "{}", VarUBits3F::get_ptr(pkt)),
            FieldEnum::PktHashF => write!(out, "{}", PktHashF::get_ptr(pkt)),
            FieldEnum::PktTypeF => write!(out, "{}", PktTypeF::get_ptr(pkt)),
            FieldEnum::PointSizeF => write!(out, "{}", PointSizeF::get_ptr(pkt)),
            FieldEnum::DataSizeF => write!(out, "{}", DataSizeF::get_val(pkt)),
            FieldEnum::LinksLenF => write!(out, "{}", LinksLenF::get_val(pkt)),
            FieldEnum::DomainF => write!(out, "{}", DomainF::get_ptr(pkt).as_str(true)),
            FieldEnum::PathF => write!(out, "{}", PathF::get_ptr(pkt)),
            FieldEnum::IPathF => write!(out, "{}", IPathF::get_ptr(pkt).spath()),// TODO might be wrong
            FieldEnum::PathLenF => write!(out, "{}", PathLenF::get_ptr(pkt)),
            FieldEnum::PathComp0F => write!(out, "{}", AB(PathComp0F::get_ptr(pkt))),
            FieldEnum::PathComp1F => write!(out, "{}", AB(PathComp1F::get_ptr(pkt))),
            FieldEnum::PathComp2F => write!(out, "{}", AB(PathComp2F::get_ptr(pkt))),
            FieldEnum::PathComp3F => write!(out, "{}", AB(PathComp3F::get_ptr(pkt))),
            FieldEnum::PathComp4F => write!(out, "{}", AB(PathComp4F::get_ptr(pkt))),
            FieldEnum::PathComp5F => write!(out, "{}", AB(PathComp5F::get_ptr(pkt))),
            FieldEnum::PathComp6F => write!(out, "{}", AB(PathComp6F::get_ptr(pkt))),
            FieldEnum::PathComp7F => write!(out, "{}", AB(PathComp7F::get_ptr(pkt))),
            FieldEnum::GroupIDF => write!(out, "{}", GroupIDF::get_ptr(pkt)),
            FieldEnum::CreateF => write!(out, "{}", CreateF::get_ptr(pkt)),
            FieldEnum::PubKeyF => write!(out, "{}", PubKeyF::get_ptr(pkt)),
            FieldEnum::SignatureF => write!(out, "{}", SignatureF::get_ptr(pkt)),
            FieldEnum::DataF => {
                let data = pkt.as_point().data();
                let data = std::str::from_utf8(data)?;
                out.write_all(data.as_bytes())
            }
        }?;
        Ok(())
    }

    pub fn into_abe(self, pkt: &dyn NetPkt) -> String {
        let mut v = vec![];
        self.abe(pkt, &mut v).unwrap();
        String::from_utf8(v).unwrap()
    }
    pub fn abe(self, pkt: &dyn NetPkt, mut out: impl std::io::Write) -> std::io::Result<()> {
        let string = match self {
            FieldEnum::VarNetFlagsF => print_abe(U8::new(VarNetFlagsF::get_val(pkt)).abe_bits()),
            FieldEnum::VarHopF => VarHopF::get_ptr(pkt).to_abe_str(),
            FieldEnum::VarStampF => VarStampF::get_ptr(pkt).to_abe_str(),
            FieldEnum::VarUBits0F => VarUBits0F::get_ptr(pkt).to_abe_str(),
            FieldEnum::VarUBits1F => VarUBits1F::get_ptr(pkt).to_abe_str(),
            FieldEnum::VarUBits2F => VarUBits2F::get_ptr(pkt).to_abe_str(),
            FieldEnum::VarUBits3F => VarUBits3F::get_ptr(pkt).to_abe_str(),
            FieldEnum::PktHashF => PktHashF::get_ptr(pkt).to_abe_str(),
            FieldEnum::PktTypeF => print_abe(U8::new(PktTypeF::get_val(pkt)).abe_bits()),
            FieldEnum::PointSizeF => PointSizeF::get_ptr(pkt).to_abe_str(),
            FieldEnum::DomainF => DomainF::get_ptr(pkt).to_abe_str(),
            FieldEnum::IPathF => PathF::get_ptr(pkt).to_abe_str(), // TODO might be the wrong choice
            FieldEnum::PathF => PathF::get_ptr(pkt).to_abe_str(),
            FieldEnum::PathLenF => U8::new(PathLenF::get_val(pkt)).to_abe_str(),
            FieldEnum::PathComp0F => AB(PathComp0F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp1F => AB(PathComp1F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp2F => AB(PathComp2F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp3F => AB(PathComp3F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp4F => AB(PathComp4F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp5F => AB(PathComp5F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp6F => AB(PathComp6F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::PathComp7F => AB(PathComp7F::get_ptr(pkt)).to_abe_str(),
            FieldEnum::GroupIDF => GroupIDF::get_ptr(pkt).to_abe_str(),
            FieldEnum::CreateF => CreateF::get_ptr(pkt).to_abe_str(),
            FieldEnum::PubKeyF => PubKeyF::get_ptr(pkt).to_abe_str(),
            FieldEnum::SignatureF => SignatureF::get_ptr(pkt).to_abe_str(),
            FieldEnum::DataF => format!("[:{}]", AB(pkt.as_point().data())),
            FieldEnum::LinksLenF => U16::new(LinksLenF::get_val(pkt)).to_abe_str(),
            FieldEnum::DataSizeF => U16::new(DataSizeF::get_val(pkt)).to_abe_str(),
        };
        out.write_all(string.as_bytes())
    }
}

#[test]
fn fields(){
    let ipath = ipath_buf(&[b"hello",b"world"]);
    let p = linkpoint([1;32].into(), [2;16].into(), &ipath, &[], &[], now(), ());
    let abe = abe::parse_abe("[path]").unwrap();
    let path = abe::eval::eval(&pkt_ctx(core_ctx(), &p),&abe).unwrap().into_exact_bytes().unwrap();
    let pbox = p.as_netbox();
    let path1 = abe::eval::eval(&pkt_ctx(core_ctx(), &pbox),&abe).unwrap().into_exact_bytes().unwrap();
    assert_eq!(path,ipath.spath_bytes());
    assert_eq!(path,path1)
}
