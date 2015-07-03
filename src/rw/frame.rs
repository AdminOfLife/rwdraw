use byteorder::{ReadBytesExt, LittleEndian};
use super::{Section, Struct, Result, ReadExt, Stream};

use super::{Matrix, Clump, Atomic, Extension};
use std::cell::{Ref, RefCell};
use std::rc::{Rc, Weak};

/// Holds a list of `Frame`s to be passed around.
#[derive(Debug)]
pub struct FrameList(pub Vec<Rc<Frame>>);

/// Hierarchical (transform and name) wrapper for objects.
///
/// This object provides a frame of reference for other objects, allowing them to be positioned
/// relative to each other in world space. In addition, a frame object can also be linked to parent
/// and/or child frames in a hierarchical manner.
#[derive(Debug)]
pub struct Frame {
    parent: RefCell<Option<Weak<Frame>>>,
    childs: RefCell<Vec<FrameObjectValue>>,
    matrix: Matrix,
    // I wish this wasn't a RefCell but an extension is responssible for setting it up.
    // May find a solution for this later.
    name: RefCell<String>,
}

/// Holds an object that can be attached to a frame.
#[derive(Debug, Clone)]
pub enum FrameObjectValue {
    Clump(Rc<Clump>),
    Atomic(Rc<Atomic>),
    Frame(Rc<Frame>),
}

// TODO free Weak node if it's not upgradeable

// Implemented by Rc FrameObjects (Rc<Frame>, Rc<Atomic>, ...).
pub trait FrameObject {
    /// Gets a `FrameObjectValue` from this.
    fn to_frame_object(&self) -> FrameObjectValue;

    /// Gets the parent RefCell holder.
    #[doc(hidden)]
    fn frame_refcell(&self) -> &RefCell<Option<Weak<Frame>>>;

    /// Gets the parent frame associated with this object.
    fn frame(&self) -> Option<Rc<Frame>> {
        self.frame_refcell().borrow().clone().and_then(|weak| weak.upgrade())
    }

    /// Sets the parent frame associated with this object.
    ///
    /// If the `frame` parameter is `None` this has the same effect as the `detach_frame` method.
    ///
    /// Any previosly attached frame gets detached.
    fn set_frame(&self, frame: Option<Rc<Frame>>) {
        self.detach_frame();
        if let Some(frame) = frame {
            *self.frame_refcell().borrow_mut() = Some(frame.downgrade());
            Frame::push_child(&frame, self.to_frame_object());
        }
    }

    /// Detaches this object off any parent frame.
    ///
    /// Returns the previosly attached frame.
    fn detach_frame(&self) -> Option<Rc<Frame>> {
        if let Some(parent) = self.frame() {
            *self.frame_refcell().borrow_mut() = None;
            Frame::remove_child(&parent, self.to_frame_object());
            Some(parent)
        } else {
            None
        }
    }
}

/// Rockstar North's Node Name Plugin.
///
/// This plugin is used to associate a name into a `Frame`.
#[derive(Debug)]
pub struct NodeNamePlg(String);

impl Section for FrameList {
    fn section_id() -> u32 { 0x000E }
}

impl Section for NodeNamePlg {
    fn section_id() -> u32 { 0x0253F2FE }
}

impl FrameObjectValue {
    /// Checks if two `FrameObjectValue`s point to exactly same object (same address).
    pub fn is_same_object(&self, other: &FrameObjectValue) -> bool {
        use std::ops::Deref;
        use self::FrameObjectValue::*;
        match (self, other) {
            // Frames may be same as another frame...
            (&Frame(ref rca), &Frame(ref rcb)) => {
                rca.deref() as *const _ == rcb.deref() as *const _
            },
            // Atomics may be same as another atomic...
            (&Atomic(ref rca), &Atomic(ref rcb)) => {
                rca.deref() as *const _ == rcb.deref() as *const _
            },
            // Clumps may be same as another clump...
            (&Clump(ref rca), &Clump(ref rcb)) => {
                rca.deref() as *const _ == rcb.deref() as *const _
            },
            // Inequal object types...
            _ => false,
        }
    }
}

impl FrameObject for Rc<Frame> {
    fn to_frame_object(&self) -> FrameObjectValue {
        FrameObjectValue::Frame(self.clone())
    }

    fn frame_refcell(&self) -> &RefCell<Option<Weak<Frame>>> {
        &self.parent
    }
}

impl Frame {

    /// Adds a child object into the specified `Frame`.
    ///
    /// This is a helper to `FrameObject`.
    fn push_child(myself: &Rc<Frame>, child: FrameObjectValue) {
        myself.childs.borrow_mut().push(child);
    }

    /// Removes the specified child object off the `Frame`.
    ///
    /// This is a helper to `FrameObject`.
    fn remove_child(myself: &Rc<Frame>, child: FrameObjectValue) {
        let mut childs = myself.childs.borrow_mut();
        if let Some(pos) = childs.iter().position(|obj| obj.is_same_object(&child)) {
            childs.remove(pos);
        }
    }

    /// Reads a Frame object off the RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>, frames: &[Rc<Frame>]) -> Result<Rc<Frame>> {
        let matrix = try!(Matrix::read(rws));
        let parent_id = try!(rws.read_i32::<LittleEndian>());
        let _flags = try!(rws.read_u32::<LittleEndian>());  // ignored

        let frame = Rc::new(Frame {
            parent: RefCell::new(None),
            childs: RefCell::new(Vec::new()),
            matrix: matrix,
            name: RefCell::new(String::new()),
        });

        if parent_id >= 0 {
            let parent = frames.get(parent_id as usize).map(|rc| rc.clone());
            frame.set_frame(parent);
        }

        Ok(frame)
    }
}

impl FrameList {
    /// Gets the frame at the specified index or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<Rc<Frame>> {
        self.0.get(index).map(|rcframe| rcframe.clone())
    }

    /// Reads the Frame List off the RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<FrameList> {
        let _header = try!(Self::read_header(rws));

        let nframes = try!(Struct::read_up(rws, |rws| Ok(try!(rws.read_u32::<LittleEndian>()))));

        // Read frame by frame as usual...
        let mut frames: Vec<Rc<Frame>> = Vec::with_capacity(nframes as usize);
        for _ in (0..nframes) {
            let frame = try!(Frame::read(rws, &frames));
            frames.push(frame);
        }

        // We need to assign the Node Name Plugin afterwards...
        for i in (0..nframes as usize) {
            match try!(Extension::read_for(rws, |rws| NodeNamePlg::read(rws))) {
                Some(NodeNamePlg(name)) => *frames[i].name.borrow_mut() = name,
                None => {},
            };
        }

        Ok(FrameList(frames))
    }
}

impl NodeNamePlg {
    /// Reads a Node Name Plugin off the RenderWare Stream.
    pub fn read<R: ReadExt>(rws: &mut Stream<R>) -> Result<NodeNamePlg> {
        let header = try!(Self::read_header(rws));
        let bytes = try!(rws.read_bytes(header.size as usize));
        Ok(NodeNamePlg(String::from_utf8_lossy(&bytes).into_owned()))
    }
}
