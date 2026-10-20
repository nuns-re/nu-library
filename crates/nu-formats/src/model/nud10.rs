//! NUD 1.0 model reader.

use anyhow::{bail, ensure, Context, Result};
use binrw::BinRead;

use crate::util::{magic, read_at};

/// 0x20-byte NUD10 file header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud10Header {
    pub file_id: [u8; 4],
    pub file_size: u32,
    pub version: u16,
    pub number_of_objects: u16,
    pub reserved: [u8; 0x14],
}

/// 0x20-byte NUD10 object header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud10ObjectHeader {
    pub reserved00: [u8; 8],
    pub primitive_count: u32,
    pub reserved0c: [u8; 0x14],
}

/// 0x20-byte NUD10 primitive header (file/original layout).
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud10PrimitiveHeader {
    pub reserved00: [u8; 0x14],
    pub chunk_count: u32,
    pub reserved18: [u8; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nud10Chunk {
    pub offset: usize,
    pub header: [u8; 0x20],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nud10Primitive {
    pub offset: usize,
    pub header: Nud10PrimitiveHeader,
    pub chunks: Vec<Nud10Chunk>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nud10Object {
    pub offset: usize,
    pub header: Nud10ObjectHeader,
    pub primitives: Vec<Nud10Primitive>,
}

/// A parsed NUD10 file.
#[derive(Debug, Clone)]
pub struct Nud10 {
    data: Vec<u8>,
    pub endian: binrw::Endian,
    pub header: Nud10Header,
    pub objects: Vec<Nud10Object>,
}

impl Nud10 {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = magic(bytes)?;
        let endian = match &magic {
            b"3PUN" => binrw::Endian::Little,
            b"NUP3" => binrw::Endian::Big,
            other => bail!(
                "unexpected NUD10 magic {:?}",
                String::from_utf8_lossy(other)
            ),
        };
        let header: Nud10Header = read_at(bytes, 0, endian)?;
        ensure!(
            header.number_of_objects <= 4096,
            "implausible NUD10 object count {}",
            header.number_of_objects
        );
        ensure!(
            header.file_size as usize <= bytes.len(),
            "NUD10 file size {:#x} exceeds {:#x}",
            header.file_size,
            bytes.len()
        );

        let mut cursor = 0x20usize;
        let mut objects = Vec::with_capacity(header.number_of_objects as usize);
        for object_index in 0..header.number_of_objects as usize {
            let object_offset = cursor;
            let object: Nud10ObjectHeader = read_at(bytes, object_offset, endian)
                .with_context(|| format!("NUD10 object {object_index} at {object_offset:#x}"))?;
            ensure!(
                object.primitive_count <= 4096,
                "implausible NUD10 primitive count {}",
                object.primitive_count
            );
            cursor = cursor
                .checked_add(0x20)
                .context("NUD10 object offset overflow")?;
            let mut primitives = Vec::with_capacity(object.primitive_count as usize);
            for primitive_index in 0..object.primitive_count as usize {
                let primitive_offset = cursor;
                let primitive: Nud10PrimitiveHeader = read_at(bytes, primitive_offset, endian)
                    .with_context(|| {
                        format!("NUD10 primitive {primitive_index} at {primitive_offset:#x}")
                    })?;
                ensure!(
                    primitive.chunk_count <= 4096,
                    "implausible NUD10 chunk count {}",
                    primitive.chunk_count
                );
                cursor = cursor
                    .checked_add(0x20)
                    .context("NUD10 primitive offset overflow")?;
                let mut chunks = Vec::with_capacity(primitive.chunk_count as usize);
                for chunk_index in 0..primitive.chunk_count as usize {
                    let chunk_offset = cursor;
                    let header =
                        read_at::<[u8; 0x20]>(bytes, chunk_offset, endian).with_context(|| {
                            format!("NUD10 chunk {chunk_index} at {chunk_offset:#x}")
                        })?;
                    chunks.push(Nud10Chunk {
                        offset: chunk_offset,
                        header,
                    });
                    cursor = cursor
                        .checked_add(0x20)
                        .context("NUD10 chunk offset overflow")?;
                }
                ensure!(cursor <= bytes.len(), "NUD10 data ends outside buffer");
                primitives.push(Nud10Primitive {
                    offset: primitive_offset,
                    header: primitive,
                    chunks,
                });
            }
            objects.push(Nud10Object {
                offset: object_offset,
                header: object,
                primitives,
            });
        }

        Ok(Self {
            data: bytes.to_vec(),
            endian,
            header,
            objects,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
