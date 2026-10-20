# nu

NU is a portable Rust middleware for NU-family graphics and audio assets.

It provides endian-aware format readers, backend-neutral render descriptions, and a small asset loading layer. The crate has no renderer or engine dependency.

## Formats

| Format | Status |
|---|---|
| NUD 1.0 / 2.0 models | read |
| NUT textures | read |
| NSLX shader banks | read |
| NUS2 sound banks | read |

Writers and streaming readers are the next milestones.

## Install

```toml
[dependencies]
nu = { path = "crates/nu" }
```

## Load an asset

```rust
use nu::prelude::*;

let source = FileSource::new("assets");
let mut assets = AssetManager::new(source);

match assets.load("model.nud")? {
    NuAsset::Model(model) => {
        // use model
    }
    NuAsset::Texture(texture) => {
        // use texture
    }
    _ => {}
}
```

## Use a custom registry

```rust
use nu::prelude::*;

let mut registry = Registry::new();
registry.register(
    "nud",
    model::Nud::probe,
    |bytes| model::Nud::parse(bytes).map(NuAsset::Model).map_err(Into::into),
);

let manager = AssetManager::new(MemorySource::new())
    .with_registry(registry)
    .with_options(LoadOptions::default());
```

## Renderer boundary

`nu` describes resources; the backend owns platform types:

```rust
use nu::prelude::*;

struct Backend;

impl RenderBackend for Backend {
    type Texture = ();
    type Mesh = ();
    type Shader = ();
    type Material = ();

    fn create_texture(&mut self, _desc: &TextureDesc, _data: TextureData<'_>) -> Result<Self::Texture> {
        Ok(())
    }

    fn create_mesh(&mut self, _desc: &MeshDesc, _data: &[u8]) -> Result<Self::Mesh> {
        Ok(())
    }

    fn create_shader(&mut self, _desc: &ShaderDesc, _data: &[u8]) -> Result<Self::Shader> {
        Ok(())
    }

    fn create_material(&mut self, _desc: &MaterialDesc) -> Result<Self::Material> {
        Ok(())
    }
}
```

## Design

- `nu-core`: errors, limits, endian, asset sources, format trait.
- `nu-formats`: NU model, texture, shader, and sound readers.
- `nu-render`: backend-neutral resource descriptions.
- `nu`: facade, registry, and asset manager.

The library does not implement windowing, archive mounting, gameplay, or a renderer. Those belong in adapters and applications.

## License

MIT.
