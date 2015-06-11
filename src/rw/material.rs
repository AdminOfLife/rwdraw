use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt};

use super::{SectionBuf, Extension, Rgba};
use super::Error;

use std::rc::Rc;

// STUB!!!!!!!!! MOVE TO texture.rs
#[derive(Debug)]
pub struct Texture;

#[derive(Debug, Copy, Clone)]
pub struct SurfaceProperties {
    ambient: f32,
    specular: f32,
    diffuse: f32,
}

#[derive(Debug)]
pub struct Material {
    texture: Option<Texture>,
    color: Rgba,
    surf: SurfaceProperties,
}

#[derive(Debug)]
pub struct MaterialList(pub Vec<Rc<Material>>);

impl Section for MaterialList {
    fn section_id() -> u32 { 0x0008 }
}

impl Section for Material {
    fn section_id() -> u32 { 0x0007 }
}

impl SurfaceProperties {
    pub fn read<R: ReadBytesExt>(rws: &mut R) -> Result<SurfaceProperties> {
        Ok(SurfaceProperties {
            ambient: try!(rws.read_f32::<LittleEndian>()),
            specular: try!(rws.read_f32::<LittleEndian>()),
            diffuse: try!(rws.read_f32::<LittleEndian>()),
        })
    }
}

impl MaterialList {
    pub fn read<R: ReadExt>(rws: &mut R) -> Result<MaterialList> {
        let _header = try!(Self::read_header(rws));

        let mats_id: Vec<i32> = try!(Struct::read_up(rws, |rws| {
            let matcount = try!(rws.read_u32::<LittleEndian>());
            (0..matcount).map(|_| Ok(try!(rws.read_i32::<LittleEndian>()))).collect()
        }));

        let mut mats: Vec<Rc<Material>> = Vec::with_capacity(mats_id.len());
        for &id in mats_id.iter() {
            let mat = match id {
                id if id < 0 => Rc::new(try!(Material::read(rws))),
                id => try!(mats.get(id as usize).map(|rcmat| rcmat.clone())
                               .ok_or(Error::Other("Invalid 'MaterialList' Material id".into()))),
            };
            mats.push(mat);
        }

        Ok(MaterialList(mats))
    }
}

impl Material {
    pub fn read<R: ReadExt>(rws: &mut R) -> Result<Material> {
        let _header = try!(Self::read_header(rws));

        let (_, color, _, has_tex, surf) = try!(Struct::read_up(rws, |rws| {
            Ok((try!(rws.read_u32::<LittleEndian>()),
                try!(Rgba::read(rws)),
                try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>()) != 0,
                try!(SurfaceProperties::read(rws))))
        }));

        // TODO texture
        if has_tex {
            try!(SectionBuf::skip_section_id(rws, 0x0006));
        }

        // extension
        try!(Extension::skip_section(rws));

        Ok(Material {
            texture: None,
            color: color,
            surf: surf,
        })
    }
}

