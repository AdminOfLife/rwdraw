#![allow(dead_code)]
use byteorder::{self, ReadBytesExt, LittleEndian};
use std::io::{self, Read, Seek, SeekFrom};

// TODO replace all the occ to ok_or to something more performancy because of string creation

pub mod prims;
pub use self::prims::{Rgba, Uv, Vec3, Sphere, Matrix};
pub mod section;
pub use self::section::{Struct, StringExt, Extension};
pub mod clump;
pub use self::clump::Clump;
pub mod frame;
pub use self::frame::FrameList;
pub mod atomic;
pub use self::atomic::Atomic;
pub mod geometry;
pub use self::geometry::{GeometryList, Geometry};
pub mod material;
pub use self::material::{MaterialList, Material, SurfaceProperties};



pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	//UnexpectedSection(u32),
	ExpectedSection { expect: u32, found: u32 },
	MissingSection(u32),
	IoError(byteorder::Error),
	Other(String),
}

impl From<byteorder::Error> for Error {
	fn from(e: byteorder::Error) -> Error {
		Error::IoError(e)
	}
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Error {
		Error::from(byteorder::Error::from(e))
	}
}




#[derive(Debug, Copy, Clone)]
pub struct Header {
	pub id: u32,
	pub size: u32,
	pub version: u32,
}

#[derive(Debug)]
pub struct SectionBuf {
	pub header: Header,
	pub data: Vec<u8>,
}

pub trait Section {
	fn section_id() -> u32;

	fn read_header<R: ReadExt>(rws: &mut R) -> Result<Header> {
		SectionBuf::read_header_id(rws, Self::section_id())
	}

	fn skip_section<R: ReadExt>(rws: &mut R) -> Result<u64> {
		SectionBuf::skip_section_id(rws, Self::section_id())
	}

	fn find_chunk<R: ReadExt>(rws: &mut R) -> Result<Header> {
		SectionBuf::find_chunk_id(rws, Self::section_id())
	}
}

impl SectionBuf {
	pub fn read<R: ReadExt>(rws: &mut R) -> Result<SectionBuf> {
		let header = try!(SectionBuf::read_header(rws));
		let size = header.size as usize;
		Ok(SectionBuf {
			header: header,
			data: try!(rws.read_bytes(size)),
		})
	}

	pub fn read_section_id<R: ReadExt>(rws: &mut R, id: u32) -> Result<SectionBuf> {
		let header = try!(SectionBuf::read_header_id(rws, id));
		let size = header.size as usize;
		Ok(SectionBuf {
			header: header,
			data: try!(rws.read_bytes(size)),
		})
	}

	fn read_header<R: ReadExt>(rws: &mut R) -> Result<Header> {
		Ok(Header {
			id: try!(rws.read_u32::<LittleEndian>()),
			size: try!(rws.read_u32::<LittleEndian>()),
			version: try!(rws.read_u32::<LittleEndian>()),
		})
	}

	fn read_header_id<R: ReadExt>(rws: &mut R, id: u32) -> Result<Header> {
		match SectionBuf::read_header(rws) {
			Ok(header) if header.id != id => {
				Err(Error::ExpectedSection { expect: id, found: header.id })
			},
			Ok(header) => Ok(header),
			Err(err) => Err(Error::from(err)),
		}
	}

	fn find_chunk_id<R: ReadExt>(rws: &mut R, id: u32) -> Result<Header> {
		loop {
			match SectionBuf::read_header(rws).map_err(|_| Error::MissingSection(id)) {
				Ok(header) if header.id == 0 => {	// end of stream virtually
					return Err(Error::MissingSection(id));
				},
				Ok(header) if header.id != id => {
					try!(rws.seek(SeekFrom::Current(header.size as i64)));
					// continue
				},
				Err(Error::IoError(byteorder::Error::UnexpectedEOF)) => {
					return Err(Error::MissingSection(id));
				},
				Ok(header) => return Ok(header),
				err @ Err(_) => return err,
			}
		}
	}

	fn skip_section<R: ReadExt>(rws: &mut R) -> Result<u64> {
		let header = try!(SectionBuf::read_header(rws));
		try!(rws.seek(SeekFrom::Current(header.size as i64)));
		Ok(0xC + header.size as u64)
	}

	fn skip_section_id<R: ReadExt>(rws: &mut R, id: u32) -> Result<u64> {
		let header = try!(SectionBuf::read_header_id(rws, id));
		try!(rws.seek(SeekFrom::Current(header.size as i64)));
		Ok(0xC + header.size as u64)
	}
}




pub trait ReadExt : Seek + byteorder::ReadBytesExt {
	fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>> {
		unsafe {
			let mut v = Vec::with_capacity(size);
			v.set_len(size);
			Ok(try!(self.read_full(&mut v[..]).map(|_| v)))
		}
	}

	fn read_full(&mut self, buf: &mut [u8]) -> byteorder::Result<()> {
	    use byteorder::*;
	    use std::io;
	    let mut nread = 0usize;
	    while nread < buf.len() {
	        match self.read(&mut buf[nread..]) {
	            Ok(0) => return Err(Error::UnexpectedEOF),
	            Ok(n) => nread += n,
	            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
	            Err(e) => return Err(From::from(e))
	        }
	    }
	    Ok(())
	}
}

impl<T: Seek + byteorder::ReadBytesExt> ReadExt for T {}
