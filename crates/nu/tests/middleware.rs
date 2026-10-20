use nu::prelude::*;

#[test]
fn registry_lists_builtin_formats() {
    let names = Registry::new().names().collect::<Vec<_>>();
    assert!(names.contains(&"nud"));
    assert!(names.contains(&"nut"));
    assert!(names.contains(&"nsh"));
    assert!(names.contains(&"nub"));
    assert!(names.contains(&"nfh"));
}

#[test]
fn registry_rejects_unknown_bytes() {
    assert!(matches!(
        Registry::new().parse(b"not-a-nu-asset"),
        Err(NuError::Unsupported)
    ));
}

#[test]
fn memory_source_uses_the_asset_manager() {
    let mut source = MemorySource::new();
    source.insert("empty.bin", Vec::new());
    let mut assets = AssetManager::new(source);
    assert!(matches!(
        assets.load("empty.bin"),
        Err(NuError::Unsupported)
    ));
    assert!(assets.load("missing.bin").is_err());
}

#[test]
fn file_source_rejects_absolute_paths() {
    let mut source = FileSource::new(".");
    assert!(source.open("/absolute").is_err());
}
