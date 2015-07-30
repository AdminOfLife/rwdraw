#![feature(alloc)]
#![feature(rc_weak)]
// TODO there are lots of unwraps because of testing, get rid of those and add proper error
// handling

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate itertools;
extern crate byteorder;
extern crate cgmath;
extern crate clock_ticks;
//extern crate rctree;
extern crate rustc_serialize;
extern crate docopt;

mod rw;
mod native;

use native::{NativeGeometry, NativeDictionaryList, NativeTexture};
use native::Renderer;

mod user;
use user::{UserCamera, UserControl};

mod img;
use img::{CdImage, CdEntry};

use docopt::Docopt;

use std::io::{Seek, Read};
use std::fs::File;
use std::path::{PathBuf, Path};
use std::io::BufReader;
use std::collections::HashMap;
use std::rc::Rc;

use glium::{Surface, DisplayBuild};
use glium::backend::Facade;
use glium::glutin::{self, Event};
use cgmath::{Deg, PerspectiveFov, Matrix4, Vector3, Vector4};

use std::ops::Deref;
use std::io::Cursor;


/*
pub fn load_clump_from_read<F, R1, R2>(facade: &F, dff: R1, txd: Option<R2>, txdname: &str) -> Option<native::Clump>
                                        where F: Facade, R1: Read + Seek, R2: Read + Seek {
    let mut rw = rw::Instance::new();

    println!("Loading texture dictionary...");
    let dictionary = match txd {
        Some(txd) => {
            rw::TexDictionary::read(&mut rw::Stream::new(txd, &mut rw), txdname).unwrap()
        },
        None => rw::TexDictionary::new_empty("noname"),
    };

    let mut dicts = NativeDictionaryList::new();
    dicts.add_rwdict(facade, &dictionary);

    println!("Loading atomic...");
    let maybe_clump = {
        rw.bind_dictionary(&dictionary);
        let clump = rw::Clump::read(&mut rw::Stream::new(dff, &mut rw)).unwrap();
        native::Clump::from_rw(facade, &clump, &dicts)
    };

    maybe_clump
}

pub fn load_clump<F, P1, P2>(facade: &F, dff: P1, txd: Option<P2>) -> Option<native::Clump>
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    let (txdname, file_txd) = match txd {
        Some(txd) => {
            let txd = txd.as_ref();
            let txdname = txd.file_stem().unwrap().to_string_lossy().into_owned();
            (txdname, Some(BufReader::new(File::open(&txd).unwrap())))
        },
        None => (String::new(), None),
    };

    let file_dff = {
        let dff = dff.as_ref();
        BufReader::new(File::open(dff).unwrap())
    };

    load_clump_from_read(facade, file_dff, file_txd, &txdname)
}

pub fn try_load_clump<F, P1, P2>(facade: &F, dff: Option<P1>, txd: Option<P2>) -> Option<native::Clump>
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    match (dff, txd) {
        (Some(dff), txd) => load_clump(facade, dff, txd),
        _ => None,
    }
}*/

pub fn load_dictionary<F, R>(facade: &F,
                             rw: &mut rw::Instance,
                             f: R, txdname: &str)
                            -> rw::Result<Rc<rw::TexDictionary>>
                            where F: Facade, R: Read + Seek {
    rw::TexDictionary::read(&mut rw::Stream::new(f, rw), txdname)
}

pub fn load_clump<F, R>(facade: &F, 
                        rw: &mut rw::Instance,
                        rwdict: &Rc<rw::TexDictionary>,
                        dicts: &NativeDictionaryList,
                        f: R) -> Option<native::Clump>
                        where F: Facade, R: Read + Seek {
    rw.bind_dictionary(rwdict);
    let rwclump = rw::Clump::read(&mut rw::Stream::new(f, rw)).unwrap();
    native::Clump::from_rw(facade, &rwclump, &dicts)
}



static USAGE: &'static str = "
    RenderWare Drawer

    Usage:
      rwdraw [options] <dffname> <txdname>...
      rwdraw (-h | --help)
      rwdraw --version

    Options:
      -h --help     Show this screen.
      --version     Show version.
      --img=<path>  Reads the <dffname> and <txdname> from the specified img file.
                    Filesystem paths are still accepted on <dffname> and <txdname>.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_img: Option<String>,
    arg_dffname: String,
    arg_txdname: Vec<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .map(|d| d.help(true))
                            .map(|d| d.version(Some("rwdraw 0.1.0".to_owned())))
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
    display_main(args)
}


fn display_main(args: Args) {
    let x_res = 800.0f32;
    let y_res = 600.0f32;

    // Graphics context
    let display = glutin::WindowBuilder::new()
                            .with_title("rwdraw".to_owned())
                            .with_vsync()
                            .with_dimensions(x_res as u32, y_res as u32)
                            .build_glium().unwrap();

    // User interaction controller
    let mut camera = UserCamera::new();
    let mut user = match display.get_window() {
        Some(window) => UserControl::new(Some(window.deref()), (x_res as i32, y_res as i32)),
        None => UserControl::new(None, (x_res as i32, y_res as i32)),
    };

    // cargo run -- "target/containercrane_04.dff" "target/cranes_dyn2_cj.txd"
    // Program data
    let mut should_reload_model = true;
    let mut clump = None;
    let mut cd = args.flag_img.and_then(|flag_img| {
        CdImage::open(PathBuf::from(flag_img)).ok()
    });
    let mut dffname = Some(PathBuf::from(args.arg_dffname));
    let mut txdname = Some(PathBuf::from(args.arg_txdname[0].clone()));
    
    // Frame timing for delta time
    let mut curr_frame_time: f64 = clock_ticks::precise_time_s();
    let mut last_frame_time: f64;

    // Program object / shaders for rendering.
    let program = {
        // TODO remove unwraps
        let mut vertex_shader_src = String::with_capacity(512);
        BufReader::new(
            File::open(r"src/shader/gta3_prelit_tex1.vs.glsl").unwrap()
        ).read_to_string(&mut vertex_shader_src).unwrap();

        let mut fragment_shader_src = String::with_capacity(512);
        BufReader::new(
            File::open(r"src/shader/gta3_prelit_tex1.fs.glsl").unwrap()
        ).read_to_string(&mut fragment_shader_src).unwrap();

        glium::Program::from_source(&display, 
                                    &vertex_shader_src,
                                    &fragment_shader_src,
                                    None).unwrap()
    };

    // Blank texture
    let tex_blank = Rc::new(NativeTexture::new_blank_texture(&display));

    // Setup projection matrix
    let proj: Matrix4<f32> = Matrix4::<f32>::from(PerspectiveFov {
        fovy: Deg { s: 45.0 },
        aspect: x_res / y_res,
        near: 0.1,
        far: 1000.0,
    });

    // Setup XZY to XYZ coordinate system matrix
    let xzy_to_xyz = Matrix4::<f32>::from_cols(
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, -1.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    );

    loop {
        last_frame_time = curr_frame_time;
        curr_frame_time = clock_ticks::precise_time_s();
        let delta_time  = (curr_frame_time - last_frame_time) as f32;
        
        user.process(None);
        for event in display.poll_events() {
            user.process(Some(event.clone()));
            match event {
                Event::Closed => return,
                Event::DroppedFile(path) => {
                    match path.extension().and_then(|os| os.to_str()).map(|s| s.to_lowercase()) {
                        Some(ref ext) if ext == "txd" => {
                            txdname = Some(path);
                            should_reload_model = true;
                        },
                        Some(ref ext) if ext == "dff" => {
                            dffname = Some(path);
                            should_reload_model = true;
                        },
                        _ => (),
                    }
                },
                _ => (),
            }
        }

        if should_reload_model {
            should_reload_model = false;

            // TODO remove unwraps
            clump = match cd {
                Some(ref mut cd) => {
                    let mut rw = rw::Instance::new();

                    let mut dicts = NativeDictionaryList::new();

                    let dffpath = dffname.clone().unwrap();
                    let txdpath = txdname.clone().unwrap();

                    let txd_fname = txdpath.file_name().unwrap().to_string_lossy().into_owned();
                    let txd_name = txdpath.file_stem().unwrap().to_string_lossy().into_owned();

                    let dff_fname = dffpath.file_name().unwrap().to_string_lossy().into_owned();
                    let dff_name = dffpath.file_stem().unwrap().to_string_lossy().into_owned();

                    let rwdict = cd.read(&txd_fname).ok().and_then(|data| {
                        load_dictionary(&display, &mut rw, Cursor::new(data), &txd_name).ok()
                    }).unwrap();

                    dicts.add_rwdict(&display, &rwdict);

                    cd.read(&dff_fname).ok().and_then(|data| {
                        load_clump(&display, &mut rw, &rwdict, &dicts, Cursor::new(data))
                    })
                },
                None => None,
            };
        }

        let mut renderer = Renderer::new(display.draw(), tex_blank.clone());
        renderer.target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);

        let view = camera.process_view_matrix(&user, delta_time);

        if let Some(ref clump) = clump {
            let model = Matrix4::<f32>::identity();
            clump.render(&mut renderer, &program, &proj, &(view * xzy_to_xyz * model));
        }

        renderer.into_surface().finish().unwrap();
    } 
}
