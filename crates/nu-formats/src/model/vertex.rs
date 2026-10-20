//! NUD20 vertex layouts.

use anyhow::{bail, Result};

/// Encoding of one vertex element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexEncoding {
    F32,
    F16,
    /// `GL_UNSIGNED_INT_10F_11F_11F_REV`: three unsigned floats packed into a u32.
    R11G11B10,
}

/// Semantic of one vertex element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexAttribute {
    Position,
    Normal,
    Binormal,
    Tangent,
}

/// One element of the main vertex stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexElement {
    pub attribute: VertexAttribute,
    pub offset: usize,
    pub encoding: VertexEncoding,
    pub components: u8,
}

/// Position/normal stream layout for `VertexTypeVector & 0x0f`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexLayout {
    pub stride: usize,
    pub position: VertexElement,
    pub normal: Option<VertexElement>,
    pub binormal: Option<VertexElement>,
    pub tangent: Option<VertexElement>,
}

const fn elem(
    attribute: VertexAttribute,
    offset: usize,
    encoding: VertexEncoding,
    components: u8,
) -> VertexElement {
    VertexElement {
        attribute,
        offset,
        encoding,
        components,
    }
}

const P32: VertexLayout = VertexLayout {
    stride: 0x10,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: None,
    binormal: None,
    tangent: None,
};
const P32N32: VertexLayout = VertexLayout {
    stride: 0x20,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x10, VertexEncoding::F32, 3)),
    binormal: None,
    tangent: None,
};
const P32NB32: VertexLayout = VertexLayout {
    stride: 0x30,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x10, VertexEncoding::F32, 3)),
    binormal: Some(elem(
        VertexAttribute::Binormal,
        0x20,
        VertexEncoding::F32,
        3,
    )),
    tangent: None,
};
const P32NBT32: VertexLayout = VertexLayout {
    stride: 0x40,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x10, VertexEncoding::F32, 3)),
    binormal: Some(elem(
        VertexAttribute::Binormal,
        0x20,
        VertexEncoding::F32,
        3,
    )),
    tangent: Some(elem(VertexAttribute::Tangent, 0x30, VertexEncoding::F32, 3)),
};
const P32N11: VertexLayout = VertexLayout {
    stride: 0x10,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(
        VertexAttribute::Normal,
        0x0c,
        VertexEncoding::R11G11B10,
        3,
    )),
    binormal: None,
    tangent: None,
};
const P32NBT11: VertexLayout = VertexLayout {
    stride: 0x18,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(
        VertexAttribute::Normal,
        0x0c,
        VertexEncoding::R11G11B10,
        3,
    )),
    binormal: Some(elem(
        VertexAttribute::Binormal,
        0x10,
        VertexEncoding::R11G11B10,
        3,
    )),
    tangent: Some(elem(
        VertexAttribute::Tangent,
        0x14,
        VertexEncoding::R11G11B10,
        3,
    )),
};
const P32N16: VertexLayout = VertexLayout {
    stride: 0x14,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x0c, VertexEncoding::F16, 3)),
    binormal: None,
    tangent: None,
};
const P32NBT16: VertexLayout = VertexLayout {
    stride: 0x24,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F32, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x0c, VertexEncoding::F16, 3)),
    binormal: Some(elem(
        VertexAttribute::Binormal,
        0x14,
        VertexEncoding::F16,
        3,
    )),
    tangent: Some(elem(VertexAttribute::Tangent, 0x1c, VertexEncoding::F16, 3)),
};
const P16N16: VertexLayout = VertexLayout {
    stride: 0x10,
    position: elem(VertexAttribute::Position, 0, VertexEncoding::F16, 3),
    normal: Some(elem(VertexAttribute::Normal, 0x08, VertexEncoding::F16, 3)),
    binormal: None,
    tangent: None,
};

/// Skin width from `VertexTypeVector >> 4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NudSkinSize {
    None,
    Float,
    Half,
    Byte,
}

impl NudSkinSize {
    pub fn from_high_nibble(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::None,
            1 => Self::Float,
            2 => Self::Half,
            4 => Self::Byte,
            other => bail!("unsupported NUD skin width nibble {other:#x}"),
        })
    }

    pub fn joint_bytes(self) -> usize {
        match self {
            Self::None => 0,
            Self::Float => 4,
            Self::Half => 2,
            Self::Byte => 1,
        }
    }

    pub fn weight_bytes(self) -> usize {
        self.joint_bytes()
    }

    pub fn size(self) -> usize {
        if self == Self::None {
            0
        } else {
            8 * self.joint_bytes()
        }
    }
}

/// Color width of the color/UV stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NudColorSize {
    None,
    U8,
    U16,
}

impl NudColorSize {
    pub fn bytes(self) -> usize {
        match self {
            Self::None => 0,
            Self::U8 => 4,
            Self::U16 => 8,
        }
    }
}

/// `VertexTypeTexCoord`: high nibble = UV set count, low nibble = color/UV variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NudUvFormat(u8);

impl NudUvFormat {
    pub fn new(raw: u8) -> Result<Self> {
        let sets = raw >> 4;
        if sets > 4 {
            bail!("unsupported NUD UV set count {sets}");
        }
        if raw & 0x0f > 5 {
            bail!("unsupported NUD UV variant {:#x}", raw & 0x0f);
        }
        Ok(Self(raw))
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn sets(self) -> usize {
        (self.0 >> 4) as usize
    }

    pub fn color_size(self) -> NudColorSize {
        match (self.0 >> 1) & 3 {
            0 => NudColorSize::None,
            1 => NudColorSize::U8,
            _ => NudColorSize::U16,
        }
    }

    pub fn uv_float(self) -> bool {
        self.0 & 1 != 0
    }

    pub fn uv_bytes(self) -> usize {
        if self.uv_float() {
            8
        } else {
            4
        }
    }

    pub fn stride(self) -> usize {
        self.color_size().bytes() + self.sets() * self.uv_bytes()
    }

    pub fn color_offset(self) -> usize {
        0
    }

    pub fn uv_offset(self, set: usize) -> usize {
        self.color_size().bytes() + set * self.uv_bytes()
    }
}

/// `VertexTypeVector` + `VertexTypeTexCoord` decoded together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nud20VertexFormat {
    pub vector: u8,
    pub texcoord: u8,
    pub low_nibble: u8,
    pub skin_size: NudSkinSize,
    pub uv: NudUvFormat,
}

impl Nud20VertexFormat {
    pub fn new(vector: u8, texcoord: u8) -> Result<Self> {
        let low_nibble = vector & 0x0f;
        if low_nibble > 8 {
            bail!("unsupported NUD vertex format {low_nibble}");
        }
        Ok(Self {
            vector,
            texcoord,
            low_nibble,
            skin_size: NudSkinSize::from_high_nibble(vector >> 4)?,
            uv: NudUvFormat::new(texcoord)?,
        })
    }

    pub fn layout(self) -> &'static VertexLayout {
        match self.low_nibble {
            0 => &P32,
            1 => &P32N32,
            2 => &P32NB32,
            3 => &P32NBT32,
            4 => &P32N11,
            5 => &P32NBT11,
            6 => &P32N16,
            7 => &P32NBT16,
            8 => &P16N16,
            _ => unreachable!("validated in Nud20VertexFormat::new"),
        }
    }

    pub fn is_skinned(self) -> bool {
        self.skin_size != NudSkinSize::None
    }
}
