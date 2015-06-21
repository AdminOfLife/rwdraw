use std::rc::Rc;
use std::collections::HashMap;

use rw;
use glium::texture::{ClientFormat, RawImage2d, CompressedSrgbTexture2d};
use glium::backend::Facade;


pub struct NativeDictionaryList {
    pub dicts: HashMap<String, NativeDictionary>,
}

#[derive(Debug)]
pub struct NativeDictionary {
    pub textures: HashMap<String, Rc<NativeTexture>>,
}

#[derive(Debug)]
pub struct NativeTexture {
    pub tex: CompressedSrgbTexture2d,
}

#[derive(Debug)]
pub enum NativeTextureBuffer {
    Compressed2d(CompressedSrgbTexture2d),
}

impl NativeDictionaryList {
    pub fn new() -> NativeDictionaryList {
        NativeDictionaryList {
            dicts: HashMap::new()
        }
    }

    pub fn add_rwdict<F: Facade>(&mut self, facade: &F, rwdict: &rw::TexDictionary) {
        let dictionary = NativeDictionary::from_rw(facade, rwdict);
        self.dicts.insert((*rwdict.name).clone(), dictionary);
    }

    pub fn find(&self, dict: &str, name: &str) -> Option<Rc<NativeTexture>> {
        self.dicts.get(dict).and_then(|dict| dict.find(name))
    }
}

impl NativeDictionary {
    pub fn from_rw<F: Facade>(facade: &F, rwdict: &rw::TexDictionary) -> NativeDictionary {
        NativeDictionary {
            textures: rwdict.textures.iter().map(|(texname, rctex)| {
                                (texname.clone(), Rc::new(NativeTexture::from_rw(facade, rctex)))
                      }).collect()  
        }
    }

    pub fn find(&self, name: &str) -> Option<Rc<NativeTexture>> {
        self.textures.get(name).map(|rctex| rctex.clone())
    }
}

impl NativeTexture {
    pub fn from_rw<F: Facade>(facade: &F, rwtex: &rw::Texture) -> NativeTexture {
        use rw::{TexLevel, TextureData};
        use glium::texture::{CompressedSrgbTexture2d, CompressedSrgbFormat, CompressedMipmapsOption};
        use glium::Rect;

        assert!(rwtex.mips.len() > 0);

        let format = match rwtex.mips[0] {
            TexLevel { data: TextureData::Dxt1c(_), .. } => CompressedSrgbFormat::S3tcDxt1NoAlpha,
            TexLevel { data: TextureData::Dxt1a(_), .. } => CompressedSrgbFormat::S3tcDxt1Alpha,
            TexLevel { data: TextureData::Dxt3(_), .. } => CompressedSrgbFormat::S3tcDxt3Alpha,
            _ => unimplemented!(),
        };

        let mips_gen = CompressedMipmapsOption::EmptyMipmapsMax(rwtex.mips.len() as u32 - 1);


        let tex = CompressedSrgbTexture2d::empty_with_format_if_supported(facade,
                                                             format,
                                                             mips_gen,
                                                             rwtex.width as u32, rwtex.height as u32).unwrap();//<<<<<<<<<

        for (level, rwmip) in rwtex.mips.iter().enumerate() {
            let level = level as u32;
            match *rwmip {
                TexLevel { data: TextureData::Dxt1c(ref data), width, height } |
                TexLevel { data: TextureData::Dxt1a(ref data), width, height } |
                TexLevel { data: TextureData::Dxt3(ref data), width, height } => {
                    println!("{} = {} {}", data.len(), width, height);
                    let rect = Rect { left: 0, bottom: 0, width: width as u32, height: height as u32 };
                    tex.mipmap(level).unwrap().write_compressed_data(rect, data, width as u32, height as u32, format);
                },
                _ => unimplemented!(),
            }
        }

        NativeTexture {
            tex: tex,
        }
    }

    // TODO at one point return a NativeTexture
    pub fn new_blank_texture<F: Facade>(facade: &F) -> CompressedSrgbTexture2d {
        use std::iter::repeat;
        CompressedSrgbTexture2d::new(facade, RawImage2d {
            width: 16,
            height: 16,
            format: ClientFormat::U8U8U8,
            data: repeat((255u8, 255, 255)).take(16*16).collect(),
        })
    }
}
