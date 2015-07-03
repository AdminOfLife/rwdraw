#![allow(dead_code)]
#![feature(alloc)]
use byteorder::{self, ReadBytesExt, LittleEndian};
use std::io::{self, Read, Seek, SeekFrom};
use std::rc::Rc;

// TODO replace all the occ to ok_or to something more performancy because of string creation
// TODO fix version detection on streams

mod basic;
mod section;
mod clump;
mod frame;
mod atomic;
mod geometry;
mod material;
mod texture;
mod light;

pub use self::basic::{Rgba, Uv, Vec3, Sphere, Matrix, BBox, Rect, Line};
pub use self::section::{Struct, StringExt, Extension};
pub use self::clump::Clump;
pub use self::frame::{FrameList, Frame, FrameObjectValue, FrameObject, NodeNamePlg};
pub use self::atomic::Atomic;
pub use self::geometry::{GeometryList, Geometry};
pub use self::material::{MaterialList, Material, SurfaceProperties};
pub use self::texture::{Texture, TexDictionary, TexNative, FilterMode, WrapMode, TextureData, TexLevel};
pub use self::light::Light;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    //UnexpectedSection(u32),
    ExpectedSection { expect: u32, found: u32 },
    MissingSection(u32),
    IoError(byteorder::Error),
    Other(String), // TODO Find all calls to this (ok_or) and optimize to be lazy to avoid alloc
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

    fn read_header<R: ReadExt>(rws: &mut Stream<R>) -> Result<Header> {
        SectionBuf::read_header_id(rws, Self::section_id())
    }

    fn skip_section<R: ReadExt>(rws: &mut Stream<R>) -> Result<u64> {
        SectionBuf::skip_section_id(rws, Self::section_id())
    }

    fn find_chunk<R: ReadExt>(rws: &mut Stream<R>) -> Result<Header> {
        SectionBuf::find_chunk_id(rws, Self::section_id())
    }
}

impl SectionBuf {
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<SectionBuf> {
        let header = try!(SectionBuf::read_header(rws));
        let size = header.size as usize;
        Ok(SectionBuf {
            header: header,
            data: try!(rws.read_bytes(size)),
        })
    }

    pub fn read_section_id<R: ReadExt>(rws: &mut Stream<R>, id: u32) -> Result<SectionBuf> {
        let header = try!(SectionBuf::read_header_id(rws, id));
        let size = header.size as usize;
        Ok(SectionBuf {
            header: header,
            data: try!(rws.read_bytes(size)),
        })
    }

    fn read_header<R: ReadExt>(rws: &mut Stream<R>) -> Result<Header> {
        Ok(Header {
            id: try!(rws.read_u32::<LittleEndian>()),
            size: try!(rws.read_u32::<LittleEndian>()),
            version: try!(rws.read_u32::<LittleEndian>()),
        })
    }

    fn read_header_id<R: ReadExt>(rws: &mut Stream<R>, id: u32) -> Result<Header> {
        match SectionBuf::read_header(rws) {
            Ok(header) if header.id != id => {
                Err(Error::ExpectedSection { expect: id, found: header.id })
            },
            Ok(header) => Ok(header),
            Err(err) => Err(Error::from(err)),
        }
    }

    fn find_chunk_id<R: ReadExt>(rws: &mut Stream<R>, id: u32) -> Result<Header> {
        loop {
            match SectionBuf::read_header(rws).map_err(|_| Error::MissingSection(id)) {
                Ok(header) if header.id == 0 => {   // end of stream virtually
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

    fn skip_section<R: ReadExt>(rws: &mut Stream<R>) -> Result<u64> {
        let header = try!(SectionBuf::read_header(rws));
        try!(rws.seek(SeekFrom::Current(header.size as i64)));
        Ok(0xC + header.size as u64)
    }

    fn skip_section_id<R: ReadExt>(rws: &mut Stream<R>, id: u32) -> Result<u64> {
        let header = try!(SectionBuf::read_header_id(rws, id));
        try!(rws.seek(SeekFrom::Current(header.size as i64)));
        Ok(0xC + header.size as u64)
    }
}


pub struct Instance {
    curr_dict: Option<Rc<TexDictionary>>,   // binded dictionary
}

impl Instance {
    pub fn new() -> Instance {
        Instance {
            curr_dict: None,
        }
    }

    pub fn bind_dictionary(&mut self, dictionary: &Rc<TexDictionary>) {
        self.curr_dict = Some(dictionary.clone());
    }

    pub fn unbind_dictionary(&mut self) {
        self.curr_dict = None;
    }

    pub fn read_texture(&self, name: &str, mask: Option<&str>) -> Option<Rc<Texture>> {
        match self.curr_dict {
            Some(ref dict) => dict.read_texture(name, mask),
            None => None,
        }
    }
}

pub struct Stream<'a, R> where R: ReadExt {
    inner: R,
    rw: &'a mut Instance,
}

impl<'a, R: ReadExt> Stream<'a, R> {
    pub fn new(inner: R, rw: &'a mut Instance) -> Stream<'a, R> {
        Stream {
            inner: inner,
            rw: rw,
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<'a, R: ReadExt> io::Read for Stream<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<'a, R: ReadExt + io::BufRead> io::BufRead for Stream<'a, R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}
impl<'a, R: ReadExt> io::Seek for Stream<'a, R> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
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
