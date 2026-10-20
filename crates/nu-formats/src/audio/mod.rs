//! NUS2 sound bank reader.

use anyhow::{bail, ensure, Context, Result};
use binrw::{BinRead, Endian};

use crate::util::{bytes_at, magic, read_at};

/// Bank header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nus2BankHeader {
    pub endian_marker: i8,
    pub major_version: u8,
    pub minor_version: u8,
    pub target: i8,
    pub form: i32,
    pub bank_id: i32,
    pub tone_count: i32,
    pub header_size: i32,
    pub body_size: u32,
    pub table_pos: i32,
    pub voice_pos: i32,
}

impl Nus2BankHeader {
    pub fn endian(&self) -> Result<Endian> {
        match self.endian_marker {
            0 => Ok(Endian::Big),
            1 => Ok(Endian::Little),
            other => bail!("unknown NUS2 endian marker {other}"),
        }
    }

    pub fn end(&self) -> Result<usize> {
        let header = usize::try_from(self.header_size).context("negative NUS2 header size")?;
        let body = self.body_size as usize;
        header
            .checked_add(body)
            .context("NUS2 header + body size overflow")
    }
}

/// Voice attributes.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nus2VoiceAttr {
    pub ext_string: [u8; 4],
    pub bank_id: i32,
    pub tone_no: i32,
    pub file_format: i32,
    pub rand_flag: i32,
    pub wave_size: u32,
    pub bank_offset: u32,
    pub fmt_chunk_size: i32,
    pub loop_start: u32,
    pub loop_length: u32,
    pub attr_flag: i32,
    pub link_num: i32,
}

impl Nus2VoiceAttr {
    pub fn ext_string(&self) -> String {
        String::from_utf8_lossy(&self.ext_string).into_owned()
    }
}

/// Voice parameters.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nus2VoiceDataSheet {
    pub link_no: i32,
    pub volume: f32,
    pub lfe_volume: f32,
    pub pitch: i32,
    pub pan: f32,
    pub stereo_wide: f32,
    pub speaker_distance: f32,
    pub delay_time: i32,
    pub center_balance: f32,
    pub type_3d: i32,
    pub low_pass_filter: f32,
    pub high_pass_filter: f32,
    pub out_bus: i32,
    pub voice_group: i32,
    pub fx_send_bus: i32,
    pub fx_send_volume: f32,
    pub priority: i32,
    pub probability: i32,
    pub link_flag: i32,
    pub flag: i32,
    pub attack_rate: i32,
    pub decay_rate: i32,
    pub sustain_rate: i32,
    pub sustain_level: f32,
    pub release_rate: i32,
    pub offset_volume: f32,
    pub random_pitch: i32,
    pub random_volume: f32,
    pub random_pan: f32,
    pub random_speaker_distance: f32,
    pub adjust_pitch: f32,
    pub user1: i32,
    pub user2: i32,
    pub user3: i32,
    pub user4: i32,
}

/// Voice info.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nus2VoiceInfo {
    pub attr: Nus2VoiceAttr,
    pub data_sheet: Nus2VoiceDataSheet,
}

/// BNSF format info.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct BnsfSfmtInfo {
    pub version: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub num_samples: u32,
    pub offset_samples: u32,
    pub block_align: u16,
    pub samples_per_block: u16,
}

/// BNSF stream header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct BnsfHeader {
    pub magic: [u8; 4],
    pub bnsf_size: u32,
    pub codec: [u8; 4],
    pub sfmt_tag: [u8; 4],
    pub sfmt_size: u32,
    pub sfmt: BnsfSfmtInfo,
    pub sdat_tag: [u8; 4],
    pub sdat_size: u32,
}

impl BnsfHeader {
    pub fn codec(&self) -> String {
        String::from_utf8_lossy(&self.codec).into_owned()
    }

    pub fn is_is14(&self) -> bool {
        &self.codec == b"IS14" || &self.codec == b"is14"
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(&self.magic == b"BNSF", "missing BNSF magic");
        ensure!(&self.sfmt_tag == b"sfmt", "missing sfmt tag");
        ensure!(&self.sdat_tag == b"sdat", "missing sdat tag");
        Ok(())
    }
}

/// One tone record.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nus2ToneRecord {
    pub voice_info: Nus2VoiceInfo,
    pub bnsf: BnsfHeader,
    pub tail: [u8; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nus2Tone {
    pub offset: usize,
    pub record: Nus2ToneRecord,
}

impl Nus2Tone {
    pub fn audio<'a>(&self, bank: &'a Nus2Bank) -> Result<&'a [u8]> {
        let start = (bank.header.header_size as usize)
            .checked_add(self.record.voice_info.attr.bank_offset as usize)
            .context("NUS2 tone audio offset overflow")?;
        bytes_at(
            bank.data(),
            start,
            self.record.voice_info.attr.wave_size as usize,
        )
        .with_context(|| {
            format!(
                "NUS2 tone {} audio at {start:#x}+{:#x}",
                self.record.voice_info.attr.tone_no, self.record.voice_info.attr.wave_size
            )
        })
    }

    pub fn sfmt(&self) -> &BnsfSfmtInfo {
        &self.record.bnsf.sfmt
    }
}

/// NUS2 bank.
#[derive(Debug, Clone)]
pub struct Nus2Bank {
    data: Vec<u8>,
    pub endian: Endian,
    pub header: Nus2BankHeader,
    pub tone_offsets: Vec<u32>,
    pub tones: Vec<Nus2Tone>,
}

impl Nus2Bank {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 0x20 {
            bail!("NUS2 bank is shorter than its header");
        }
        // Byte 0 is the endian marker; fall back to structural validation if
        // it is not a known value.
        let endian = match bytes[0] {
            0 => Endian::Big,
            1 => Endian::Little,
            _ => {
                if Nus2Bank::parse_with_endian(bytes, Endian::Little).is_ok() {
                    Endian::Little
                } else {
                    Endian::Big
                }
            }
        };
        Self::parse_with_endian(bytes, endian)
    }

    pub fn parse_with_endian(bytes: &[u8], endian: Endian) -> Result<Self> {
        let header: Nus2BankHeader = read_at(bytes, 0, endian)?;
        ensure!(
            header.major_version == 2,
            "unsupported NUS2 major version {}",
            header.major_version
        );
        ensure!(
            header.minor_version == 1,
            "unsupported NUS2 minor version {}",
            header.minor_version
        );
        ensure!(header.tone_count >= 0, "negative NUS2 tone count");
        ensure!(
            header.tone_count <= 65536,
            "implausible NUS2 tone count {}",
            header.tone_count
        );
        let expected_end = header.end()?;
        ensure!(
            expected_end == bytes.len(),
            "NUS2 header_size + body_size = {:#x}, file is {:#x}",
            expected_end,
            bytes.len()
        );
        ensure!(
            header.table_pos >= 0x20,
            "invalid NUS2 tone table offset {}",
            header.table_pos
        );

        let table_pos = header.table_pos as usize;
        let tone_count = header.tone_count as usize;
        let mut tone_offsets = Vec::with_capacity(tone_count);
        for i in 0..tone_count {
            let offset = read_at::<u32>(bytes, table_pos + i * 4, endian)
                .with_context(|| format!("NUS2 tone offset {i}"))?;
            ensure!(
                offset as usize >= 0x20 && offset as usize + 0xf0 <= bytes.len(),
                "NUS2 tone offset {i} {offset:#x} outside buffer"
            );
            tone_offsets.push(offset);
        }

        let mut tones = Vec::with_capacity(tone_count);
        for (i, offset) in tone_offsets.iter().enumerate() {
            let record: Nus2ToneRecord = read_at(bytes, *offset as usize, endian)
                .with_context(|| format!("NUS2 tone {i} at {offset:#x}"))?;
            record.bnsf.validate()?;
            tones.push(Nus2Tone {
                offset: *offset as usize,
                record,
            });
        }

        Ok(Self {
            data: bytes.to_vec(),
            endian,
            header,
            tone_offsets,
            tones,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn tone(&self, index: usize) -> Option<&Nus2Tone> {
        self.tones.get(index)
    }

    /// Check the NUS2 signature.
    pub fn probe(bytes: &[u8]) -> bool {
        if bytes.len() < 8 {
            return false;
        }
        let magic = match magic(bytes) {
            Ok(magic) => magic,
            Err(_) => return false,
        };
        // The bank has no ASCII tag; use the version bytes at +1/+2.
        magic[1] == 2 && magic[2] == 1 && (magic[0] == 0 || magic[0] == 1)
    }
}
