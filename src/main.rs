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

mod user;
use user::{UserCamera, UserControl};

use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::collections::HashMap;

use glium::{Surface, DisplayBuild};
use glium::backend::Facade;
use glium::glutin::{self, Event};
use cgmath::{Deg, PerspectiveFov, Matrix4, Vector3, Vector4};

pub fn load_model<F, P1, P2>(facade: &F, dff: P1, txd: P2) -> NativeGeometry
                             where F: Facade, P1: AsRef<Path>, P2: AsRef<Path> {
    let mut rw = rw::Instance::new();

    let (dff, txd) = (dff.as_ref(), txd.as_ref());

    println!("Loading texture dictionary...");
    let dictionary = {
        let txdname = txd.file_stem().unwrap().to_string_lossy().into_owned();
        let f = BufReader::new(File::open(&txd).unwrap());
        rw::TexDictionary::read(&mut rw::Stream::new(f, &mut rw), txdname).unwrap()
    };

    println!("Loading atomic...");
    let atomic = {
        rw.bind_dictionary(&dictionary);
        let f = BufReader::new(File::open(dff).unwrap());
        let clump = rw::Clump::read(&mut rw::Stream::new(f, &mut rw)).unwrap();
        clump.into_atomic().unwrap()
    };

    let mut dicts = NativeDictionaryList::new();
    dicts.add_rwdict(facade, &dictionary);

    NativeGeometry::from_rw(facade, &atomic.geometry, &dicts).unwrap()
}

fn main() {
    use std::ops::Deref;
    use std::io::Read;

    let x_res = 800.0f32;
    let y_res = 600.0f32;

    let (dffname, txdname) = {
        let mut args = std::env::args();
        match (args.next(), args.next(), args.next()) {
            (Some(_), Some(dffname), Some(txdname)) => {
                println!("Using clump '{}' with dictionary '{}'", dffname, txdname);
                (dffname, txdname)
            },
            _ => {
                println!("Usage: rwdraw <dffpath> <txdpath>");
                return;
            },
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

    let tex_blank = NativeTexture::new_blank_texture(&display);

    // cargo run -- "target/containercrane_04.dff" "target/cranes_dyn2_cj.txd"
    let natgeo = load_model(&display, dffname, txdname);

    loop {
        last_frame_time = curr_frame_time;
        curr_frame_time = clock_ticks::precise_time_s();
        let delta_time  = (curr_frame_time - last_frame_time) as f32;

        user.process(None);
        for event in display.poll_events() {
            user.process(Some(event.clone()));
            match event {
                Event::Closed => return,
                _ => (),
            }
        }

        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);

        let xzy_to_xyz = Matrix4::<f32>::from_cols(
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, -1.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );

        let view = camera.process_view_matrix(&user, delta_time);

        for &model in &[Matrix4::<f32>::identity()]
        {
            for mesh in natgeo.meshes.iter() {

                use glium::draw_parameters::{DepthTest, BlendingFunction};
                use glium::draw_parameters::LinearBlendingFactor::*;

                let uniforms = uniform! {
                    model_view_proj: proj * view * xzy_to_xyz * model,
                    tex: mesh.texture.as_ref().map(|texture| &texture.tex).unwrap_or(&tex_blank),
                };

                let params = glium::DrawParameters {
                    depth_test: DepthTest::IfLess,
                    depth_write: true,
                    blending_function: Some(BlendingFunction::Addition { source: SourceAlpha, destination: OneMinusSourceAlpha }),
                    .. Default::default()
                };
                target.draw(&natgeo.vbo, natgeo.ibo.slice(mesh.range.clone()).unwrap(), &program, &uniforms, &params).unwrap();
            }
        }

        target.finish().unwrap();
    }
}
