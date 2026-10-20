use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub trait AssetSource {
    fn open(&mut self, path: &str) -> crate::Result<Box<dyn ReadSeek>>;
}

#[derive(Debug, Clone)]
pub struct FileSource {
    root: std::path::PathBuf,
}

impl FileSource {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl AssetSource for FileSource {
    fn open(&mut self, path: &str) -> crate::Result<Box<dyn ReadSeek>> {
        let path = std::path::Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(crate::NuError::Invalid("asset path"));
        }
        let file = std::fs::File::open(self.root.join(path))?;
        Ok(Box::new(file))
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemorySource {
    files: HashMap<String, Vec<u8>>,
}

impl MemorySource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(path.into(), bytes.into());
    }
}

impl AssetSource for MemorySource {
    fn open(&mut self, path: &str) -> crate::Result<Box<dyn ReadSeek>> {
        let bytes = self
            .files
            .get(path)
            .cloned()
            .ok_or(crate::NuError::Invalid("asset path"))?;
        Ok(Box::new(Cursor::new(bytes)))
    }
}

pub fn read_all<R: Read + ?Sized>(reader: &mut R, max: usize) -> crate::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(max.saturating_add(1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(crate::NuError::Limit("asset size"));
    }
    Ok(bytes)
}
