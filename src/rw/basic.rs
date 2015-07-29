// TODO rename module to fundamentals or something alike
use byteorder::{ReadBytesExt, LittleEndian};
use super::{Result, ReadExt, Stream};

//#[cfg(feature="cgmath")]
use cgmath;

/// Represents a 2D point or vector.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Vec2(pub f32, pub f32);

/// Represents a 3D point or vector.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Vec3(pub f32, pub f32, pub f32);

/// Represents color and alpha components in four 8 bit values.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

/// Represents UV texture coordinates.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Uv(pub f32, pub f32);

/// Represents a 3D line.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Line {
    /// Line start.
    pub start: Vec3,
    /// Line end.
    pub end: Vec3,
}

/// Represents a 3D axis-aligned bounding-box.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct BBox {
    /// Supremum vertex (contains largest values).
    pub sup: Vec3,
    /// Infimum vertex (contains smallest values).
    pub inf: Vec3,
}

/// Represents a 2D device space rectangle.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Rect {
    /// X value of the top-left corner.
    pub x: u32,
    /// Y value of the top-left corner.
    pub y: u32,
    /// Width of the rectangle.
    pub w: u32,
    /// Height of the rectangle.
    pub h: u32,
}

/// Represents a sphere in 3D space.
#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

/// 3D space transformation matrix.
///
/// RenderWare uses 4x3, row-major affine matrices. 
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix {
    pub right: Vec3,
    pub top: Vec3,
    pub at: Vec3,
    pub pos: Vec3,
}

impl Rgba {
    /// Reads a `[u8; 4]` as a RGBA value off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Rgba> {
        Ok(Rgba(
            try!(rws.read_u8()),
            try!(rws.read_u8()),
            try!(rws.read_u8()),
            try!(rws.read_u8()),
        ))
    }
}

impl Uv {
    /// Reads a `[f32; 2]` as a UV value off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Uv> {
        Ok(Uv(
            try!(rws.read_f32::<LittleEndian>()),
            try!(rws.read_f32::<LittleEndian>()),
        ))
    }
}

impl Vec3 {
    /// Reads a `[f32; 3]` as a 3D vector value off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Vec3> {
        Ok(Vec3(
            try!(rws.read_f32::<LittleEndian>()),
            try!(rws.read_f32::<LittleEndian>()),
            try!(rws.read_f32::<LittleEndian>()),
        ))
    }
}

impl Sphere {
    /// Reads a `[f32; 4]` as a sphere coordinate and radius off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Sphere> {
        Ok(Sphere {
            center: try!(Vec3::read(rws)),
            radius: try!(rws.read_f32::<LittleEndian>()),
        })
    }
}

impl Matrix {
    /// Reads a `f32` 4x3 matrix off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Matrix> {
        Ok(Matrix {
            right: try!(Vec3::read(rws)),
            top: try!(Vec3::read(rws)),
            at: try!(Vec3::read(rws)),
            pos: try!(Vec3::read(rws)),
        })
    }
}

//#[cfg(feature="cgmath")]
impl From<Rgba> for cgmath::Vector4<f32> {
    /// This additionally converts the RGBA range from 0-255 to 0-1.
    fn from(rgba: Rgba) -> cgmath::Vector4<f32> {
        cgmath::Vector4::new(
            rgba.0 as f32 / 255.0,
            rgba.1 as f32 / 255.0,
            rgba.2 as f32 / 255.0,
            rgba.3 as f32 / 255.0,
        )
    }
}

//#[cfg(feature="cgmath")]
impl From<Uv> for cgmath::Vector2<f32> {
    fn from(uv: Uv) -> cgmath::Vector2<f32> {
        cgmath::Vector2::new(uv.0, uv.1)
    }
}

//#[cfg(feature="cgmath")]
impl From<Vec3> for cgmath::Point3<f32> {
    fn from(v: Vec3) -> cgmath::Point3<f32> {
        cgmath::Point3::new(v.0, v.1, v.2)
    }
}

//#[cfg(feature="cgmath")]
impl From<Vec3> for cgmath::Vector3<f32> {
    fn from(v: Vec3) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(v.0, v.1, v.2)
    }
}

//#[cfg(feature="cgmath")]
impl From<Matrix> for cgmath::Matrix4<f32> {
    fn from(m: Matrix) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::new(
            m.right.0,  m.right.1,  m.right.2,  0.0f32,
            m.top.0,    m.top.1,    m.top.2,    0.0f32,
            m.at.0,     m.at.1,     m.at.2,     0.0f32,
            m.pos.0,    m.pos.1,    m.pos.2,    1.0f32,
        )
    }
}

