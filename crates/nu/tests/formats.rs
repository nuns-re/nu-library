use nu::audio::Nus2Bank;
use nu::font::Nfh;
use nu::model::{decode_r11g11b10, Nud, Nud20};
use nu::shader::Nsh;
use nu::texture::Nut;
use nu::Endian;

struct W {
    data: Vec<u8>,
    endian: Endian,
}

impl W {
    fn new(endian: Endian) -> Self {
        Self {
            data: Vec::new(),
            endian,
        }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    fn u8(&mut self, value: u8) {
        self.data.push(value);
    }

    fn u16(&mut self, value: u16) {
        match self.endian {
            Endian::Big => self.data.extend_from_slice(&value.to_be_bytes()),
            Endian::Little => self.data.extend_from_slice(&value.to_le_bytes()),
        }
    }

    fn u32(&mut self, value: u32) {
        match self.endian {
            Endian::Big => self.data.extend_from_slice(&value.to_be_bytes()),
            Endian::Little => self.data.extend_from_slice(&value.to_le_bytes()),
        }
    }

    fn i32(&mut self, value: i32) {
        self.u32(value as u32);
    }

    fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    fn zeros(&mut self, count: usize) {
        self.data.resize(self.data.len() + count, 0);
    }

    fn pad_to(&mut self, offset: usize) {
        if self.data.len() < offset {
            self.zeros(offset - self.data.len());
        }
    }
}

fn nud20_bytes(endian: Endian, magic: &[u8; 4]) -> Vec<u8> {
    // One object, one primitive, one triangle. Vertex format 0 (P32), UV
    // format 0x10 (one f16 UV set, no color).
    let object_section = 0x60usize;
    let index_size = 6usize;
    let static_size = 3 * (0x10 + 4);
    let file_size = 0x30 + object_section + index_size + static_size;
    let mut w = W::new(endian);
    w.bytes(magic);
    w.u32(file_size as u32);
    w.u8(2);
    w.u8(0);
    w.u16(1);
    w.u16(0);
    w.u16(0);
    w.u32(object_section as u32);
    w.u32(index_size as u32);
    w.u32(static_size as u32);
    w.u32(0);
    w.zeros(16); // bounding sphere

    // Object at 0x30.
    w.zeros(16); // bounding sphere
    w.f32(0.0);
    w.f32(0.0);
    w.f32(0.0);
    w.f32(0.0);
    w.u32(0); // name
    w.u16(0); // reserved
    w.u16(0); // attributes
    w.u16(0); // matrix index
    w.u16(1); // primitive count
    w.u32(0x60); // primitive address

    // Primitive at 0x60.
    w.u32(0); // index offset
    w.u32(0); // static vertex offset
    w.u32(0); // dynamic vertex offset
    w.u16(3); // vertex count
    w.u8(0); // vertex format
    w.u8(0x10); // uv format: 1 set, f16 UV
    w.u32(0);
    w.u32(0);
    w.u32(0);
    w.u32(0);
    w.u16(3); // index count
    w.u16(0); // attributes
    w.u32(0); // linked index offset
    w.u32(0); // transformed size
    w.u32(0); // transformed vertex offset

    // Index section at 0x90.
    w.u16(0);
    w.u16(1);
    w.u16(2);

    // Static vertices at 0x96.
    for i in 0..3 {
        w.f32(i as f32);
        w.f32(0.0);
        w.f32(0.0);
        w.f32(0.0);
        w.u16(0);
        w.u16(0);
    }
    assert_eq!(w.data.len(), file_size);
    w.data
}

fn nud10_bytes(endian: Endian, magic: &[u8; 4]) -> Vec<u8> {
    let mut w = W::new(endian);
    w.bytes(magic);
    w.u32(0x80); // file size
    w.u16(0x100); // version
    w.u16(1); // object count
    w.zeros(0x14);
    // Object at 0x20.
    w.zeros(8);
    w.u32(1); // primitive count
    w.zeros(0x14);
    // Primitive at 0x40, file/original layout.
    w.zeros(0x14);
    w.u32(1); // chunk count
    w.zeros(8);
    // Chunk at 0x60.
    w.zeros(0x20);
    w.pad_to(0x80);
    w.data
}

fn nut_bytes(endian: Endian, magic: &[u8; 4]) -> Vec<u8> {
    let mut w = W::new(endian);
    w.bytes(magic);
    w.u8(2); // format version
    w.u8(0); // format id
    w.u16(1); // pictures
    w.zeros(8);
    // Picture at 0x10, total 0x40.
    w.u32(0x40); // total size
    w.u32(0); // clut size
    w.u32(8); // image size
    w.u16(0x30); // header size
    w.u16(0); // clut colors
    w.u8(0); // pict format
    w.u8(2); // mipmap textures
    w.u8(0); // clut type
    w.u8(0); // image type BC1
    w.u16(2); // width
    w.u16(2); // height
    w.u32(0); // caps1
    w.u32(0); // caps2
    w.u32(0x38); // image offset
    w.zeros(12);
    // Mip sizes at +0x30.
    w.u32(4);
    w.u32(4);
    // Image at +0x38.
    w.bytes(&[0xAA; 4]);
    w.bytes(&[0xBB; 4]);
    assert_eq!(w.data.len(), 0x50);
    w.data
}

fn cube_nut_bytes() -> Vec<u8> {
    let mut w = W::new(Endian::Little);
    w.bytes(b"NTLX");
    w.u8(2);
    w.u8(0);
    w.u16(1);
    w.zeros(8);
    w.u32(0x4c); // total size
    w.u32(0);
    w.u32(24); // image size
    w.u16(0x30);
    w.u16(0);
    w.u8(0);
    w.u8(1); // one mip
    w.u8(0);
    w.u8(0); // BC1
    w.u16(4);
    w.u16(4);
    w.u32(0);
    w.u32(0x200); // cube map
    w.u32(0x34); // image offset, after cube size
    w.zeros(12);
    w.u32(24); // cube map image size at +0x30
    for i in 0..6 {
        w.bytes(&[i as u8; 4]);
    }
    assert_eq!(w.data.len(), 0x5c);
    w.data
}

fn nsh_bytes() -> Vec<u8> {
    let mut w = W::new(Endian::Little);
    w.bytes(b"NSLX");
    w.u8(1);
    w.u8(0);
    w.u16(0);
    w.u32(0);
    w.u16(1); // object count
    w.zeros(0x12);
    // Object at 0x20, total 0x100.
    w.u32(0x0000_F00A); // shader index
    w.u32(0); // attribute
    w.u32(0x20); // vertex shader chunk
    w.u32(0x2c); // pixel shader chunk
    w.u32(0x38); // vertex parameter chunk
    w.u32(0x44); // pixel parameter chunk
    w.u32(0x100); // total size
    w.u32(0);
    // Chunks at +0x40, +0x4c, +0x58, +0x64.
    w.u32(0x50); // vertex data offset
    w.u32(1);
    w.u32(4);
    w.u32(0x54); // pixel data offset
    w.u32(1);
    w.u32(4);
    w.u32(0x58); // vertex parameter array offset
    w.u32(1);
    w.u32(0x20);
    w.u32(0x78); // pixel parameter array offset
    w.u32(1);
    w.u32(0x20);
    // Vertex/pixel shader bytes at +0x70/+0x74.
    w.bytes(&[1, 2, 3, 4]);
    w.bytes(&[5, 6, 7, 8]);
    // Vertex parameter at +0x58 (absolute 0x78).
    w.u32(0x98); // name
    w.u32(0x50); // value
    w.u32(0xa9); // type
    w.u32(0);
    w.u32(0);
    w.u32(1); // number of parameter
    w.u32(0);
    w.u32(0);
    // Pixel parameter at +0x78 (absolute 0x98).
    w.u32(0x98);
    w.u32(0x54);
    w.u32(0xa9);
    w.u32(0);
    w.u32(0);
    w.u32(1);
    w.u32(0);
    w.u32(0);
    // Strings at +0x98 (relative to object).
    w.pad_to(0x20 + 0x98);
    w.bytes(b"MATRIX_WORLDVIEW\0");
    w.bytes(b"MATRIX\0");
    w.pad_to(0x20 + 0x100);
    w.data
}

fn nfh_bytes() -> Vec<u8> {
    let mut w = W::new(Endian::Little);
    w.bytes(b"NFH\0");
    w.i32(20);
    w.i32(2);
    w.zeros(4);
    let mut family = [0u8; 32];
    family[..4].copy_from_slice(b"Test");
    w.bytes(&family);
    let mut style = [0u8; 32];
    style[..6].copy_from_slice(b"Normal");
    w.bytes(&style);
    for index in 0..256 {
        w.i32(match index {
            0x41 => 0,
            0x42 => 1,
            _ => 0,
        });
    }
    w.i32(10);
    w.u16(0);
    w.u16(0);
    w.zeros(12);
    w.u8(8);
    w.u8(8);
    w.u8(0x41);
    w.u8(0);
    w.u8(0);
    w.u8(1);
    w.zeros(6);
    w.i32(11);
    w.u16(0);
    w.u16(0);
    w.zeros(12);
    w.u8(8);
    w.u8(8);
    w.u8(0x42);
    w.u8(0);
    w.u8(0);
    w.u8(0);
    w.zeros(6);
    w.data
}

fn nub_bytes(endian: Endian, marker: i8) -> Vec<u8> {
    let mut w = W::new(endian);
    w.u8(marker as u8);
    w.u8(2);
    w.u8(1);
    w.u8(0);
    w.i32(0); // form
    w.i32(7); // bank id
    w.i32(1); // tone count
    w.i32(0x120); // header size (tone record ends at 0x114)
    w.u32(0x20); // body size
    w.i32(0x20); // table pos
    w.i32(0x24); // voice pos
                 // Table at 0x20.
    w.u32(0x24);
    // Tone at 0x24.
    w.bytes(b"IS14");
    w.i32(7);
    w.i32(0);
    w.i32(1);
    w.i32(0);
    w.u32(0x20); // wave size
    w.u32(0); // bank offset
    w.i32(0x14); // fmt chunk size
    w.u32(0); // loop start
    w.u32(0); // loop length
    w.i32(0); // attr flag
    w.i32(-1); // link num
               // Voice data sheet (0x8c = 35 u32/f32 fields).
    w.i32(-1); // link no
    w.f32(1.0); // volume
    for _ in 0..33 {
        w.u32(0);
    }
    // BNSF header.
    w.bytes(b"BNSF");
    w.u32(0x30);
    w.bytes(b"IS14");
    w.bytes(b"sfmt");
    w.u32(0x14);
    w.u16(0);
    w.u16(2);
    w.u32(48_000);
    w.u32(100);
    w.u32(0);
    w.u16(0x10);
    w.u16(0x10);
    w.bytes(b"sdat");
    w.u32(0x20);
    w.zeros(4); // tail
    w.pad_to(0x120);
    w.bytes(&[0x5A; 0x20]);
    w.data
}

#[test]
fn nud20_little_and_big_endian() {
    for (endian, magic) in [(Endian::Little, b"NDLX"), (Endian::Big, b"NDP3")] {
        let bytes = nud20_bytes(endian, magic);
        let nud = Nud20::parse(&bytes).unwrap();
        assert_eq!(nud.endian, endian);
        assert_eq!(nud.objects.len(), 1);
        let primitive = &nud.objects[0].primitives[0];
        let mesh = primitive.decode(&nud).unwrap();
        assert_eq!(
            mesh.positions,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]
        );
        assert_eq!(mesh.uv_sets.len(), 1);
        assert_eq!(mesh.triangles, vec![[1, 2, 0]]);
    }
}

#[test]
fn nud_dispatch_handles_both_versions() {
    let v2 = nud20_bytes(Endian::Little, b"NDLX");
    assert!(matches!(Nud::parse(&v2).unwrap(), Nud::V2(_)));
    let v1 = nud10_bytes(Endian::Little, b"3PUN");
    let nud = Nud::parse(&v1).unwrap();
    let Nud::V1(nud) = nud else {
        panic!("expected NUD10")
    };
    assert_eq!(nud.objects.len(), 1);
    assert_eq!(nud.objects[0].primitives.len(), 1);
    assert_eq!(nud.objects[0].primitives[0].chunks.len(), 1);
}

#[test]
fn packed_r11g11b10_decodes_one() {
    let r = 15u32 << 5;
    let g = 15u32 << 6;
    let b = 15u32 << 6;
    let bits = r | (g << 10) | (b << 21);
    let decoded = decode_r11g11b10(bits);
    assert!((decoded[0] - 1.0).abs() < 1e-4);
    assert!((decoded[1] - 1.0).abs() < 1e-4);
    assert!((decoded[2] - 1.0).abs() < 1e-4);
}

#[test]
fn nut_levels_and_cube_faces() {
    for (endian, magic) in [(Endian::Little, b"NTLX"), (Endian::Big, b"NTP3")] {
        let bytes = nut_bytes(endian, magic);
        let nut = Nut::parse(&bytes).unwrap();
        assert_eq!(nut.endian, endian);
        let picture = &nut.pictures[0];
        assert_eq!(picture.header.width, 2);
        assert_eq!(picture.header.height, 2);
        assert_eq!(picture.mipmap_sizes, vec![4, 4]);
        let levels = nut.levels(picture).unwrap();
        assert_eq!(levels, vec![&[0xAA; 4][..], &[0xBB; 4][..]]);
    }

    let bytes = cube_nut_bytes();
    let nut = Nut::parse(&bytes).unwrap();
    let picture = &nut.pictures[0];
    assert!(picture.header.is_cube_map());
    assert_eq!(picture.cube_map_image_size, Some(24));
    let faces = nut.faces(picture).unwrap();
    assert_eq!(faces.len(), 6);
    for (i, face) in faces.iter().enumerate() {
        assert_eq!(*face, [i as u8; 4]);
    }
}

#[test]
fn nsh_objects_chunks_and_parameters() {
    let bytes = nsh_bytes();
    let nsh = Nsh::parse(&bytes).unwrap();
    assert_eq!(nsh.objects.len(), 1);
    let object = &nsh.objects[0];
    assert_eq!(object.record.shader_index, 0x0000_F00A);
    assert_eq!(
        object.vertex_shader_bytes(nsh.data()).unwrap(),
        Some(&[1, 2, 3, 4][..])
    );
    assert_eq!(
        object.pixel_shader_bytes(nsh.data()).unwrap(),
        Some(&[5, 6, 7, 8][..])
    );
    assert_eq!(object.vertex_parameters.len(), 1);
    assert_eq!(object.vertex_parameters[0].name, "MATRIX_WORLDVIEW");
    assert_eq!(object.vertex_parameters[0].type_name, "MATRIX");
    assert_eq!(object.vertex_parameters[0].number_of_parameter, 1);
}

#[test]
fn nfh_header_entries_and_lookup() {
    let bytes = nfh_bytes();
    assert_eq!(nu::detect(&bytes).map(|(name, _)| name), Some("NFH"));
    let nfh = Nfh::parse(&bytes).unwrap();
    assert_eq!(nfh.header.font_height, 20);
    assert_eq!(nfh.header.family(), "Test");
    assert_eq!(nfh.header.style(), "Normal");
    assert_eq!(nfh.entries.len(), 2);
    assert_eq!(nfh.entry_for_char('A').unwrap().glyph_id, 10);
    assert_eq!(nfh.entry_for_char('B').unwrap().glyph_id, 11);
    assert!(nfh.entry_for_char('C').is_none());
}

#[test]
fn nus2_bank_parses_tone_and_audio() {
    for (endian, marker) in [(Endian::Little, 1), (Endian::Big, 0)] {
        let bytes = nub_bytes(endian, marker);
        let bank = Nus2Bank::parse(&bytes).unwrap();
        assert_eq!(bank.endian, endian);
        assert_eq!(bank.header.bank_id, 7);
        assert_eq!(bank.header.tone_count, 1);
        assert_eq!(bank.tones.len(), 1);
        let tone = &bank.tones[0];
        assert_eq!(tone.record.voice_info.attr.ext_string(), "IS14");
        assert_eq!(tone.record.voice_info.attr.wave_size, 0x20);
        assert_eq!(tone.record.bnsf.codec(), "IS14");
        assert_eq!(tone.sfmt().channels, 2);
        assert_eq!(tone.sfmt().sample_rate, 48_000);
        assert_eq!(tone.audio(&bank).unwrap(), &[0x5A; 0x20]);
    }
}
