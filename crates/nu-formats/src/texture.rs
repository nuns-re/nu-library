//! NUT texture reader.

use anyhow::{bail, ensure, Context, Result};
use binrw::BinRead;

use crate::util::{bytes_at, magic, read_at};

/// NUT file header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NutFileHeader {
    pub file_id: [u8; 4],
    pub format_version: u8,
    pub format_id: u8,
    pub pictures: u16,
    pub pad: [u8; 8],
}

/// NUT picture header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NutPictureHeader {
    pub total_size: u32,
    pub clut_size: u32,
    pub image_size: u32,
    pub header_size: u16,
    pub clut_colors: u16,
    pub pict_format: u8,
    pub mipmap_textures: u8,
    pub clut_type: u8,
    pub image_type: u8,
    pub width: u16,
    pub height: u16,
    pub caps1: u32,
    pub caps2: u32,
    pub image_offset: u32,
    pub reserved: [u8; 12],
}

impl NutPictureHeader {
    pub fn is_cube_map(&self) -> bool {
        self.caps2 & 0x200 != 0
    }

    pub fn has_clut(&self) -> bool {
        self.clut_type & 0x1f != 0
    }
}

/// The native-format table entry selected by `ImageType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NutImageType {
    pub raw: u8,
    pub native_format: u32,
    pub texel_pitch: u32,
}

impl NutImageType {
    pub const NATIVE_FORMATS: [(u32, u32); 22] = [
        (0x83F1, 2),
        (0x83F2, 4),
        (0x83F3, 4),
        (0x885E, 2),
        (0x885F, 2),
        (0x8040, 1),
        (0x8057, 2),
        (0x8056, 2),
        (0x1907, 2),
        (0, 2),
        (0, 2),
        (0x803E, 2),
        (0x8057, 2),
        (0x8057, 2),
        (0x8058, 4),
        (0x8048, 4),
        (0x8816, 4),
        (0x8058, 4),
        (0x881F, 4),
        (0x881A, 8),
        (0x8814, 16),
        (0x8045, 2),
    ];

    pub fn new(raw: u8) -> Result<Self> {
        let (native_format, texel_pitch) = *Self::NATIVE_FORMATS
            .get(raw as usize)
            .ok_or_else(|| anyhow::anyhow!("unsupported NUT ImageType {raw:#x}"))?;
        Ok(Self {
            raw,
            native_format,
            texel_pitch,
        })
    }

    /// BC1/DXT1 (`ImageType 0`).
    pub fn is_bc1(self) -> bool {
        self.raw == 0
    }

    /// BC2/DXT3 (`ImageType 1`).
    pub fn is_bc2(self) -> bool {
        self.raw == 1
    }

    /// BC3/DXT5 (`ImageType 2`).
    pub fn is_bc3(self) -> bool {
        self.raw == 2
    }

    pub fn is_supported_by_current_decoder(self) -> bool {
        matches!(self.raw, 0 | 1 | 2 | 6 | 7 | 8 | 14 | 17)
    }
}

/// One picture in a NUT file.
#[derive(Debug, Clone, PartialEq)]
pub struct NutPicture {
    pub offset: usize,
    pub header: NutPictureHeader,
    /// Cube-map face info.
    pub cube_map_image_size: Option<u32>,
    /// Per-level byte sizes; empty for a single-level picture.
    pub mipmap_sizes: Vec<u32>,
}

impl NutPicture {
    pub fn image_type(&self) -> Result<NutImageType> {
        NutImageType::new(self.header.image_type)
    }

    /// Absolute start of the image payload.
    pub fn image_top(&self, format_version: u8) -> Result<usize> {
        let relative = if format_version >= 2 {
            self.header.image_offset
        } else {
            self.header.header_size as u32
        };
        Ok(self.offset + relative as usize)
    }

    /// Byte sizes of the stored levels.
    pub fn level_sizes(&self) -> Vec<u32> {
        if self.mipmap_sizes.is_empty() {
            vec![self.header.image_size]
        } else {
            self.mipmap_sizes.clone()
        }
    }

    /// Raw level slices, base level first.
    pub fn levels<'a>(&self, data: &'a [u8], format_version: u8) -> Result<Vec<&'a [u8]>> {
        let mut pos = self.image_top(format_version)?;
        let mut out = Vec::new();
        for size in self.level_sizes() {
            let level = bytes_at(data, pos, size as usize)
                .with_context(|| format!("NUT level at {pos:#x}"))?;
            out.push(level);
            pos = pos
                .checked_add(size as usize)
                .context("NUT level offset overflow")?;
        }
        Ok(out)
    }

    /// Six cube-map faces for the base level. Multi-mip cube interleaving is
    /// not assumed; callers can use `levels()` for the raw payload.
    pub fn faces<'a>(&self, data: &'a [u8], format_version: u8) -> Result<Vec<&'a [u8]>> {
        ensure!(
            self.header.is_cube_map(),
            "picture at {:#x} is not a cube map",
            self.offset
        );
        let payload = self
            .levels(data, format_version)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("cube map has no image level"))?;
        ensure!(
            payload.len() % 6 == 0,
            "cube map payload {} is not divisible by six",
            payload.len()
        );
        let face_size = payload.len() / 6;
        Ok(payload.chunks_exact(face_size).collect())
    }
}

/// A parsed NUT file.
#[derive(Debug, Clone)]
pub struct Nut {
    data: Vec<u8>,
    pub endian: binrw::Endian,
    pub header: NutFileHeader,
    pub pictures: Vec<NutPicture>,
}

impl Nut {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = magic(bytes)?;
        let endian = match &magic {
            b"NTLX" => binrw::Endian::Little,
            b"NTP3" => binrw::Endian::Big,
            other => bail!("unexpected NUT magic {:?}", String::from_utf8_lossy(other)),
        };
        let header: NutFileHeader = read_at(bytes, 0, endian)?;
        ensure!(header.pictures > 0, "NUT has no pictures");
        ensure!(header.pictures <= 4096, "implausible NUT picture count");

        let mut pictures = Vec::with_capacity(header.pictures as usize);
        let mut cursor = 0x10usize;
        for index in 0..header.pictures as usize {
            let header: NutPictureHeader = read_at(bytes, cursor, endian)
                .with_context(|| format!("NUT picture {index} at {cursor:#x}"))?;
            ensure!(
                header.total_size >= 0x30,
                "NUT picture {index} total size {:#x} is too small",
                header.total_size
            );
            let end = cursor
                .checked_add(header.total_size as usize)
                .context("NUT picture size overflow")?;
            ensure!(end <= bytes.len(), "NUT picture {index} exceeds buffer");

            let (cube_map_image_size, mipmap_sizes) =
                parse_mipmap_table(bytes, cursor, &header, endian)?;
            pictures.push(NutPicture {
                offset: cursor,
                header,
                cube_map_image_size,
                mipmap_sizes,
            });
            cursor = end;
        }

        Ok(Self {
            data: bytes.to_vec(),
            endian,
            header,
            pictures,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn levels<'a>(&'a self, picture: &NutPicture) -> Result<Vec<&'a [u8]>> {
        picture.levels(&self.data, self.header.format_version)
    }

    pub fn faces<'a>(&'a self, picture: &NutPicture) -> Result<Vec<&'a [u8]>> {
        picture.faces(&self.data, self.header.format_version)
    }
}

fn parse_mipmap_table(
    data: &[u8],
    picture_offset: usize,
    header: &NutPictureHeader,
    endian: binrw::Endian,
) -> Result<(Option<u32>, Vec<u32>)> {
    let cube = header.is_cube_map();
    let cube_size = if cube {
        Some(read_at::<u32>(data, picture_offset + 0x30, endian)?)
    } else {
        None
    };
    let count = header.mipmap_textures as usize;
    if count <= 1 {
        return Ok((cube_size, Vec::new()));
    }
    let table_offset = if cube {
        picture_offset + 0x40
    } else {
        picture_offset + 0x30
    };
    let mut sizes = Vec::with_capacity(count);
    for i in 0..count {
        sizes.push(read_at::<u32>(data, table_offset + i * 4, endian)?);
    }
    Ok((cube_size, sizes))
}
