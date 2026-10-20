use binrw::Endian;

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub max_asset_bytes: usize,
    pub max_texture_bytes: usize,
    pub max_vertices: usize,
    pub max_mips: u32,
    pub strict: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            max_asset_bytes: 256 * 1024 * 1024,
            max_texture_bytes: 128 * 1024 * 1024,
            max_vertices: 4_000_000,
            max_mips: 16,
            strict: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    pub endian: Option<Endian>,
}
