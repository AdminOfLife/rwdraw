
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
extern crate image;

mod rw;

mod user;
use user::{UserCamera, UserControl};

use std::fs::File;
use std::io::BufReader;
use std::ops::Range;

use glium::{Surface, DisplayBuild};
use glium::vertex::{VertexBuffer};
use glium::index::{IndexBuffer, PrimitiveType};
use glium::glutin::{self, Event};
use glium::draw_parameters::DepthTest;
use glium::backend::Facade;

use cgmath::{Deg, Point3, PerspectiveFov, Matrix4, Vector2, Vector4};




#[derive(Debug, Copy, Clone)]
struct VertexPrelit {
    pos: Point3<f32>,
    color: Vector4<f32>,
    uv0: Vector2<f32>,
}

implement_vertex!(VertexPrelit, pos, color, uv0);

#[derive(Debug)]
enum NativeVertexBuffer {
    Prelit(VertexBuffer<VertexPrelit>),
}

impl<'a> glium::vertex::IntoVerticesSource<'a> for &'a NativeVertexBuffer {
    fn into_vertices_source(self) -> glium::vertex::VerticesSource<'a> {
        use NativeVertexBuffer::*;
        match *self {
            Prelit(ref vbo) => vbo.into_vertices_source(),
        }
    }
}

#[derive(Debug)]
struct NativeMesh {
    range: Range<usize>,
}

#[derive(Debug)]
struct NativeGeometry {
    vbo: NativeVertexBuffer,
    ibo: IndexBuffer<u16>,
    meshes: Vec<NativeMesh>,
}

impl NativeGeometry {

    fn from_rw<F: Facade>(rwgeo: &rw::Geometry, facade: &F) -> Option<NativeGeometry> {
        use NativeVertexBuffer::*;
        
        struct RwData<'a> {
            verts: Option<&'a Vec<rw::Vec3>>,
            normals: Option<&'a Vec<rw::Vec3>>,
            colors: Option<&'a Vec<rw::Rgba>>,
            uv0: Option<&'a Vec<rw::Uv>>,
        }

        // Gather all the information we need to pattern match this RwGeometry and build the
        // correct Vertex Buffer Object.
        let rwdata = {
            RwData {
                // ignore any morph target that is not the first one because gta uses only that.
                verts: rwgeo.targets.get(0).and_then(|target| target.verts.as_ref()),
                normals: rwgeo.targets.get(0).and_then(|target| target.normals.as_ref()),
                colors: rwgeo.colors.as_ref(),
                uv0: rwgeo.uv_sets.get(0),
            }
        };

        // Build the vertex buffer specific for this type of model, we gonna do this by pattern
        // matching the data we previosly built. 
        let vertex_buffer = match rwdata {
            // In case it's a prelit geometry...
            RwData { verts: Some(verts), normals: _, colors: Some(colors), uv0: Some(uv0) } => {

                // Maybe make this a pattern guard?
                if verts.len() != colors.len() || colors.len() != uv0.len() {
                    return None;
                }

                NativeVertexBuffer::Prelit(
                    VertexBuffer::new(
                        facade,
                        izip!(verts.iter(), colors.iter(), uv0.iter()).map(|(vert, rgba, uv0)| {
                            VertexPrelit {
                                pos: (*vert).into(),
                                color: (*rgba).into(), // auto converts between 0-255 to 0-1 range
                                uv0: (*uv0).into(),
                            }
                        }).collect::<Vec<_>>()
                    )
                )
            },
            // Not sure what we're dealing with:
            _ => return None,
        };

        // Builds the index buffer and meshes, a mesh basically consists of a range of indices in
        // the index buffer to be used to render a slice of the geometry.
        let (indices, meshes) = {
            let mut current_index = 0;
            let mut indices = Vec::with_capacity(rwgeo.meshlist.total_indices as usize);
            let mut meshes = Vec::with_capacity(rwgeo.meshlist.meshes.len());

            for rwmesh in rwgeo.meshlist.meshes.iter() {
                let start = current_index;  // beggining of current mesh
                current_index += rwmesh.indices.len();
                indices.extend(rwmesh.indices.iter().cloned());
                meshes.push(NativeMesh {
                    range: Range { start: start, end: current_index },
                });
            }

            (indices, meshes)
        };

        // TODO other formats other than TriStrip, check RwGeometry flags
        let index_buffer = IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &indices);

        Some(NativeGeometry {
            vbo: vertex_buffer,
            ibo: index_buffer,
            meshes: meshes,
        })
    }
}

fn main() {
    use std::ops::Deref;
    use std::io::Read;

    let x_res = 800.0f32;
    let y_res = 600.0f32;

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

    // TODO less unwrap
    let mut rw = rw::Instance::new();
    let f = BufReader::new(File::open("target/barrel4.dff").unwrap());
    let clump = rw::Clump::read(&mut rw::Stream::new(f, &mut rw)).unwrap();
    let atomic = clump.into_atomic().unwrap();
    let ximage = image::load(BufReader::new(File::open("target/redallu.png").unwrap()), image::PNG).unwrap();
    let texture = glium::texture::Texture2d::new(&display, ximage);
    let xg = NativeGeometry::from_rw(&atomic.geometry, &display).unwrap();
    println!("{:?}", xg);

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

        let view = camera.process_view_matrix(&user, delta_time);
        let model = Matrix4::<f32>::identity();

        let uniforms = uniform! {
            model_view_proj: proj * view * model,
            tex: &texture,
        };

        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);
        for mesh in xg.meshes.iter() {
            let params = glium::DrawParameters {
                depth_test: DepthTest::IfLess,
                depth_write: true,
                .. Default::default()
            };
            target.draw(&xg.vbo, xg.ibo.slice(mesh.range.clone()).unwrap(), &program, &uniforms, &params).unwrap();
        }
        target.finish();
    }
}
