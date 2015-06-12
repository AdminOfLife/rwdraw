use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{StringExt};
use super::{Extension, Vec3, Uv, Sphere, Rgba};
use super::{Material, MaterialList};

use std::rc::Rc;

#[derive(Debug)]
pub struct Texture;

impl Texture {
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Rc<Texture>> {
        let (filter_flags, _) = try!(Struct::read_up(rws, |rws| {
            Ok((try!(rws.read_u16::<LittleEndian>()), try!(rws.read_u16::<LittleEndian>())))
        }));

        let name = try!(String::read(rws));
        let mask = try!(String::read(rws));

        try!(Extension::skip_section(rws));

        rws.rw.read_texture(&name, Some(&mask))
              .ok_or(Error::Other("TODO NO TEX".into()))
    }
}
