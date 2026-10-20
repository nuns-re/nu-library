//! Small binrw helpers for absolute-offset reads.

use anyhow::{anyhow, Context, Result};
use binrw::{BinRead, BinReaderExt, Endian, NullString};
use std::io::{Cursor, Seek, SeekFrom};

pub(crate) fn magic(bytes: &[u8]) -> Result<[u8; 4]> {
    let slice = bytes
        .get(..4)
        .ok_or_else(|| anyhow!("file is shorter than four bytes"))?;
    Ok([slice[0], slice[1], slice[2], slice[3]])
}

pub(crate) fn bytes_at(data: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| anyhow!("offset overflow"))?;
    data.get(offset..end)
        .ok_or_else(|| anyhow!("range {offset:#x}..{end:#x} outside buffer"))
}

/// Read a binrw type at an absolute file offset with runtime endianness.
pub(crate) fn read_at<T: BinRead>(data: &[u8], offset: usize, endian: Endian) -> Result<T>
where
    T::Args: Default,
{
    read_at_args(data, offset, endian, T::Args::default())
}

/// Read a binrw type with explicit arguments at an absolute file offset.
pub(crate) fn read_at_args<T: BinRead>(
    data: &[u8],
    offset: usize,
    endian: Endian,
    args: T::Args,
) -> Result<T> {
    let mut cursor = Cursor::new(data);
    cursor
        .seek(SeekFrom::Start(offset as u64))
        .with_context(|| format!("seek to {offset:#x}"))?;
    cursor
        .read_type_args(endian, args)
        .with_context(|| format!("read at {offset:#x}"))
}

/// Read a NUL-terminated UTF-8 string at an absolute offset.
pub(crate) fn cstring_at(data: &[u8], offset: usize) -> Result<String> {
    let null: NullString = read_at(data, offset, Endian::Little)?;
    String::from_utf8(null.0).with_context(|| format!("string at {offset:#x} is not UTF-8"))
}
