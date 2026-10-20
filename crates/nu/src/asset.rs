use nu_core::{read_all, AssetSource, LoadOptions, NuError, NuFormat, Result};
use nu_formats::{audio, model, texture, NuAsset};

type Probe = fn(&[u8]) -> bool;
type Reader = fn(&[u8]) -> Result<NuAsset>;

struct Entry {
    name: &'static str,
    probe: Probe,
    read: Reader,
}

#[derive(Default)]
pub struct Registry {
    entries: Vec<Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
            .with("nud", model::Nud::probe, |bytes| {
                model::Nud::parse(bytes)
                    .map(NuAsset::Model)
                    .map_err(Into::into)
            })
            .with("nut", texture::Nut::probe, |bytes| {
                texture::Nut::parse(bytes)
                    .map(NuAsset::Texture)
                    .map_err(Into::into)
            })
            .with(
                "nsh",
                |bytes| bytes.starts_with(b"NSLX"),
                |bytes| {
                    nu_formats::shader::Nsh::parse(bytes)
                        .map(NuAsset::Shader)
                        .map_err(Into::into)
                },
            )
            .with("nub", audio::Nus2Bank::probe, |bytes| {
                audio::Nus2Bank::parse(bytes)
                    .map(NuAsset::Audio)
                    .map_err(Into::into)
            })
    }

    pub fn with(mut self, name: &'static str, probe: Probe, read: Reader) -> Self {
        self.register(name, probe, read);
        self
    }

    pub fn register(&mut self, name: &'static str, probe: Probe, read: Reader) {
        self.entries.push(Entry { name, probe, read });
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.iter().map(|entry| entry.name)
    }

    pub fn parse(&self, bytes: &[u8]) -> Result<NuAsset> {
        let entry = self
            .entries
            .iter()
            .find(|entry| (entry.probe)(bytes))
            .ok_or(NuError::Unsupported)?;
        (entry.read)(bytes)
    }
}

pub struct AssetManager<S: AssetSource> {
    source: S,
    registry: Registry,
    options: LoadOptions,
}

impl<S: AssetSource> AssetManager<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            registry: Registry::new(),
            options: LoadOptions::default(),
        }
    }

    pub fn with_options(mut self, options: LoadOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_registry(mut self, registry: Registry) -> Self {
        self.registry = registry;
        self
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    pub fn load(&mut self, path: &str) -> Result<NuAsset> {
        let mut reader = self.source.open(path)?;
        let bytes = read_all(&mut reader, self.options.max_asset_bytes)?;
        self.registry.parse(&bytes)
    }

    pub fn load_bytes(&self, bytes: &[u8]) -> Result<NuAsset> {
        self.registry.parse(bytes)
    }
}
