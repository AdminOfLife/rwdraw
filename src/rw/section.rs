use super::{Result, Error, ReadExt, Header, Section, SectionBuf};

use std::io::{Seek, SeekFrom};
use std::any::Any;

#[derive(Debug)]
pub struct Struct;

#[derive(Debug)]
pub struct Extension;

impl Section for Struct {
    fn section_id() -> u32 { 0x0001 }
}

impl Section for String {
    fn section_id() -> u32 { 0x0002 }
}

impl Section for Extension {
    fn section_id() -> u32 { 0x0003 }
}

impl Struct {
    pub fn read_up<R: ReadExt, T, F>(rws: &mut R, f: F) -> Result<T>
                                    where F: FnOnce(&mut R) -> Result<T> {
        let _header = try!(Self::read_header(rws));
        f(rws)
        // TODO check if f() readed too much
    }
}

impl Extension {
    pub fn read_up<R: ReadExt, F>(rws: &mut R, f: F) -> Result<Vec<Box<Any>>>
                                    where F: Fn(&mut R, Header) -> Result<Option<Box<Any>>> {
        let header = try!(Self::read_header(rws));
        if header.size > 0 {
            let endoff = (header.size as u64) + try!(rws.seek(SeekFrom::Current(0)));
            let mut vec = Vec::new(); // XXX maybe with capacity if we add too many plugins
            while endoff > try!(rws.seek(SeekFrom::Current(0))) {
                let plg_header = try!(SectionBuf::read_header(rws));
                try!(rws.seek(SeekFrom::Current(-12)));
                // TODO check if f() readed too much
                match try!(f(rws, plg_header)) {
                    Some(plg) => vec.push(plg),
                    None => { try!(SectionBuf::skip_section(rws)); },
                }
            }
            Ok(vec)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn read_for<R: ReadExt, T: Any + Section, F>(rws: &mut R, f: F) -> Result<Option<T>>
                                        where F: Fn(&mut R) -> Result<T> {
        let boxes = try!(Extension::read_up(rws, |rws, header| {
            if header.id == T::section_id() {
                f(rws).map(|val| Some(Box::new(val) as Box<Any>))
            } else {
                Ok(None)
            }
        }));
        Ok(boxes.into_iter()
                .find(|bx_any| bx_any.is::<T>())
                .map(|bx_any| bx_any.downcast().unwrap())
                .map(|bx_tyy| *bx_tyy))
    }
}

pub trait StringExt : Section {
    fn read<R: ReadExt>(rws: &mut R) -> Result<Self>;
}

impl StringExt for String {
    fn read<R: ReadExt>(rws: &mut R) -> Result<Self> {
        let header = try!(SectionBuf::read_header_id(rws, Self::section_id()));
        rws.read_bytes(header.size as usize).and_then(|mut vec| {
            match vec.iter().find(|&&c| c == 0) {
                Some(&endpos) => vec.truncate(endpos as usize),
                None => {},
            };
            String::from_utf8(vec).map_err(|_| Error::Other("RwString is not valid UTF-8".into()))
        })
    }
}
