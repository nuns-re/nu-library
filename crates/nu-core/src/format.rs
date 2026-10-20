use std::io::{Read, Seek, Write};

use crate::options::{LoadOptions, WriteOptions};
use crate::Result;

pub trait NuFormat: Sized {
    const NAME: &'static str;

    fn probe(bytes: &[u8]) -> bool;
    fn from_bytes(bytes: &[u8], options: &LoadOptions) -> Result<Self>;

    fn read<R: Read + Seek>(reader: &mut R, options: &LoadOptions) -> Result<Self> {
        let bytes = crate::read_all(reader, options.max_asset_bytes)?;
        Self::from_bytes(&bytes, options)
    }

    fn write<W: Write + Seek>(&self, _writer: &mut W, _options: &WriteOptions) -> Result<()> {
        Err(crate::NuError::Unsupported)
    }
}
