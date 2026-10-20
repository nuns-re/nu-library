//! NFH bitmap font reader.

use anyhow::{ensure, Context, Result};
use binrw::{BinRead, Endian};

use crate::util::read_at;

const HEADER_SIZE: usize = 0x450;
const ENTRY_SIZE: usize = 0x20;
const MAX_ENTRIES: i32 = 1_000_000;

/// NFH header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NfhHeader {
    pub file_id: [u8; 4],
    pub font_height: i32,
    pub entry_count: i32,
    pub padding: [u8; 4],
    pub family_name: [u8; 32],
    pub style_name: [u8; 32],
    pub offsets: [i32; 256],
}

impl NfhHeader {
    pub fn family(&self) -> String {
        c_string(&self.family_name)
    }

    pub fn style(&self) -> String {
        c_string(&self.style_name)
    }
}

/// NFH glyph entry.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct NfhEntry {
    pub glyph_id: i32,
    pub u: u16,
    pub v: u16,
    pub h_bearing_x: i16,
    pub h_bearing_y: i16,
    pub h_advance: i16,
    pub v_bearing_x: i16,
    pub v_bearing_y: i16,
    pub v_advance: i16,
    pub width: u8,
    pub height: u8,
    pub first: u8,
    pub second: u8,
    pub previous: u8,
    pub next: u8,
    pub padding: [u8; 6],
}

impl NfhEntry {
    pub fn code(&self) -> u16 {
        u16::from(self.first) | (u16::from(self.second) << 8)
    }
}

/// Parsed NFH font.
#[derive(Debug, Clone)]
pub struct Nfh {
    data: Vec<u8>,
    pub endian: Endian,
    pub header: NfhHeader,
    pub entries: Vec<NfhEntry>,
}

impl Nfh {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        Self::parse_with_endian(bytes, Endian::Little)
            .or_else(|_| Self::parse_with_endian(bytes, Endian::Big))
    }

    pub fn parse_with_endian(bytes: &[u8], endian: Endian) -> Result<Self> {
        ensure!(bytes.len() >= HEADER_SIZE, "NFH is shorter than its header");
        let header: NfhHeader = read_at(bytes, 0, endian).context("NFH header")?;
        ensure!(&header.file_id == b"NFH\0", "unexpected NFH magic");
        ensure!(header.font_height > 0, "invalid NFH font height");
        ensure!(
            (0..=MAX_ENTRIES).contains(&header.entry_count),
            "invalid NFH entry count {}",
            header.entry_count
        );

        let count = header.entry_count as usize;
        let entries_end = HEADER_SIZE
            .checked_add(
                count
                    .checked_mul(ENTRY_SIZE)
                    .context("NFH entry size overflow")?,
            )
            .context("NFH entry table overflow")?;
        ensure!(entries_end <= bytes.len(), "NFH entry table exceeds file");

        for (index, offset) in header.offsets.iter().enumerate() {
            ensure!(
                *offset >= 0 && *offset as usize <= count,
                "NFH offset {index} = {offset} outside entry table"
            );
        }

        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let offset = HEADER_SIZE + index * ENTRY_SIZE;
            entries.push(
                read_at::<NfhEntry>(bytes, offset, endian)
                    .with_context(|| format!("NFH entry {index}"))?,
            );
        }

        Ok(Self {
            data: bytes.to_vec(),
            endian,
            header,
            entries,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn entry(&self, code: u16) -> Option<&NfhEntry> {
        let low = usize::from(code as u8);
        let high = usize::from((code >> 8) as u8);
        let base = self.header.offsets[low];
        if base >= 0 {
            let index = base as usize + high;
            if let Some(entry) = self.entries.get(index) {
                if entry.code() == code {
                    return Some(entry);
                }
            }
        }
        self.entries.iter().find(|entry| entry.code() == code)
    }

    pub fn entry_for_char(&self, ch: char) -> Option<&NfhEntry> {
        let code = u32::from(ch);
        if code > u32::from(u16::MAX) {
            return None;
        }
        self.entry(code as u16)
    }

    pub fn probe(bytes: &[u8]) -> bool {
        bytes.starts_with(b"NFH\0")
    }
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
