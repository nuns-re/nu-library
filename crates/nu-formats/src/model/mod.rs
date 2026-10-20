//! NUD model containers.

pub mod nud10;
pub mod nud20;
pub mod vertex;

pub use nud10::{Nud10, Nud10Chunk, Nud10Object, Nud10Primitive};
pub use nud20::{
    decode_r11g11b10, expand_strips, MeshData, Nud20, Nud20Header, Nud20Material,
    Nud20MaterialHeader, Nud20MaterialProperty, Nud20MaterialPropertyRecord, Nud20MaterialTexture,
    Nud20Object, Nud20ObjectRecord, Nud20Primitive, Nud20PrimitiveRecord, Nud20Sections,
};
pub use vertex::{
    Nud20VertexFormat, NudColorSize, NudSkinSize, NudUvFormat, VertexAttribute, VertexElement,
    VertexEncoding, VertexLayout,
};

use anyhow::Result;
use binrw::Endian;

/// A NUD mesh container, versioned by its magic.
#[derive(Debug, Clone)]
pub enum Nud {
    V1(Nud10),
    V2(Nud20),
}

impl Nud {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = crate::util::magic(bytes)?;
        match &magic {
            b"NDLX" | b"NDP3" => Ok(Self::V2(Nud20::parse(bytes)?)),
            b"3PUN" | b"NUP3" => Ok(Self::V1(Nud10::parse(bytes)?)),
            other => anyhow::bail!("unexpected NUD magic {:?}", String::from_utf8_lossy(other)),
        }
    }

    pub fn endian(&self) -> Endian {
        match self {
            Self::V1(nud) => nud.endian,
            Self::V2(nud) => nud.endian,
        }
    }
}
