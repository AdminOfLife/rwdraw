#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate itertools;
extern crate byteorder;
extern crate cgmath;
extern crate clock_ticks;

mod rw;

use std::fs::File;
use std::io::BufReader;

use std::ops::Range;

use cgmath::{Vector2, Vector3, Matrix4};

use glium::Surface;
use glium::VertexBuffer;
use glium::index::{IndexBuffer, IndexBufferSlice};
use glium::index::PrimitiveType;
use glium::backend::glutin_backend::GlutinFacade;

#[derive(Debug, Copy, Clone)]
struct Vertex {
    pos: Vector3<f32>,
    normal: Vector3<f32>,
    uv0: Vector2<f32>,
}

implement_vertex!(Vertex, pos, normal, uv0);

#[derive(Debug)]
struct MyMesh {
    range: Range<usize>,
}

#[derive(Debug)]
struct MyGeometry {
    vbo: VertexBuffer<Vertex>,
    ibo: IndexBuffer<u16>,
    meshes: Vec<MyMesh>,
}

fn build_geometry(facade: &GlutinFacade, rwgeo: &rw::Geometry) -> Option<MyGeometry> {
    // TODO do proper bounding checks
    // TODO check if size of uv0 == size of normals == size of verts
    // TODO other stuff other than TriStrip

    use std::mem::replace;

    let target0 = rwgeo.targets.get(0).unwrap();

    let vbo = match (target0.verts.as_ref(), target0.normals.as_ref(), rwgeo.uv_sets.get(0).as_ref()) {
        (Some(verts), Some(normals), Some(uv0)) => {
            VertexBuffer::new(facade, izip!(verts.iter(), normals.iter(), uv0.iter())
                .map(|(vert, normal, uv0): (&rw::Vec3, &rw::Vec3, &rw::Uv)|
                    Vertex {
                        pos: Vector3::new(vert.0, vert.1, vert.2),
                        normal: Vector3::new(normal.0, normal.1, normal.2),
                        uv0: Vector2::new(uv0.0, uv0.1),
                }).collect::<Vec<Vertex>>())
        },
        _ => unimplemented!(),
    };

    let mut indices = Vec::with_capacity(rwgeo.meshlist.total_indices as usize);

    let meshes = {
        rwgeo.meshlist.meshes.iter().scan(0, |curr, rwmesh| {
            let start = *curr;
            *curr = *curr + rwmesh.indices.len();
            indices.extend(rwmesh.indices.iter().cloned());
            Some(Range {
                start: start,
                end: start + rwmesh.indices.len(),
            })
        }).map(|range| MyMesh { range: range} ).collect()
    };

    let ibo = IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &indices);

    Some(MyGeometry {
        vbo: vbo,
        ibo: ibo,
        meshes: meshes,
    })
}

fn main() {
    use glium::DisplayBuild;
    use cgmath::{Point, Vector, Deg, Point2, Point3, PerspectiveFov, Matrix4};
    use std::collections::HashMap;
    use glium::glutin::{self, Event, ElementState, VirtualKeyCode, MouseButton};

    let x_res = 800.0f32;
    let y_res = 600.0f32;

    let display = glutin::WindowBuilder::new()
                            .with_title("rwdraw".into())
                            .with_vsync()
                            .with_dimensions(x_res as u32, y_res as u32)
                            .build_glium().unwrap();

    let mut curr_frame_time: f64 = clock_ticks::precise_time_s();
    let mut last_frame_time: f64 = curr_frame_time;

    // TODO less unwrap
    let mut f = BufReader::new(File::open(/*"newbuildsm01.dff"*/"box.dff").unwrap());
    let clump = rw::Clump::read(&mut f).unwrap();
    let atomic = clump.into_atomic().unwrap();
    let xg = build_geometry(&display, &atomic.geometry).unwrap();
    println!("{:?}", xg);

    let vertex_shader_src = r#"
        #version 140

        in vec3 pos;

        uniform mat4 modelViewProj;

        void main() {
            gl_Position = modelViewProj * vec4(pos.x, pos.z, pos.y, 1.0);// + vec4(0.0, 0.0, 0.0, 0.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        out vec4 color;

        void main() {
            color = vec4(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut t = -0.5;

    let mut keyboard = HashMap::with_capacity(256);

    let mut mouse_pos = Vector2::new((x_res as i32) / 2, (y_res as i32) / 2);
    display.get_window().unwrap().set_cursor_position(mouse_pos.x, mouse_pos.y);
    // TODO FIX UNWRAP ABOVE ON DISPLAY GET WINDOW


    // -2.190002 -0.52999985 points to the cube
    let mut horizontal_angle = -2.190002f32;
    let mut vertical_angle = -0.52999985f32;

    let mut position = Point3::new(4.0, 3.0, 3.0);
    let mouse_speed = 0.005f32;
    loop {
        last_frame_time = curr_frame_time;
        curr_frame_time = clock_ticks::precise_time_s();
        let delta_time  = (curr_frame_time - last_frame_time) as f32;

        // TODO optimize cos sin calls here
        let direction = Vector3::new(
            vertical_angle.cos() * horizontal_angle.sin(), 
            vertical_angle.sin(),
            vertical_angle.cos() * horizontal_angle.cos()
        );

        let right = Vector3::new(
            (horizontal_angle - 3.14 / 2.0).sin(), 
            0.0,
            (horizontal_angle - 3.14 / 2.0).cos()
        );

        let up = right.cross(&direction);

        for event in display.poll_events() {
            
            match event {
                Event::Closed => return,
                Event::MouseInput(state, MouseButton::Left) => {
                },
                Event::MouseMoved((x, y)) => {
                    if let Some(&(_, ElementState::Pressed)) = keyboard.get(&VirtualKeyCode::Q) {
                        horizontal_angle = horizontal_angle + mouse_speed * (mouse_pos.x - x) as f32;
                        vertical_angle = vertical_angle + mouse_speed * (mouse_pos.y - y) as f32;
                    }
                    mouse_pos.x = x;
                    mouse_pos.y = y;
                },
                Event::KeyboardInput(state, _, Some(vkey)) => {
                    let just_changed = keyboard.remove(&vkey)
                                               .map(|(_, oldstate)| oldstate == state)
                                               .unwrap_or(true);
                    keyboard.insert(vkey, (just_changed, state));
                },
                _ => ()
            }
        }

        if let Some(&(_, ElementState::Pressed)) = keyboard.get(&VirtualKeyCode::W) {
            // TODO P+V and V*S in cgmath
            position = position.add_v(&direction.mul_s(delta_time * 1.0f32));
        }

        let proj: Matrix4<f32> = Matrix4::<f32>::from(PerspectiveFov {
            fovy: Deg { s: 45.0f32 },
            aspect: 4.0f32 / 3.0f32,    // 4:3 FIXME any aspect ratio
            near: 0.1f32,
            far: 100.0f32,
        });

        // TODO add P+V in cgmath
       let view: Matrix4<f32> = Matrix4::look_at(&position, 
                                                 &(position.add_v(&direction)),
                                                 &up);

        let model: Matrix4<f32> = Matrix4::identity();

        let uniforms = uniform! {
            modelViewProj: proj * view * model,
        };


        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);
        for mesh in xg.meshes.iter() {
            target.draw(&xg.vbo, xg.ibo.slice(mesh.range.clone()).unwrap(), &program, &uniforms,
                        &Default::default()).unwrap();
        }
        target.finish();
    }
}
