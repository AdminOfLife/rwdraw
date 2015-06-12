use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{Extension, Vec3, Uv, Sphere, Rgba};
use super::{Material, MaterialList};

use std::rc::Rc;

#[derive(Debug)]
pub struct Mesh {
	pub mat: Rc<Material>,
	pub indices: Vec<u16>,
}

#[derive(Debug)]
pub struct MeshHeader {
	pub is_tri_strip: bool,
	pub total_indices: u32,
	pub meshes: Vec<Mesh>,
}

// `Face`s are necessary only to calculate `Mesh`es, though those meshes are mostly like already
// precalculated inside our model files (dff).
#[derive(Debug, Copy, Clone)]
pub struct Face {
	pub y_id: u16,
	pub x_id: u16,
	pub mat_id: u16,
	pub z_id: u16,
}

#[derive(Debug)]
pub struct MorphTarget {
	pub sphere: Sphere,
	pub unk1: u32, unk2: u32,
	pub verts: Option<Vec<Vec3>>,
	pub normals: Option<Vec<Vec3>>,
}

#[derive(Debug)]
pub struct Geometry {
    pub is_tri_strip: bool,
	pub colors: Option<Vec<Rgba>>,
	pub uv_sets: Vec<Vec<Uv>>,
	pub faces: Vec<Face>,
	pub targets: Vec<MorphTarget>,
    pub matlist: MaterialList,
    pub meshlist: MeshHeader,
}

#[derive(Debug)]
pub struct GeometryList(pub Vec<Rc<Geometry>>);

impl Section for GeometryList {
	fn section_id() -> u32 { 0x001A }
}

impl Section for Geometry {
	fn section_id() -> u32 { 0x000F }
}

impl Section for MeshHeader {
	fn section_id() -> u32 { 0x050E }	// Bin Mesh PLG
}

impl GeometryList {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<GeometryList> {
		let _header = try!(Self::read_header(rws));
		let numgeo = try!(Struct::read_up(rws, |rws| { Ok(try!(rws.read_u32::<LittleEndian>())) }));
		
		let mut geolist = Vec::with_capacity(numgeo as usize);
		for _ in (0..numgeo) {
			geolist.push( Rc::new(try!(Geometry::read(rws))) );
		}

		Ok(GeometryList(geolist))
	}
}

impl Geometry {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Geometry> {
		let header = try!(Self::read_header(rws));

		// the struct section is pretty hunge in the geometry, maybe we should put it in another
		// function...
		let (flags, colors, uv_sets, faces, targets) = try!(Struct::read_up(rws, |rws| {

			let flags = try!(rws.read_u16::<LittleEndian>());
			let num_uv = try!(rws.read_u8());
			let _natflags = try!(rws.read_u8()); // TODO what is this?
			let num_tris = try!(rws.read_u32::<LittleEndian>());
			let num_verts = try!(rws.read_u32::<LittleEndian>());
			let num_morphs = try!(rws.read_u32::<LittleEndian>());

            // On 3.4.0.3 and below there are some additional information
            let _amb_difu_spec = {
                if header.version <= 0x1003FFFF {
                    Some((
                        try!(rws.read_f32::<LittleEndian>()),
                        try!(rws.read_f32::<LittleEndian>()),
                        try!(rws.read_f32::<LittleEndian>()),
                    ))
                } else {
                    None
                }
            };

			// diffuse color
			let colors = {
				if (flags & 8) != 0 {
					let mut v = Vec::with_capacity(num_verts as usize);
					for _ in (0..num_verts) {
						v.push(try!(Rgba::read(rws)));
					}
					Some(v)
				} else {
					None
				}
			};

			// uv coordinate sets
			let uv_sets = {
				let mut sets = Vec::with_capacity(num_uv as usize);
				for _ in (0..num_uv) {
					let mut v = Vec::with_capacity(num_verts as usize);
					for _ in (0..num_verts) {
						v.push(try!(Uv::read(rws)));
					}
					sets.push(v)
				}
				sets
			};

			// triangles / faces
			let faces = {
				let mut v = Vec::with_capacity(num_tris as usize);
				for _ in (0..num_tris) {
					v.push(Face {
						y_id: try!(rws.read_u16::<LittleEndian>()),
						x_id: try!(rws.read_u16::<LittleEndian>()),
						mat_id: try!(rws.read_u16::<LittleEndian>()),
						z_id: try!(rws.read_u16::<LittleEndian>()),
					});
				}
				v
			};
			
			// morph targets (well, the mesh)
			let targets = {
				let mut v = Vec::with_capacity(num_morphs as usize);
				for _ in (0..num_morphs) {
					v.push(MorphTarget {
						sphere: try!(Sphere::read(rws)),
						unk1: try!(rws.read_u32::<LittleEndian>()),
						unk2: try!(rws.read_u32::<LittleEndian>()),
						verts: {
							if (flags & 2) != 0 {
								let mut verts = Vec::with_capacity(num_verts as usize);
								for _ in (0..num_verts) {
									verts.push(try!(Vec3::read(rws)));
								}
								Some(verts)
							} else {
								None
							}
						},
						normals: {
							if (flags & 16) != 0 {
								let mut normz = Vec::with_capacity(num_verts as usize);
								for _ in (0..num_verts) {
									normz.push(try!(Vec3::read(rws)));
								}
								Some(normz)
							} else {
								None
							}
						},
					});
				}
				v
			};

			Ok((flags, colors, uv_sets, faces, targets))
		}));

		// material list
        let matlist = try!(MaterialList::read(rws));

		// extension
		let meshlist = try!(Extension::read_for(rws, |rws| MeshHeader::read(rws, &matlist)));

		Ok(Geometry {
            is_tri_strip: (flags & 1) != 0,
			colors: colors,
			uv_sets: uv_sets,
			faces: faces,
			targets: targets,
            matlist: matlist,
            meshlist: meshlist.unwrap_or_else(|| {
            	unimplemented!()	// calculate meshlist ourselves
            }),
		})
	}
}

impl MeshHeader {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>, matlist: &MaterialList) -> Result<MeshHeader> {
		let _header = try!(Self::read_header(rws));

		let flags = try!(rws.read_u32::<LittleEndian>());
		let nmesh = try!(rws.read_u32::<LittleEndian>());
		let total_nidx = try!(rws.read_u32::<LittleEndian>());

		Ok(MeshHeader {
			is_tri_strip: (flags & 1) != 0,
			total_indices: total_nidx,
			meshes: try!((0..nmesh).map(|_| Mesh::read(rws, matlist)).collect()),
		})
	}
}

impl Mesh {
	pub fn read<R: ReadExt>(rws: &mut Stream<R>, matlist: &MaterialList) -> Result<Mesh> {
		let nidx = try!(rws.read_u32::<LittleEndian>()) as usize;
		let matid = try!(rws.read_u32::<LittleEndian>()) as usize;
		Ok(Mesh {
			mat: try!(matlist.0.get(matid).map(|rcmat| rcmat.clone())
				               .ok_or(Error::Other("Invalid 'Mesh' material id".into()))),
			indices: {
				let mut v = Vec::with_capacity(nidx);
				for _ in (0..nidx) {
					v.push(try!(rws.read_u32::<LittleEndian>().map(|x| x as u16)));
				}
				v
			},
		})
	}
}
