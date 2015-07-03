use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, Error, ReadExt, Stream};

use super::{FrameList, Frame, FrameObjectValue, FrameObject};
use super::{GeometryList, Geometry, Extension};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

bitflags! {
    /// Specifies the options available for controlling the behavior of atomics.
    flags AtomicFlags: u32 {
        /// A generic collision flag to indicate that the atomic should be considered in collision tests.
        const COLLISION_TEST = 0x1,
        /// The atomic is rendered if it is in the view frustum. 
        const RENDER = 0x4,
    }
}

/// Container object for Geometry and Frame objects.
///
/// Complex models are often built up of sections grouped into a hierarchy and `Atomic`s provide the
/// necessary linkage with `Frame` objects to enable this. When used in this way, related Atomics
/// are usually grouped together by adding them to a Clump for convenience, although this is not mandatory.
///
/// Atomics also contain a reference to it's associated `Geometry`, which holds the actual model data.
#[derive(Debug)]
pub struct Atomic {
    parent: RefCell<Option<Weak<Frame>>>,
    pub geometry: Rc<Geometry>,
    pub flags: AtomicFlags,
}


impl Section for Atomic {
    fn section_id() -> u32 { 0x0014 }
}

impl FrameObject for Rc<Atomic> {
    fn to_frame_object(&self) -> FrameObjectValue {
        FrameObjectValue::Atomic(self.clone())
    }

    fn frame_refcell(&self) -> &RefCell<Option<Weak<Frame>>> {
        &self.parent
    }
}

impl Atomic {
    /// Constructs an atomic containing the specified geometry.
    pub fn new(flags: AtomicFlags, geometry: Rc<Geometry>) -> Atomic {
        Atomic {
            parent: RefCell::new(None),
            geometry: geometry,
            flags: flags,
        }
    }

    /// Constructs an atomic attached to `frame` containing the specified geometry.
    pub fn with_frame(frame: Option<Rc<Frame>>,
                      flags: AtomicFlags, geometry: Rc<Geometry>)
                      -> Rc<Atomic> {
        let atomic = Rc::new(Atomic::new(flags, geometry));
        atomic.set_frame(frame);
        atomic
    }

    /// Reads the `Atomic` off the RenderWare Stream.
    ///
    /// The previosly read geometry and frames from the same clump object in the stream must
    /// be passed to this read procedure.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>, framelist: &FrameList, geolist: &GeometryList)
                                                                           -> Result<Rc<Atomic>> {
        let _header = try!(Self::read_header(rws));

        let (frame_index, geo_index, flags, _) = try!(Struct::read_up(rws, |rws| {
            Ok((try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>()),
                try!(rws.read_u32::<LittleEndian>())))  // unused
        }));

        // A geometry is available on the Atomic stream when the Clump geometry list is empty.
        let geometry = match geolist {
            ref geolist if geolist.0.is_empty() => Rc::new(try!(Geometry::read(rws))),
            ref geolist => try!(geolist.get(geo_index as usize)
                                       .ok_or(Error::Other("Invalid 'Atomic' Geometry id".into()))),
        };

        // Extensions.
        try!(Extension::skip_section(rws));

        Ok(Atomic::with_frame(
             framelist.get(frame_index as usize),
             AtomicFlags::from_bits_truncate(flags),
             geometry))
    }
}
