use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{Extension, StringExt};

use std::rc::Rc;
use std::collections::HashMap;

// TODO support III/VC and PS2 texture dictionaries

#[derive(Debug)]
pub enum FilterMode {
    None,
    Nearest,
    Linear,
    MipNearest,
    MipLinear,
    LinearMipNearest,
    LinearMipLinear,
}

#[derive(Debug)]
pub enum WrapMode {
    None,
    Repeat,
    Mirror,
    Clamp,
    Border,
}

#[derive(Debug)]
enum RasterFormat {
    /// 1 bit alpha, RGB 5 bits each; also used for DXT1 with alpha.
    R5G5B5A1,
    /// 5 bits red, 6 bits green, 5 bits blue; also used for DXT1 without alpha
    R5G6B5,
    /// RGBA 4 bits each; also used for DXT3
    R4G4B4A4,
    /// gray scale
    A8L8,
    /// RGBA 8 bits each
    R8G8B8A8,
    /// RGB 8 bits each
    R8G8B8,
    /// RGB 5 bits each
    R5G5B5,
}

impl FilterMode {
    fn from_raw(raw: u32) -> Option<FilterMode> {
        match raw {
            0 => Some(FilterMode::None),
            1 => Some(FilterMode::Nearest),
            2 => Some(FilterMode::Linear),
            3 => Some(FilterMode::MipNearest),
            4 => Some(FilterMode::MipLinear),
            5 => Some(FilterMode::LinearMipNearest),
            6 => Some(FilterMode::LinearMipLinear),
            _ => None,
        }
    }
}

impl WrapMode {
    fn from_raw(raw: u32) -> Option<WrapMode> {
        match raw {
            0 => Some(WrapMode::None),
            1 => Some(WrapMode::Repeat),
            2 => Some(WrapMode::Mirror),
            3 => Some(WrapMode::Clamp),
            4 => Some(WrapMode::Border),
            _ => None,
        }
    }
}

impl RasterFormat {
    fn from_raw(raw: u32) -> Option<RasterFormat> {
        match raw & 0x0F00 {
            0x0100 => Some(RasterFormat::R5G5B5A1),
            0x0200 => Some(RasterFormat::R5G6B5),
            0x0300 => Some(RasterFormat::R4G4B4A4),
            0x0400 => Some(RasterFormat::A8L8),
            0x0500 => Some(RasterFormat::R8G8B8A8),
            0x0600 => Some(RasterFormat::R8G8B8),
            0x0A00 => Some(RasterFormat::R5G5B5),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TextureData {
    Dxt1a(Vec<u8>),
    Dxt1c(Vec<u8>),
    Dxt3(Vec<u8>),
    Dxt5(Vec<u8>),
}

#[derive(Debug)]
pub struct Texture {
    pub dict: Rc<String>,
    pub name: Rc<String>,
    pub mask: Rc<String>,
    pub data: TextureData,
    pub width: u16,
    pub height: u16,
    pub filter: FilterMode,
    pub wrap_x: WrapMode,
    pub wrap_y: WrapMode,
}

#[derive(Debug)]
pub struct TexDictionary {
    textures: HashMap<String, Rc<Texture>>,
}

#[derive(Debug)]
pub struct TexNative;

impl Section for Texture {
    fn section_id() -> u32 { 0x0006 }
}

impl Section for TexDictionary {
    fn section_id() -> u32 { 0x0016 }
}

impl Section for TexNative {
    fn section_id() -> u32 { 0x0015 }
}

impl Texture {
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Rc<Texture>> {
        let _header = try!(Self::read_header(rws));

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

impl TexDictionary {

    pub fn read_texture(&self, name: &str, mask: Option<&str>) -> Option<Rc<Texture>> {
        self.textures.get(name).map(|rctex| rctex.clone())
    }

    pub fn read<R: ReadExt, S: Into<String>>(rws: &mut Stream<R>, dict_name: S) -> Result<Rc<TexDictionary>> {
        let header = try!(Self::read_header(rws));

        let dict_name = Rc::new(dict_name.into());

        let num_textures = {
            if header.version < 0x1803FFFF { // lesser than 3.6.0.0
                try!(Struct::read_up(rws, |rws| {
                    Ok((try!(rws.read_u32::<LittleEndian>())))
                }))
            } else {
                let (count, _dev_id) = try!(Struct::read_up(rws, |rws| {
                    Ok((try!(rws.read_u16::<LittleEndian>()), try!(rws.read_u16::<LittleEndian>())))
                }));
                count as u32
            }
        };

        let mut textures = HashMap::with_capacity(num_textures as usize);
        for _ in (0..num_textures) {
            let (name, _mask, texture) = try!(TexNative::read(rws, &dict_name));
            textures.insert((*name).clone(), Rc::new(texture));
        }

        try!(Extension::skip_section(rws));

        Ok(Rc::new(TexDictionary {
            textures: textures,
        }))
    }
}

impl TexNative {
    pub fn read<R: ReadExt>(rws: &mut Stream<R>, dict_name: &Rc<String>) -> Result<(Rc<String>, Rc<String>, Texture)> {
        let header = try!(Self::read_header(rws));

        let platform_id = try!(Struct::peek_up(rws, |rws| Ok(try!(rws.read_u32::<LittleEndian>()))));

        let result = try!(match platform_id {
            2 => unimplemented!(),                              // OpenGL
            4 | 0x00325350 => unimplemented!(),                 // PS2 ("PS2/0")
            5 => unimplemented!(),                              // Xbox
            8 => Struct::read_up(rws, |rws| Self::read_struct_d3dx(rws, dict_name)),  // D3D8
            9 => Struct::read_up(rws, |rws| Self::read_struct_d3dx(rws, dict_name)),  // D3D9
            _ => Err(Error::Other(format!("Unknown texture dictionary platform id {}", platform_id))),
        });

        try!(Extension::skip_section(rws));

        Ok(result)
    }

    fn read_struct_d3dx<R: ReadExt>(rws: &mut Stream<R>, dict_name: &Rc<String>) -> Result<(Rc<String>, Rc<String>, Texture)> {
        // TODO TXDs are confusing, review this later

        let platform_id = try!(rws.read_u32::<LittleEndian>());
        assert!(platform_id == 8 || platform_id == 9);

        let filter_flags = try!(rws.read_u32::<LittleEndian>());

        let name = try!(String::from_null_terminated_buffer(try!(rws.read_bytes(32))));
        let mask = try!(String::from_null_terminated_buffer(try!(rws.read_bytes(32))));

        let raster_format = try!(rws.read_u32::<LittleEndian>());
        let d3d_format = try!(rws.read_u32::<LittleEndian>());

        let flag_auto_mip = (raster_format & 0x1000) != 0;
        let flag_ext_pal8 = (raster_format & 0x2000) != 0;
        let flag_ext_pal4 = (raster_format & 0x4000) != 0;
        let flag_mipmaps  = (raster_format & 0x8000) != 0;

        let width = try!(rws.read_u16::<LittleEndian>());
        let height = try!(rws.read_u16::<LittleEndian>());
        let depth = try!(rws.read_u8());
        let num_levels = try!(rws.read_u8());
        let raster_type = try!(rws.read_u8());
        let type_flags = try!(rws.read_u8());

        let has_alpha = (type_flags & 0x0001) != 0;
        let is_cubemap = (type_flags & 0x0002) != 0;
        let auto_mipmaps = (type_flags & 0x004) != 0;
        let is_compressed = (type_flags & 0x0008) != 0;

        let filter = FilterMode::from_raw(filter_flags & 0xFF).unwrap_or(FilterMode::None);
        let wrap_x = WrapMode::from_raw((filter_flags >> 8) & 0xF).unwrap_or(WrapMode::None);
        let wrap_y = WrapMode::from_raw((filter_flags >> 12) & 0xF).unwrap_or(WrapMode::None);
        let format = try!(RasterFormat::from_raw(raster_format)
                            .ok_or(Error::Other(format!("Invalid raster format {}", raster_format))));

        let data = {
            if (flag_ext_pal8 || flag_ext_pal4) && (true)  {
                unimplemented!();
            } else {
                // TODO check if raster_size matches the width height format things

                // TODO mipmaps

                let raster_size = try!(rws.read_u32::<LittleEndian>()) as usize;

                match (format, is_compressed, has_alpha) {
                    (RasterFormat::R5G6B5, true, false) => { // DXT1c
                        TextureData::Dxt1c(try!(rws.read_bytes(raster_size)))
                    },
                    (RasterFormat::R5G5B5A1, true, true) => { // DXT1a
                        TextureData::Dxt1a(try!(rws.read_bytes(raster_size)))
                    },
                    (RasterFormat::R4G4B4A4, true, true) => { // DXT3
                        TextureData::Dxt3(try!(rws.read_bytes(raster_size)))
                    },
                    _ => unimplemented!(),
                }
            }
        };

        let name = Rc::new(name);
        let mask = Rc::new(mask);

        let texture = Texture {
            dict: dict_name.clone(),
            name: name.clone(),
            mask: mask.clone(),
            data: data,
            width: width,
            height: height,
            filter: filter,
            wrap_x: wrap_x,
            wrap_y: wrap_y,
        };

        Ok((name, mask, texture))
    }
}
