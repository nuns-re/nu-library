//! NUD 2.0 model reader.

use anyhow::{bail, ensure, Context, Result};
use binrw::{BinRead, Endian};
use half::f16;

use crate::model::vertex::{Nud20VertexFormat, NudSkinSize, VertexElement, VertexEncoding};
use crate::util::{bytes_at, cstring_at, magic, read_at};

/// NUD20 header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20Header {
    pub file_id: [u8; 4],
    pub file_size: u32,
    pub major_version: u8,
    pub minor_version: u8,
    pub number_of_objects: u16,
    pub min_matrix_index: u16,
    pub max_matrix_index: u16,
    pub object_section_size: u32,
    pub index_section_size: u32,
    pub static_vertex_section_size: u32,
    pub dynamic_vertex_section_size: u32,
    pub bounding_sphere: [f32; 4],
}

impl Nud20Header {
    pub fn file_id_string(&self) -> String {
        String::from_utf8_lossy(&self.file_id).into_owned()
    }
}

/// Section bases derived from the header sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nud20Sections {
    pub object_base: usize,
    pub index_base: usize,
    pub static_vertex_base: usize,
    pub dynamic_vertex_base: usize,
    pub names_base: usize,
    pub end: usize,
}

impl Nud20Sections {
    fn from_header(header: &Nud20Header) -> Result<Self> {
        let object_base = 0x30usize;
        let index_base = object_base
            .checked_add(header.object_section_size as usize)
            .context("NUD object section overflow")?;
        let static_vertex_base = index_base
            .checked_add(header.index_section_size as usize)
            .context("NUD index section overflow")?;
        let dynamic_vertex_base = static_vertex_base
            .checked_add(header.static_vertex_section_size as usize)
            .context("NUD static vertex section overflow")?;
        let names_base = dynamic_vertex_base
            .checked_add(header.dynamic_vertex_section_size as usize)
            .context("NUD dynamic vertex section overflow")?;
        Ok(Self {
            object_base,
            index_base,
            static_vertex_base,
            dynamic_vertex_base,
            names_base,
            end: names_base,
        })
    }
}

/// NUD20 object.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20ObjectRecord {
    pub bounding_sphere: [f32; 4],
    pub sort_center: [f32; 3],
    pub sort_bias: f32,
    pub name_address: u32,
    pub reserved: u16,
    pub attributes: u16,
    pub matrix_index: u16,
    pub number_of_primitives: u16,
    pub primitive_address: u32,
}

/// A parsed object / mesh group.
#[derive(Debug, Clone, PartialEq)]
pub struct Nud20Object {
    pub offset: usize,
    pub name: Option<String>,
    pub record: Nud20ObjectRecord,
    pub primitives: Vec<Nud20Primitive>,
}

/// NUD20 primitive.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20PrimitiveRecord {
    pub index_offset: u32,
    pub static_vertex_offset: u32,
    pub dynamic_vertex_offset: u32,
    pub number_of_vertices: u16,
    pub vertex_type_vector: u8,
    pub vertex_type_texcoord: u8,
    pub material_address: [u32; 4],
    pub number_of_indices: u16,
    pub attributes: u16,
    pub linked_index_offset: u32,
    pub transformed_size: u32,
    pub transformed_vertex_offset: u32,
}

/// A parsed primitive / mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct Nud20Primitive {
    pub offset: usize,
    pub record: Nud20PrimitiveRecord,
    pub materials: Vec<Nud20Material>,
}

impl Nud20Primitive {
    pub fn is_skinned(&self) -> bool {
        self.record.vertex_type_vector >> 4 != 0
    }

    pub fn draw_slot(&self) -> u8 {
        (self.record.attributes & 3) as u8
    }

    pub fn decode(&self, nud: &Nud20) -> Result<MeshData> {
        nud.decode_primitive(self)
    }
}

/// NUD20 material header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20MaterialHeader {
    pub shader_index: u32,
    pub p_shader: u32,
    pub attributes: u16,
    pub number_of_textures: u16,
    pub alpha_blend: u16,
    pub alpha_test_func: u16,
    pub alpha_test_ref: u16,
    pub cull_face: u16,
    pub reserved: u32,
    pub offset_scale: f32,
    pub offset_bias: f32,
}

impl Nud20MaterialHeader {
    pub fn blend_enabled(&self) -> bool {
        self.attributes & 1 != 0
    }

    pub fn blend_rows(&self) -> (u8, u8) {
        (self.alpha_blend as u8, (self.alpha_blend >> 8) as u8)
    }
}

/// One 0x18-byte texture descriptor after a material header.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20MaterialTexture {
    pub texture_source: i32,
    pub reserved04: u32,
    pub reserved08: u16,
    pub map_mode: u16,
    pub wrap_s: u8,
    pub wrap_t: u8,
    pub min_filter: u8,
    pub mag_filter: u8,
    pub mip_detail: u8,
    pub unknown11: u8,
    pub reserved12: u32,
    pub unknown16: u16,
}

impl Nud20MaterialTexture {
    pub fn lod(&self) -> u16 {
        self.reserved12 as u16
    }
}

/// Variable-length material property record.
#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Nud20MaterialPropertyRecord {
    pub next_offset: u32,
    pub name_offset: u32,
    pub reserved08: [u8; 3],
    pub value_count: u8,
    pub reserved0c: u32,
    #[br(count = value_count)]
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nud20MaterialProperty {
    pub name: String,
    pub values: Vec<f32>,
}

/// A material pass: header + texture descriptors + property linked list.
#[derive(Debug, Clone, PartialEq)]
pub struct Nud20Material {
    pub offset: usize,
    pub header: Nud20MaterialHeader,
    pub textures: Vec<Nud20MaterialTexture>,
    pub properties: Vec<Nud20MaterialProperty>,
}

/// A fully parsed NUD 2.0 file.
#[derive(Debug, Clone)]
pub struct Nud20 {
    data: Vec<u8>,
    pub endian: Endian,
    pub header: Nud20Header,
    pub sections: Nud20Sections,
    pub objects: Vec<Nud20Object>,
}

impl Nud20 {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = magic(bytes)?;
        let endian = match &magic {
            b"NDLX" => Endian::Little,
            b"NDP3" => Endian::Big,
            other => bail!(
                "unexpected NUD20 magic {:?}",
                String::from_utf8_lossy(other)
            ),
        };
        let header: Nud20Header = read_at(bytes, 0, endian)?;
        ensure!(
            header.major_version == 2,
            "unsupported NUD major version {}",
            header.major_version
        );
        let sections = Nud20Sections::from_header(&header)?;
        ensure!(
            sections.end <= bytes.len(),
            "NUD sections end at {:#x}, file is {:#x}",
            sections.end,
            bytes.len()
        );
        ensure!(
            header.number_of_objects <= 4096,
            "implausible NUD object count {}",
            header.number_of_objects
        );

        let mut objects = Vec::with_capacity(header.number_of_objects as usize);
        for i in 0..header.number_of_objects as usize {
            let offset = sections.object_base + i * 0x30;
            let record: Nud20ObjectRecord = read_at(bytes, offset, endian)
                .with_context(|| format!("NUD object {i} at {offset:#x}"))?;
            let name = if record.name_address == 0 {
                None
            } else {
                Some(cstring_at(bytes, record.name_address as usize)?)
            };
            let primitives = parse_primitives(bytes, &record, endian, sections.names_base)?;
            objects.push(Nud20Object {
                offset,
                name,
                record,
                primitives,
            });
        }

        Ok(Self {
            data: bytes.to_vec(),
            endian,
            header,
            sections,
            objects,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn decode_primitive(&self, primitive: &Nud20Primitive) -> Result<MeshData> {
        let record = &primitive.record;
        let format =
            Nud20VertexFormat::new(record.vertex_type_vector, record.vertex_type_texcoord)?;
        let layout = format.layout();
        let skinned = format.is_skinned();
        let main_base = if skinned {
            self.sections.dynamic_vertex_base + record.dynamic_vertex_offset as usize
        } else {
            self.sections.static_vertex_base + record.static_vertex_offset as usize
        };
        let uv_base = self.sections.static_vertex_base + record.static_vertex_offset as usize;
        let main_stride = layout.stride + if skinned { format.skin_size.size() } else { 0 };
        let uv_stride = format.uv.stride();
        let vertex_stride = if skinned {
            main_stride
        } else {
            layout.stride + uv_stride
        };
        let vertex_count = record.number_of_vertices as usize;
        ensure!(
            vertex_count <= 1_000_000,
            "implausible NUD vertex count {vertex_count}"
        );

        let mut positions = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut binormals = Vec::with_capacity(vertex_count);
        let mut tangents = Vec::with_capacity(vertex_count);
        let mut uv_sets: Vec<Vec<[f32; 2]>> = (0..format.uv.sets())
            .map(|_| Vec::with_capacity(vertex_count))
            .collect();
        let mut colors = Vec::with_capacity(vertex_count);
        let mut joints = Vec::with_capacity(vertex_count);
        let mut weights = Vec::with_capacity(vertex_count);

        for i in 0..vertex_count {
            let vertex_base = main_base + i * vertex_stride;
            positions.push(read_element(
                &self.data,
                vertex_base,
                layout.position,
                self.endian,
            )?);
            if let Some(element) = layout.normal {
                normals.push(read_element(&self.data, vertex_base, element, self.endian)?);
            } else {
                normals.push([0.0, 0.0, 1.0]);
            }
            if let Some(element) = layout.binormal {
                binormals.push(read_element(&self.data, vertex_base, element, self.endian)?);
            }
            if let Some(element) = layout.tangent {
                tangents.push(read_element(&self.data, vertex_base, element, self.endian)?);
            }
            if skinned {
                let (j, w) = read_skin(
                    &self.data,
                    vertex_base + layout.stride,
                    format.skin_size,
                    self.endian,
                )?;
                joints.push(j);
                weights.push(w);
            }

            let uv_vertex_base = if skinned {
                uv_base + i * uv_stride
            } else {
                main_base + i * (layout.stride + uv_stride) + layout.stride
            };
            if format.uv.color_size() != crate::model::vertex::NudColorSize::None {
                colors.push(read_color(
                    &self.data,
                    uv_vertex_base + format.uv.color_offset(),
                    format.uv.color_size(),
                    self.endian,
                )?);
            }
            for (set, uv_set) in uv_sets.iter_mut().enumerate().take(format.uv.sets()) {
                let offset = uv_vertex_base + format.uv.uv_offset(set);
                uv_set.push(read_uv(
                    &self.data,
                    offset,
                    format.uv.uv_float(),
                    self.endian,
                )?);
            }
        }

        let index_base = self.sections.index_base + record.index_offset as usize;
        let index_count = record.number_of_indices as usize;
        let mut indices = Vec::with_capacity(index_count);
        for i in 0..index_count {
            indices.push(read_at::<u16>(&self.data, index_base + i * 2, self.endian)?);
        }
        let triangles = expand_strips(&indices, vertex_count)?;

        Ok(MeshData {
            positions,
            normals,
            binormals: (!binormals.is_empty()).then_some(binormals),
            tangents: (!tangents.is_empty()).then_some(tangents),
            uv_sets,
            colors: (!colors.is_empty()).then_some(colors),
            joints: (!joints.is_empty()).then_some(joints),
            weights: (!weights.is_empty()).then_some(weights),
            indices,
            triangles,
        })
    }
}

fn parse_primitives(
    data: &[u8],
    object: &Nud20ObjectRecord,
    endian: Endian,
    names_base: usize,
) -> Result<Vec<Nud20Primitive>> {
    let count = object.number_of_primitives as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    ensure!(
        object.primitive_address != 0,
        "NUD object has {count} primitives but a zero primitive address"
    );
    let base = object.primitive_address as usize;
    ensure!(
        base.checked_add(count * 0x30)
            .is_some_and(|end| end <= data.len()),
        "NUD primitive table {base:#x}..{:#x} outside buffer",
        base + count * 0x30
    );
    let mut primitives = Vec::with_capacity(count);
    for i in 0..count {
        let offset = base + i * 0x30;
        let record: Nud20PrimitiveRecord = read_at(data, offset, endian)
            .with_context(|| format!("NUD primitive {i} at {offset:#x}"))?;
        let mut materials = Vec::new();
        for &material_offset in &record.material_address {
            if material_offset != 0 {
                materials.push(parse_material(
                    data,
                    material_offset as usize,
                    endian,
                    names_base,
                )?);
            }
        }
        primitives.push(Nud20Primitive {
            offset,
            record,
            materials,
        });
    }
    Ok(primitives)
}

fn parse_material(
    data: &[u8],
    offset: usize,
    endian: Endian,
    names_base: usize,
) -> Result<Nud20Material> {
    let header: Nud20MaterialHeader = read_at(data, offset, endian)
        .with_context(|| format!("NUD material header at {offset:#x}"))?;
    ensure!(
        header.number_of_textures <= 64,
        "implausible NUD texture count {}",
        header.number_of_textures
    );
    let texture_base = offset
        .checked_add(0x20)
        .context("NUD material header overflow")?;
    let mut textures = Vec::with_capacity(header.number_of_textures as usize);
    for i in 0..header.number_of_textures as usize {
        let texture_offset = texture_base + i * 0x18;
        textures.push(
            read_at::<Nud20MaterialTexture>(data, texture_offset, endian)
                .with_context(|| format!("NUD texture {i} at {texture_offset:#x}"))?,
        );
    }

    let mut properties = Vec::new();
    let mut cursor = texture_base + header.number_of_textures as usize * 0x18;
    for _ in 0..1024 {
        let record: Nud20MaterialPropertyRecord = read_at(data, cursor, endian)
            .with_context(|| format!("NUD material property at {cursor:#x}"))?;
        if record.name_offset != 0 {
            let name = cstring_at(data, names_base + record.name_offset as usize)?;
            properties.push(Nud20MaterialProperty {
                name,
                values: record.values.clone(),
            });
        }
        if record.next_offset == 0 {
            break;
        }
        cursor = cursor
            .checked_add(record.next_offset as usize)
            .context("NUD material property offset overflow")?;
        ensure!(cursor < data.len(), "NUD material property outside buffer");
    }

    Ok(Nud20Material {
        offset,
        header,
        textures,
        properties,
    })
}

fn read_element(
    data: &[u8],
    base: usize,
    element: VertexElement,
    endian: Endian,
) -> Result<[f32; 3]> {
    let offset = base + element.offset;
    Ok(match element.encoding {
        VertexEncoding::F32 => [
            read_at::<f32>(data, offset, endian)?,
            read_at::<f32>(data, offset + 4, endian)?,
            read_at::<f32>(data, offset + 8, endian)?,
        ],
        VertexEncoding::F16 => [
            f16::from_bits(read_at::<u16>(data, offset, endian)?).to_f32(),
            f16::from_bits(read_at::<u16>(data, offset + 2, endian)?).to_f32(),
            f16::from_bits(read_at::<u16>(data, offset + 4, endian)?).to_f32(),
        ],
        VertexEncoding::R11G11B10 => decode_r11g11b10(read_at::<u32>(data, offset, endian)?),
    })
}

fn read_color(
    data: &[u8],
    base: usize,
    size: crate::model::vertex::NudColorSize,
    endian: Endian,
) -> Result<[f32; 4]> {
    Ok(match size {
        crate::model::vertex::NudColorSize::None => [1.0; 4],
        crate::model::vertex::NudColorSize::U8 => {
            let bytes = bytes_at(data, base, 4)?;
            [
                bytes[0] as f32 / 255.0,
                bytes[1] as f32 / 255.0,
                bytes[2] as f32 / 255.0,
                bytes[3] as f32 / 255.0,
            ]
        }
        crate::model::vertex::NudColorSize::U16 => [
            read_at::<u16>(data, base, endian)? as f32 / 65535.0,
            read_at::<u16>(data, base + 2, endian)? as f32 / 65535.0,
            read_at::<u16>(data, base + 4, endian)? as f32 / 65535.0,
            read_at::<u16>(data, base + 6, endian)? as f32 / 65535.0,
        ],
    })
}

fn read_uv(data: &[u8], base: usize, float: bool, endian: Endian) -> Result<[f32; 2]> {
    if float {
        Ok([
            read_at::<f32>(data, base, endian)?,
            read_at::<f32>(data, base + 4, endian)?,
        ])
    } else {
        Ok([
            f16::from_bits(read_at::<u16>(data, base, endian)?).to_f32(),
            f16::from_bits(read_at::<u16>(data, base + 2, endian)?).to_f32(),
        ])
    }
}

fn read_skin(
    data: &[u8],
    base: usize,
    skin: NudSkinSize,
    endian: Endian,
) -> Result<([u16; 4], [f32; 4])> {
    match skin {
        NudSkinSize::None => bail!("called read_skin on an unskinned primitive"),
        NudSkinSize::Float => Ok((
            [
                read_at::<u32>(data, base, endian)? as u16,
                read_at::<u32>(data, base + 4, endian)? as u16,
                read_at::<u32>(data, base + 8, endian)? as u16,
                read_at::<u32>(data, base + 12, endian)? as u16,
            ],
            [
                read_at::<f32>(data, base + 16, endian)?,
                read_at::<f32>(data, base + 20, endian)?,
                read_at::<f32>(data, base + 24, endian)?,
                read_at::<f32>(data, base + 28, endian)?,
            ],
        )),
        NudSkinSize::Half => Ok((
            [
                read_at::<u16>(data, base, endian)?,
                read_at::<u16>(data, base + 2, endian)?,
                read_at::<u16>(data, base + 4, endian)?,
                read_at::<u16>(data, base + 6, endian)?,
            ],
            [
                f16::from_bits(read_at::<u16>(data, base + 8, endian)?).to_f32(),
                f16::from_bits(read_at::<u16>(data, base + 10, endian)?).to_f32(),
                f16::from_bits(read_at::<u16>(data, base + 12, endian)?).to_f32(),
                f16::from_bits(read_at::<u16>(data, base + 14, endian)?).to_f32(),
            ],
        )),
        NudSkinSize::Byte => {
            let bytes = bytes_at(data, base, 8)?;
            Ok((
                [
                    bytes[0] as u16,
                    bytes[1] as u16,
                    bytes[2] as u16,
                    bytes[3] as u16,
                ],
                [
                    bytes[4] as f32 / 255.0,
                    bytes[5] as f32 / 255.0,
                    bytes[6] as f32 / 255.0,
                    bytes[7] as f32 / 255.0,
                ],
            ))
        }
    }
}

/// Decode `GL_UNSIGNED_INT_10F_11F_11F_REV` (R=10 bits, G/B=11 bits).
pub fn decode_r11g11b10(bits: u32) -> [f32; 3] {
    fn unsigned_float(bits: u32, mantissa_bits: u32) -> f32 {
        let mantissa_mask = (1u32 << mantissa_bits) - 1;
        let mantissa = bits & mantissa_mask;
        let exponent = (bits >> mantissa_bits) & 0x1f;
        if exponent == 0 {
            // Subnormal: 2^-14 * mantissa / 2^mantissa_bits
            (mantissa as f32 / mantissa_mask as f32) * 2.0f32.powi(-14)
        } else if exponent == 0x1f {
            f32::INFINITY
        } else {
            (1.0 + mantissa as f32 / mantissa_mask as f32) * 2.0f32.powi(exponent as i32 - 15)
        }
    }
    [
        unsigned_float(bits & 0x3ff, 5),
        unsigned_float((bits >> 10) & 0x7ff, 6),
        unsigned_float((bits >> 21) & 0x7ff, 6),
    ]
}

/// Expand native triangle strips (0xffff restarts) into triangle lists.
pub fn expand_strips(raw: &[u16], vertex_count: usize) -> Result<Vec<[u32; 3]>> {
    let mut result = Vec::new();
    for strip in raw.split(|&index| index == 0xffff) {
        for (i, triangle) in strip.windows(3).enumerate() {
            let (a, b, c) = if i % 2 == 0 {
                (triangle[1], triangle[2], triangle[0])
            } else {
                (triangle[2], triangle[1], triangle[0])
            };
            ensure!(
                (a as usize) < vertex_count
                    && (b as usize) < vertex_count
                    && (c as usize) < vertex_count,
                "triangle index outside {vertex_count}-vertex buffer"
            );
            if a != b && a != c && b != c {
                result.push([a as u32, b as u32, c as u32]);
            }
        }
    }
    Ok(result)
}

/// Decoded primitive ready for a renderer adapter.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub binormals: Option<Vec<[f32; 3]>>,
    pub tangents: Option<Vec<[f32; 3]>>,
    pub uv_sets: Vec<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub joints: Option<Vec<[u16; 4]>>,
    pub weights: Option<Vec<[f32; 4]>>,
    pub indices: Vec<u16>,
    pub triangles: Vec<[u32; 3]>,
}
