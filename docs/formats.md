# Format support

## Implemented

| Format | Magic | Extension | Coverage |
|---|---|---|---|
| NUD 1.0 | `3PUN` / `NUP3` | `.nud` | header, object/primitive/chunk walk |
| NUD 2.0 | `NDLX` / `NDP3` | `.nud` | header, objects, primitives, vertex/index decode, materials |
| NUT | `NTLX` / `NTP3` | `.nut` | pictures, mip levels, cube faces |
| NSH | `NSLX` | `.nsh` | shader objects, bytecode chunks, parameters |
| NUS2 bank | version bytes | `.nub` | bank header, tone table, voice data, BNSF metadata |
| NFH | `NFH\0` | `.nfh` | header, names, glyph table, code lookup |

## Gaps

| Area | Missing |
|---|---|
| NUD 1.0 | mesh vertex/index extraction and material traversal |
| NUD 2.0 index | structured `GIDX` parsing; it is currently only unwrapped |
| NUT | CLUT/palette access and palette image decode |
| NUS2 | streaming sound containers and codec payloads |
| NSH | variants other than little-endian `NSLX` |
| NFH | glyph atlas pairing with the companion `.nut` is left to the caller |

`XFBIN`, `CPK`, Flash/LMD, and game-specific archive formats are outside this crate.
