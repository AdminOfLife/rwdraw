use byteorder::{ReadBytesExt, LittleEndian};
use super::{Result, ReadExt, Stream};

//#[cfg(feature="cgmath")]
use cgmath;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Uv(pub f32, pub f32);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Vec3(pub f32, pub f32, pub f32);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Sphere {
	pub center: Vec3,
	pub radius: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix {
	pub right: Vec3,
	pub top: Vec3,
	pub at: Vec3,
	pub pos: Vec3,
}

impl Rgba {
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
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Uv> {
		Ok(Uv(
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
		))
	}
}

impl Vec3 {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Vec3> {
		Ok(Vec3(
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
		))
	}
}

impl Sphere {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Sphere> {
		Ok(Sphere {
			center: try!(Vec3::read(rws)),
			radius: try!(rws.read_f32::<LittleEndian>()),
		})
	}
}

impl Matrix {
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
