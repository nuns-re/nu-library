//! NU shader bank reader.

use anyhow::{bail, ensure, Context, Result};
use binrw::BinRead;

use crate::util::{bytes_at, cstring_at, magic, read_at};

/// 0x20-byte `NshHeader`.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NshHeader {
    pub file_id: [u8; 4],
    pub format_version: u8,
    pub format_id: u8,
    pub reserved06: u16,
    pub reserved08: u32,
    pub number_of_objects: u16,
    pub reserved0c: [u8; 0x12],
}

/// 0x20-byte `NshObject`.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NshObjectRecord {
    pub shader_index: u32,
    pub attribute: u32,
    pub vertex_shader_data: u32,
    pub pixel_shader_data: u32,
    pub vertex_shader_param: u32,
    pub pixel_shader_param: u32,
    pub total_size: u32,
    pub reserved1c: u32,
}

/// 0x0c-byte `NshChunk` / `NshParameterChunk`.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NshChunkRecord {
    pub data_offset: u32,
    pub data_num: u32,
    pub data_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NshChunk {
    /// Absolute offset of the chunk record.
    pub offset: usize,
    pub record: NshChunkRecord,
}

impl NshChunk {
    pub fn data<'a>(&self, data: &'a [u8], object_offset: usize) -> Result<&'a [u8]> {
        let start = object_offset
            .checked_add(self.record.data_offset as usize)
            .context("NSH chunk offset overflow")?;
        bytes_at(data, start, self.record.data_size as usize)
            .with_context(|| format!("NSH chunk at {:#x}", self.offset))
    }
}

/// 0x20-byte `NshParameter`.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NshParameterRecord {
    pub name_offset: u32,
    pub value_offset: u32,
    pub parameter_type_offset: u32,
    pub reserved0c: u32,
    pub reserved10: u32,
    pub number_of_parameter: u32,
    pub reserved18: u32,
    pub reserved1c: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NshParameter {
    pub offset: usize,
    pub name: String,
    pub type_name: String,
    pub value_offset: u32,
    pub number_of_parameter: u32,
    pub record: NshParameterRecord,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NshObject {
    pub offset: usize,
    pub record: NshObjectRecord,
    pub vertex_shader: Option<NshChunk>,
    pub pixel_shader: Option<NshChunk>,
    pub vertex_parameters: Vec<NshParameter>,
    pub pixel_parameters: Vec<NshParameter>,
}

impl NshObject {
    pub fn vertex_shader_bytes<'a>(&self, data: &'a [u8]) -> Result<Option<&'a [u8]>> {
        self.vertex_shader
            .as_ref()
            .map(|chunk| chunk.data(data, self.offset))
            .transpose()
    }

    pub fn pixel_shader_bytes<'a>(&self, data: &'a [u8]) -> Result<Option<&'a [u8]>> {
        self.pixel_shader
            .as_ref()
            .map(|chunk| chunk.data(data, self.offset))
            .transpose()
    }
}

/// A parsed NSH file.
#[derive(Debug, Clone)]
pub struct Nsh {
    data: Vec<u8>,
    pub endian: binrw::Endian,
    pub header: NshHeader,
    pub objects: Vec<NshObject>,
}

impl Nsh {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = magic(bytes)?;
        let endian = match &magic {
            b"NSLX" => binrw::Endian::Little,
            other => bail!("unexpected NSH magic {:?}", String::from_utf8_lossy(other)),
        };
        let header: NshHeader = read_at(bytes, 0, endian)?;
        ensure!(
            header.number_of_objects <= 4096,
            "implausible NSH object count {}",
            header.number_of_objects
        );

        let mut objects = Vec::with_capacity(header.number_of_objects as usize);
        let mut cursor = 0x20usize;
        for index in 0..header.number_of_objects as usize {
            let record: NshObjectRecord = read_at(bytes, cursor, endian)
                .with_context(|| format!("NSH object {index} at {cursor:#x}"))?;
            ensure!(
                record.total_size >= 0x20,
                "NSH object {index} total size {:#x} is too small",
                record.total_size
            );
            let end = cursor
                .checked_add(record.total_size as usize)
                .context("NSH object size overflow")?;
            ensure!(end <= bytes.len(), "NSH object {index} exceeds buffer");

            let vertex_shader = read_chunk(bytes, cursor, record.vertex_shader_data, endian)?;
            let pixel_shader = read_chunk(bytes, cursor, record.pixel_shader_data, endian)?;
            let vertex_parameters =
                read_parameters(bytes, cursor, record.vertex_shader_param, endian)?;
            let pixel_parameters =
                read_parameters(bytes, cursor, record.pixel_shader_param, endian)?;

            objects.push(NshObject {
                offset: cursor,
                record,
                vertex_shader,
                pixel_shader,
                vertex_parameters,
                pixel_parameters,
            });
            cursor = end;
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

fn read_chunk(
    data: &[u8],
    object_offset: usize,
    relative: u32,
    endian: binrw::Endian,
) -> Result<Option<NshChunk>> {
    if relative == 0 {
        return Ok(None);
    }
    let offset = object_offset
        .checked_add(relative as usize)
        .context("NSH chunk offset overflow")?;
    let record: NshChunkRecord = read_at(data, offset, endian)?;
    ensure!(
        record.data_size <= 512 * 1024 * 1024,
        "implausible NSH chunk size {}",
        record.data_size
    );
    Ok(Some(NshChunk { offset, record }))
}

fn read_parameters(
    data: &[u8],
    object_offset: usize,
    relative: u32,
    endian: binrw::Endian,
) -> Result<Vec<NshParameter>> {
    if relative == 0 {
        return Ok(Vec::new());
    }
    let chunk_offset = object_offset
        .checked_add(relative as usize)
        .context("NSH parameter chunk offset overflow")?;
    let chunk: NshChunkRecord = read_at(data, chunk_offset, endian)?;
    ensure!(
        chunk.data_num <= 4096,
        "implausible NSH parameter count {}",
        chunk.data_num
    );
    let array_offset = object_offset
        .checked_add(chunk.data_offset as usize)
        .context("NSH parameter array offset overflow")?;
    let mut parameters = Vec::with_capacity(chunk.data_num as usize);
    for index in 0..chunk.data_num as usize {
        let offset = array_offset + index * 0x20;
        let record: NshParameterRecord = read_at(data, offset, endian)
            .with_context(|| format!("NSH parameter {index} at {offset:#x}"))?;
        let name = if record.name_offset == 0 {
            String::new()
        } else {
            cstring_at(data, object_offset + record.name_offset as usize)?
        };
        let type_name = if record.parameter_type_offset == 0 {
            String::new()
        } else {
            cstring_at(data, object_offset + record.parameter_type_offset as usize)?
        };
        parameters.push(NshParameter {
            offset,
            name,
            type_name,
            value_offset: record.value_offset,
            number_of_parameter: record.number_of_parameter,
            record,
        });
    }
    Ok(parameters)
}
