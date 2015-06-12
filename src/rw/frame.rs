use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt, Stream};

use super::{Extension, Matrix};

// TODO

#[derive(Debug)]
pub struct FrameList;

impl FrameList {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<FrameList> {
		let _header = try!(Self::read_header(rws));

		let nframes = try!(Struct::read_up(rws, |rws| { Ok(try!(rws.read_u32::<LittleEndian>())) }));

		for _ in (0..nframes) {
			let _mat = Matrix::read(rws);
			let _parent =  try!(rws.read_u32::<LittleEndian>());
			try!(rws.read_u32::<LittleEndian>());	// matrix creation flags, ignored
		}

		for _ in (0..nframes) {
			try!(Extension::skip_section(rws));
		}

		Ok(FrameList)
	}
}

impl Section for FrameList {
	fn section_id() -> u32 { 0x000E }
}
