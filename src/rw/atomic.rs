use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{Extension};
use super::{FrameList, FrameRef, GeometryList, Geometry};

use std::rc::Rc;

bitflags! {
	flags AtomicFlags: u32 {
		const COLLISION_TEST = 0x1,
		const RENDER = 0x4,
	}
}

#[derive(Debug)]
pub struct Atomic {
	pub frame: Option<FrameRef>,
	pub geometry: Rc<Geometry>,
	pub flags: AtomicFlags,
}


impl Section for Atomic {
	fn section_id() -> u32 { 0x0014 }
}

impl Atomic {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>,
		                    framelist: &FrameList,
		                    geolist: &GeometryList)
	                            -> Result<Atomic> {

		let _header = try!(Self::read_header(rws));

		let (frame_index, geo_index, flags, _) = try!(Struct::read_up(rws, |rws| {
			Ok((try!(rws.read_u32::<LittleEndian>()),
				try!(rws.read_u32::<LittleEndian>()),
				try!(rws.read_u32::<LittleEndian>()),
				try!(rws.read_u32::<LittleEndian>())))
		}));

		let rcframe = framelist.frames.get(frame_index as usize).map(|rcframe| rcframe.clone());

		// read a child Geometry section if `geo_list` is empty.
		let rcgeo = match geolist.0 {
			ref geolist if geolist.is_empty() => Rc::new(try!(Geometry::read(rws))),
			ref geolist => try!(geolist.get(geo_index as usize).map(|rcgeo| rcgeo.clone())
					                   .ok_or(Error::Other("Invalid 'Atomic' Geometry id".into()))),
		};

		// extension
		try!(Extension::skip_section(rws));

		Ok(Atomic {
			frame: rcframe,
			geometry: rcgeo,
			flags: AtomicFlags::from_bits_truncate(flags),
		})
	}
}
