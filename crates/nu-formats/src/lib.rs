//! NU asset formats.

mod util;

pub mod audio;
pub mod font;
pub mod model;
pub mod shader;
pub mod texture;

pub use binrw::Endian;
pub use nu_core::{NuError, NuFormat, Result};

use nu_core::LoadOptions;

#[derive(Debug, Clone)]
pub enum NuAsset {
    Model(model::Nud),
    Texture(texture::Nut),
    Shader(shader::Nsh),
    Audio(audio::Nus2Bank),
    Font(Box<font::Nfh>),
}

pub fn detect(bytes: &[u8]) -> Option<(&'static str, Endian)> {
    let magic = bytes.get(..4)?;
    match magic {
        b"NDLX" => Some(("NUD20", Endian::Little)),
        b"NDP3" => Some(("NUD20", Endian::Big)),
        b"3PUN" => Some(("NUD10", Endian::Little)),
        b"NUP3" => Some(("NUD10", Endian::Big)),
        b"GIDX" => Some(("NUD20-index", Endian::Little)),
        b"NTLX" => Some(("NUT", Endian::Little)),
        b"NTP3" => Some(("NUT", Endian::Big)),
        b"NSLX" => Some(("NSH", Endian::Little)),
        b"NFH\0" => Some(("NFH", Endian::Little)),
        _ if audio::Nus2Bank::probe(bytes) => {
            let endian = if bytes.first() == Some(&1) {
                Endian::Little
            } else {
                Endian::Big
            };
            Some(("NUB", endian))
        }
        _ => None,
    }
}

pub fn parse(bytes: &[u8]) -> Result<NuAsset> {
    let magic = bytes.get(..4).ok_or(NuError::Truncated("magic"))?;
    match magic {
        b"NDLX" | b"NDP3" | b"3PUN" | b"NUP3" => Ok(NuAsset::Model(model::Nud::parse(bytes)?)),
        b"GIDX" => {
            let inner = bytes
                .get(0x10..)
                .ok_or(NuError::Truncated("GIDX wrapper"))?;
            Ok(NuAsset::Model(model::Nud::parse(inner)?))
        }
        b"NTLX" | b"NTP3" => Ok(NuAsset::Texture(texture::Nut::parse(bytes)?)),
        b"NSLX" => Ok(NuAsset::Shader(shader::Nsh::parse(bytes)?)),
        b"NFH\0" => Ok(NuAsset::Font(Box::new(font::Nfh::parse(bytes)?))),
        _ if audio::Nus2Bank::probe(bytes) => Ok(NuAsset::Audio(audio::Nus2Bank::parse(bytes)?)),
        _ => Err(NuError::Unsupported),
    }
}

impl NuFormat for model::Nud {
    const NAME: &'static str = "nud";

    fn probe(bytes: &[u8]) -> bool {
        matches!(bytes.get(..4), Some(b"NDLX" | b"NDP3" | b"3PUN" | b"NUP3"))
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        Self::parse(bytes).map_err(Into::into)
    }
}

impl NuFormat for texture::Nut {
    const NAME: &'static str = "nut";

    fn probe(bytes: &[u8]) -> bool {
        matches!(bytes.get(..4), Some(b"NTLX" | b"NTP3"))
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        Self::parse(bytes).map_err(Into::into)
    }
}

impl NuFormat for shader::Nsh {
    const NAME: &'static str = "nsh";

    fn probe(bytes: &[u8]) -> bool {
        bytes.starts_with(b"NSLX")
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        Self::parse(bytes).map_err(Into::into)
    }
}

impl NuFormat for font::Nfh {
    const NAME: &'static str = "nfh";

    fn probe(bytes: &[u8]) -> bool {
        font::Nfh::probe(bytes)
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        Self::parse(bytes).map_err(Into::into)
    }
}

impl NuFormat for audio::Nus2Bank {
    const NAME: &'static str = "nub";

    fn probe(bytes: &[u8]) -> bool {
        audio::Nus2Bank::probe(bytes)
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        Self::parse(bytes).map_err(Into::into)
    }
}

impl NuFormat for NuAsset {
    const NAME: &'static str = "asset";

    fn probe(bytes: &[u8]) -> bool {
        detect(bytes).is_some()
    }

    fn from_bytes(bytes: &[u8], _options: &LoadOptions) -> Result<Self> {
        parse(bytes)
    }
}
