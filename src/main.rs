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

mod rw;
mod native;

use native::{NativeGeometry, NativeDictionaryList, NativeTexture};
use native::Renderer;

mod user;
use user::{UserCamera, UserControl};

use std::fs::File;
use std::path::{PathBuf, Path};
use std::io::BufReader;
use std::collections::HashMap;
use std::rc::Rc;

use glium::{Surface, DisplayBuild};
use glium::backend::Facade;
use glium::glutin::{self, Event};
use cgmath::{Deg, PerspectiveFov, Matrix4, Vector3, Vector4};

pub fn load_atomic<F, P1, P2>(facade: &F, dff: P1, txd: Option<P2>) -> NativeGeometry
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    let mut rw = rw::Instance::new();

    //println!("Using clump '{}' with dictionary '{}'", dff, txd);

    println!("Loading texture dictionary...");
    let dictionary = match txd {
        Some(txd) => {
            let txd = txd.as_ref();
            let txdname = txd.file_stem().unwrap().to_string_lossy().into_owned();
            let f = BufReader::new(File::open(&txd).unwrap());
            rw::TexDictionary::read(&mut rw::Stream::new(f, &mut rw), txdname).unwrap()
        },
        None => rw::TexDictionary::new_empty("noname"),
    };

    println!("Loading atomic...");
    let atomic = {
        let dff = dff.as_ref();
        rw.bind_dictionary(&dictionary);
        let f = BufReader::new(File::open(dff).unwrap());
        let clump = rw::Clump::read(&mut rw::Stream::new(f, &mut rw)).unwrap();
        clump.into_atomic().unwrap()
    };

    let mut dicts = NativeDictionaryList::new();
    dicts.add_rwdict(facade, &dictionary);

    NativeGeometry::from_rw(facade, &atomic.geometry, &dicts).unwrap()
}

pub fn try_load_atomic<F, P1, P2>(facade: &F, dff: Option<P1>, txd: Option<P2>) -> Option<NativeGeometry>
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    match (dff, txd) {
        (Some(dff), txd) => Some(load_atomic(facade, dff, txd)),
        _ => None,
    }
}

pub fn load_clump<F, P1, P2>(facade: &F, dff: P1, txd: Option<P2>) -> native::Clump
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    let mut rw = rw::Instance::new();

    //println!("Using clump '{}' with dictionary '{}'", dff, txd);

    println!("Loading texture dictionary...");
    let dictionary = match txd {
        Some(txd) => {
            let txd = txd.as_ref();
            let txdname = txd.file_stem().unwrap().to_string_lossy().into_owned();
            let f = BufReader::new(File::open(&txd).unwrap());
            rw::TexDictionary::read(&mut rw::Stream::new(f, &mut rw), txdname).unwrap()
        },
        None => rw::TexDictionary::new_empty("noname"),
    };

    let mut dicts = NativeDictionaryList::new();
    dicts.add_rwdict(facade, &dictionary);

    println!("Loading clump...");
    let clump = {
        let dff = dff.as_ref();
        rw.bind_dictionary(&dictionary);
        let f = BufReader::new(File::open(dff).unwrap());
        let clump = rw::Clump::read(&mut rw::Stream::new(f, &mut rw)).unwrap();
        native::Clump::from_rw(facade, &clump, &dicts)
    };

    clump.unwrap()
}

pub fn try_load_clump<F, P1, P2>(facade: &F, dff: Option<P1>, txd: Option<P2>) -> Option<native::Clump>
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    match (dff, txd) {
        (Some(dff), txd) => Some(load_clump(facade, dff, txd)),
        _ => None,
    }
}

fn main() {
    use std::ops::Deref;
    use std::io::Read;

    let x_res = 800.0f32;
    let y_res = 600.0f32;

    let (mut dffname, mut txdname) = {
        let mut args = std::env::args();
        match (args.next(), args.next(), args.next()) {
            (Some(_), dffname, txdname) => {
                (dffname.map(PathBuf::from), txdname.map(PathBuf::from))
            },
            _ => (None, None),
        }
    };

    let display = glutin::WindowBuilder::new()
                            .with_title("rwdraw".into())
                            .with_vsync()
                            .with_dimensions(x_res as u32, y_res as u32)
                            .build_glium().unwrap();

    let mut user = match display.get_window() {
        Some(window) => UserControl::new(Some(window.deref()), (x_res as i32, y_res as i32)),
        None => UserControl::new(None, (x_res as i32, y_res as i32)),
    };

    let mut curr_frame_time: f64 = clock_ticks::precise_time_s();
    let mut last_frame_time: f64;

    let mut vertex_shader_src = String::with_capacity(512);
    BufReader::new(
        File::open(r"src/shader/gta3_prelit_tex1.vs.glsl").unwrap()
    ).read_to_string(&mut vertex_shader_src).unwrap();

    let mut fragment_shader_src = String::with_capacity(512);
    BufReader::new(
        File::open(r"src/shader/gta3_prelit_tex1.fs.glsl").unwrap()
    ).read_to_string(&mut fragment_shader_src).unwrap();

    let program = glium::Program::from_source(&display, 
                                              &vertex_shader_src,
                                              &fragment_shader_src,
                                              None).unwrap();

    let mut camera = UserCamera::new();

    let proj: Matrix4<f32> = Matrix4::<f32>::from(PerspectiveFov {
        fovy: Deg { s: 45.0 },
        aspect: x_res / y_res,
        near: 0.1,
        far: 1000.0,
    });

    let tex_blank = Rc::new(NativeTexture::new_blank_texture(&display));

    let xzy_to_xyz = Matrix4::<f32>::from_cols(
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, -1.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    );

    // cargo run -- "target/containercrane_04.dff" "target/cranes_dyn2_cj.txd"
    let mut should_reload_model = true;
    //let mut natgeo = None;//try_load_model(&display, dffname, txdname);
    let mut clump = None;

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
            //natgeo = try_load_model(&display, dffname.clone(), txdname.clone());
            clump = try_load_clump(&display, dffname.clone(), txdname.clone());
            should_reload_model = false;
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
