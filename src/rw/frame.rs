use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt, Stream};

use super::{Extension, Matrix};
use rctree;

pub type FrameRef = rctree::NodeRef<FrameData>;

#[derive(Debug)]
pub struct FrameData {
	matrix: Matrix,
	name: String,
}

#[derive(Debug)]
pub struct FrameList {
	pub frames: Vec<FrameRef>,
}

#[derive(Debug)]
pub struct NodeNamePlg(String);

impl Section for FrameList {
	fn section_id() -> u32 { 0x000E }
}

impl Section for NodeNamePlg {
	fn section_id() -> u32 { 0x0253F2FE }
}

impl NodeNamePlg {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<NodeNamePlg> {
		let header = try!(Self::read_header(rws));
		let bytes = try!(rws.read_bytes(header.size as usize));
		Ok(NodeNamePlg(String::from_utf8_lossy(&bytes).into_owned()))
	}
}

impl FrameList {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<FrameList> {
		let _header = try!(Self::read_header(rws));

		let nframes = try!(Struct::read_up(rws, |rws| Ok(try!(rws.read_u32::<LittleEndian>()))));

		let mut frames: Vec<FrameRef> = Vec::with_capacity(nframes as usize);
		for _ in (0..nframes) {

			let frame = {
				let matrix = try!(Matrix::read(rws));
				let parent_id = try!(rws.read_i32::<LittleEndian>());
				let _flags = try!(rws.read_u32::<LittleEndian>());	// ignored

				let frame = FrameRef::new(FrameData {
					matrix: matrix,
					name: String::new(),
				});

				if parent_id >= 0 {
					if let Some(parent) = frames.get(parent_id as usize) {
						parent.append(frame.clone());
					}
				}

				frame
			};

			frames.push(frame);
		}

		for i in (0..nframes as usize) {
			match try!(Extension::read_for(rws, |rws| NodeNamePlg::read(rws))) {
				Some(NodeNamePlg(name)) => frames[i].borrow_mut().name = name,
				None => {},
			};
		}

		Ok(FrameList {
			frames: frames,
		})
	}
}
