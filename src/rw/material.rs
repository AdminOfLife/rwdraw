use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{Texture, Extension, Rgba};
use std::rc::Rc;

/// Holds a list of `Material`s to be passed around.
#[derive(Debug)]
pub struct MaterialList(pub Vec<Rc<Material>>);

/// Materials describe how things are to appear when rendered.
///
/// This object defines how the surface of a particular triangle in a model will look and react to
/// lighting. Materials define color, texture, specularity, ambient reflectivity and diffusion.
///
/// Materials also provide a hook into the PowerPipe mechanism, so you can attach your own
/// rendering nodes on a per-material basis (not implemented).
#[derive(Debug)]
pub struct Material {
    pub texture: Option<Rc<Texture>>,
    pub color: Rgba,
    pub surf: SurfaceProperties,
}

/// Surface coefficients.
///
/// This type represents the ambient, diffuse and specular reflection coefficients of a particular
/// geometry. Each coefficient is specified in the range 0.0 (no reflection) to 1.0 (maximum reflection).
/// Note that currently the specular element is not used. 
#[derive(Debug, Copy, Clone)]
pub struct SurfaceProperties {
    /// Ambient reflection coefficient.
    pub ambient: f32,
    /// Reflection coefficient.
    pub specular: f32,
    /// Specular reflection coefficient.
    pub diffuse: f32,
}

impl Section for MaterialList {
    fn section_id() -> u32 { 0x0008 }
}

impl Section for Material {
    fn section_id() -> u32 { 0x0007 }
}

impl SurfaceProperties {
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<SurfaceProperties> {
        Ok(SurfaceProperties {
            ambient: try!(rws.read_f32::<LittleEndian>()),
            specular: try!(rws.read_f32::<LittleEndian>()),
            diffuse: try!(rws.read_f32::<LittleEndian>()),
        })
    }
}

impl MaterialList {
    /// Gets the material at the specified index or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<Rc<Material>> {
        self.0.get(index).map(|rcmat| rcmat.clone())
    }

    /// Reads a Material List off the RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<MaterialList> {
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
    /// Reads a Material off the RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Material> {
        let _header = try!(Self::read_header(rws));

        let (_, color, _, has_tex, surf) = try!(Struct::read_up(rws, |rws| {
            Ok((try!(rws.read_u32::<LittleEndian>()),       // unused flags
                try!(Rgba::read(rws)),
                try!(rws.read_u32::<LittleEndian>()),       // unused
                try!(rws.read_u32::<LittleEndian>()) != 0,
                try!(SurfaceProperties::read(rws))))
        }));

        // Associated texture...
        let texture = if has_tex {
            Some(try!(Texture::read(rws)))
        } else {
            None
        };

        // Extension...
        try!(Extension::skip_section(rws));

        Ok(Material {
            texture: texture,
            color: color,
            surf: surf,
        })
    }
}

