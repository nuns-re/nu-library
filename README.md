# nu-library

nu-library is a portable Rust middleware for NU Library graphics and audio assets.



## Formats

| Format | Status |
|---|---|
| [NUD 1.0 / 2.0 models](crates/nu-formats/src/model/mod.rs) | read |
| [NUT textures](crates/nu-formats/src/texture.rs) | read |
| [NSLX shader banks](crates/nu-formats/src/shader.rs) | read |
| [NUS2 sound banks](crates/nu-formats/src/audio/mod.rs) | read |
| [NFH bitmap fonts](crates/nu-formats/src/font.rs) | read |



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



## License

MIT.
