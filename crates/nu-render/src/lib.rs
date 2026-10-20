//! Backend-neutral render descriptions.

use nu_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8,
    Rgba8,
    Bgra8,
    Rgba16Float,
    Bc1,
    Bc3,
    Bc7,
    Astc4x4,
    Astc8x8,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub format: TextureFormat,
    pub cube: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureLevel<'a> {
    pub width: u32,
    pub height: u32,
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct TextureData<'a> {
    pub levels: Vec<TextureLevel<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    TriangleList,
    TriangleStrip,
    Lines,
    Points,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshDesc {
    pub vertex_count: u32,
    pub index_count: u32,
    pub vertex_stride: u32,
    pub topology: PrimitiveTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Pixel,
    Compute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderFormat {
    Nvn,
    Dxbc,
    SpirV,
    Msl,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderDesc {
    pub stage: ShaderStage,
    pub format: ShaderFormat,
}

#[derive(Debug, Clone)]
pub struct MaterialDesc {
    pub name: String,
    pub shader: String,
    pub textures: Vec<String>,
}

pub trait RenderBackend {
    type Texture;
    type Mesh;
    type Shader;
    type Material;

    fn create_texture(
        &mut self,
        desc: &TextureDesc,
        data: TextureData<'_>,
    ) -> Result<Self::Texture>;
    fn create_mesh(&mut self, desc: &MeshDesc, data: &[u8]) -> Result<Self::Mesh>;
    fn create_shader(&mut self, desc: &ShaderDesc, data: &[u8]) -> Result<Self::Shader>;
    fn create_material(&mut self, desc: &MaterialDesc) -> Result<Self::Material>;
}
