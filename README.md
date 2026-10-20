# nulib

nulib is a portable Rust middleware for NU Library graphics and audio assets.

It provides endian aware format readers, backend neutral render descriptions, and a small asset loading layer. 

## Formats

| Format | Status |
|---|---|
| NUD 1.0 / 2.0 models | read |
| NUT textures | read |
| NSLX shader banks | read |
| NUS2 sound banks | read |
| NFH bitmap fonts | read |

See [docs/formats.md](docs/formats.md) for coverage and remaining gaps.



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
