use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt, Stream};

use super::{FrameList, Light, Atomic, GeometryList, Extension};


#[derive(Debug)]
pub struct Clump {
	atomics: Vec<Atomic>,
}

impl Clump {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Clump> {
		let _header = try!(Self::read_header(rws));

		let (natomics, nlights, _ncameras) = try!(Struct::read_up(rws, |rws| {
			Ok((try!(rws.read_u32::<LittleEndian>()),
			    try!(rws.read_u32::<LittleEndian>()),
			    try!(rws.read_u32::<LittleEndian>())))
		}));

		// framelist
		let framelist = try!(FrameList::read(rws));

		// geometry list
		let geolist = try!(GeometryList::read(rws));

		// atomic
		let mut atomics = Vec::with_capacity(natomics as usize);
		for _ in (0..natomics) {
			atomics.push(try!(Atomic::read(rws, &framelist, &geolist)));
		}

		// TODO
		for _ in (0..nlights) {
			try!(Struct::skip_section(rws));
			try!(Light::skip_section(rws));
		}

		// camera
		// TODO

		// extension!
		try!(Extension::skip_section(rws));

		Ok(Clump {
			atomics: atomics,
		})
	}

	pub fn into_atomic(mut self) -> Option<Atomic> {
		self.atomics.pop()
	}
}

impl Section for Clump {
	fn section_id() -> u32 { 0x0010 }
}
