use byteorder::{ReadBytesExt, LittleEndian};
use super::{Result};

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
	pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<Rgba> {
		Ok(Rgba(
			try!(rws.read_u8()),
			try!(rws.read_u8()),
			try!(rws.read_u8()),
			try!(rws.read_u8()),
		))
	}
}

impl Uv {
	pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<Uv> {
		Ok(Uv(
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
		))
	}
}

impl Vec3 {
	pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<Vec3> {
		Ok(Vec3(
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
			try!(rws.read_f32::<LittleEndian>()),
		))
	}
}

impl Sphere {
	pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<Sphere> {
		Ok(Sphere {
			center: try!(Vec3::read(rws)),
			radius: try!(rws.read_f32::<LittleEndian>()),
		})
	}
}

impl Matrix {
	pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<Matrix> {
		Ok(Matrix {
			right: try!(Vec3::read(rws)),
			top: try!(Vec3::read(rws)),
			at: try!(Vec3::read(rws)),
			pos: try!(Vec3::read(rws)),
		})
	}
}
