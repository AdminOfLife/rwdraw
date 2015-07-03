use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt, Stream};

use super::{GeometryList, Atomic, Light, Extension};
use super::{FrameList, Frame, FrameObjectValue, FrameObject};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Container for Atomics.
///
/// Clumps are containers for `Atomic`s and can also be linked to their own `Frame`.
///
/// Clumps are intended to group related Atomics together. The Atomics are stored by refcount,
/// so an Atomic can be stored in more than one Clump if necessary.
#[derive(Debug)]
pub struct Clump {
    parent: RefCell<Option<Weak<Frame>>>,
    atomics: Vec<Rc<Atomic>>,
    frames: FrameList,
}

impl Section for Clump {
    fn section_id() -> u32 { 0x0010 }
}

impl FrameObject for Rc<Clump> {
    fn to_frame_object(&self) -> FrameObjectValue {
        FrameObjectValue::Clump(self.clone())
    }

    fn frame_refcell(&self) -> &RefCell<Option<Weak<Frame>>> {
        &self.parent
    }
}

impl Clump {
    /// Constructs a clump containing the related atomics.
    pub fn new(atomics: Vec<Rc<Atomic>>, frames: FrameList) -> Clump {
        Clump {
            parent: RefCell::new(None),
            atomics: atomics,
            frames: frames,
        }
    }

    /// Constructs a clump attached to `frame` and containing the related atomics.
    pub fn with_frame(frame: Option<Rc<Frame>>,
                      atomics: Vec<Rc<Atomic>>, frames: FrameList)
                      -> Rc<Clump> {

        let clump = Rc::new(Clump::new(atomics, frames));
        clump.set_frame(frame);
        clump
    }

    /// Reads a Clump off a RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<Clump> {
        let _header = try!(Self::read_header(rws));

        let (natomics, nlights, ncameras) = try!(Struct::read_up(rws, |rws| {
            Ok((try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>())))
        }));

        let framelist = try!(FrameList::read(rws));
        let geolist = try!(GeometryList::read(rws));

        let atomics = try!((0..natomics).map(|_| Atomic::read(rws, &framelist, &geolist)).collect());

        // TODO Light
        for _ in (0..nlights) {
            
            try!(Struct::skip_section(rws));
            try!(Light::skip_section(rws));
        }

        // TODO Camera
        for _ in (0..ncameras) {
            // ----
        }

        try!(Extension::skip_section(rws));

        Ok(Clump::new(atomics, framelist))
    }

    /// Converts this `Clump` into a single `Atomic`.
    ///
    /// This extracts the last atomic of the clump's atomic list.
    ///
    /// It's recommend to detach the returned atomic from any frame using `Atomic::detach` on the
    /// returned atomic but not required.
    pub fn into_atomic(mut self) -> Option<Rc<Atomic>> {
        self.atomics.pop()
    }
}
