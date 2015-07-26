// TODO NativeMaterial instead of pushing in stuff to the NativeMesh

use std::rc::Rc;
use std::collections::HashMap;
use super::{NativeDictionary, NativeDictionaryList, NativeTexture};

use rw;
use std::ops::Range;
use glium;
use glium::vertex::{VertexBuffer};
use glium::index::{IndexBuffer, PrimitiveType};
use glium::backend::Facade;

use cgmath::{Point3, Vector2, Vector4};

#[derive(Debug, Copy, Clone)]
pub struct VertexPrelit {
    pos: Point3<f32>,
    color: Vector4<f32>,
    uv0: Vector2<f32>,
}

implement_vertex!(VertexPrelit, pos, color, uv0);

#[derive(Debug)]
pub enum NativeVertexBuffer {
    Prelit(VertexBuffer<VertexPrelit>),
}

impl<'a> glium::vertex::IntoVerticesSource<'a> for &'a NativeVertexBuffer {
    fn into_vertices_source(self) -> glium::vertex::VerticesSource<'a> {
        use self::NativeVertexBuffer::*;
        match *self {
            Prelit(ref vbo) => vbo.into_vertices_source(),
        }
    }
}

#[derive(Debug)]
pub struct NativeMesh {
    pub range: Range<usize>,
    pub texture: Option<Rc<NativeTexture>>,
}

#[derive(Debug)]
pub struct NativeGeometry {
    pub vbo: NativeVertexBuffer,
    pub ibo: IndexBuffer<u16>,
    pub meshes: Vec<NativeMesh>,
}

impl NativeGeometry {

    pub fn from_rw<F: Facade>(facade: &F, rwgeo: &rw::Geometry,
                              dicts: &NativeDictionaryList) -> Option<NativeGeometry> {

        use self::NativeVertexBuffer::*;
        
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

                let maybe_buffer = VertexBuffer::new(facade,
                    izip!(verts.iter(), colors.iter(), uv0.iter()).map(|(vert, rgba, uv0)| {
                        VertexPrelit {
                            pos: (*vert).into(),
                            color: (*rgba).into(), // auto converts between 0-255 to 0-1 range
                            uv0: (*uv0).into(),
                        }
                    }).collect::<Vec<_>>().as_ref()
                );

                match maybe_buffer {
                    Ok(vbo) => NativeVertexBuffer::Prelit(vbo),
                    Err(_) => return None,
                }
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

                let texture = match rwmesh.material.texture {
                    Some(ref tex) => dicts.find(&tex.dict, &tex.name),
                    None => None,
                };

                meshes.push(NativeMesh {
                    range: Range { start: start, end: current_index },
                    texture: texture,
                });
            }

            (indices, meshes)
        };

        // TODO other formats other than TriStrip, check RwGeometry flags
        let index_buffer = {
            let maybe = IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &indices);
            match maybe {
                Ok(ibo) => ibo,
                Err(_) => return None,
            }
        };

        Some(NativeGeometry {
            vbo: vertex_buffer,
            ibo: index_buffer,
            meshes: meshes,
        })
    }
}
