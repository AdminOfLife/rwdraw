use super::{NativeGeometry, NativeDictionaryList};
use cgmath::Matrix4;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use rw;
use glium::backend::Facade;
use glium;
use glium::Surface;
use super::Renderer;

// TODO make checks to confirm that our hierarchy is getting the same as the one in the rw::Clump

#[derive(Debug)]
pub enum NodeObject {
    Atomic(Rc<RefCell<Atomic>>),
    Node(Rc<RefCell<Node>>),
}

#[derive(Debug)]
pub struct Node {
    pub parent: Option<Weak<RefCell<Node>>>,
    pub childs: Vec<NodeObject>,

    pub name: String,
    pub matrix: Matrix4<f32>,
    pub world_matrix: Matrix4<f32>,
}

#[derive(Debug)]
pub struct Atomic {
    pub parent: Option<Weak<RefCell<Node>>>,
    pub geometry: Rc<NativeGeometry>,
    // TODO atomic flags
    // TODO program?
}

#[derive(Debug)]
pub struct Clump {
    root: Rc<RefCell<Node>>,
}

impl Clump {
    pub fn from_rw<F: Facade>(facade: &F, clump: &rw::Clump, dicts: &NativeDictionaryList) -> Option<Clump> {
        let root = clump.atomic_root().unwrap(); // TODO
        match Node::from_rw(facade, &root, None, dicts) {
            Some(node) => Some(Clump { root: node }),
            None => None,
        }
    }

    pub fn render<S: Surface>(&self, renderer: &mut Renderer<S>, program: &glium::Program,
                                  proj: &Matrix4<f32>, model_view: &Matrix4<f32>) {

        self.root.borrow().render(renderer, program, proj, model_view)
    }

}

impl Node {
    pub fn render<S: Surface>(&self, renderer: &mut Renderer<S>, program: &glium::Program,
                                  proj: &Matrix4<f32>, model_view: &Matrix4<f32>) {

        for child in self.childs.iter() {
            match *child {
                NodeObject::Atomic(ref rcatomic) => {
                    rcatomic.borrow().render(renderer, program, proj, model_view)
                },
                NodeObject::Node(ref rcnode) => {
                    rcnode.borrow().render(renderer, program, proj, model_view)
                },
            }
        }
    }

    pub fn matrix(myself: &Rc<RefCell<Node>>) -> Matrix4<f32> {
        myself.borrow().matrix
    }

    pub fn from_rw<F: Facade>(facade: &F, frame: &rw::Frame,
                   parent: Option<Rc<RefCell<Node>>>, dicts: &NativeDictionaryList) -> Option<Rc<RefCell<Node>>> {

        let matrix = Matrix4::<f32>::from(frame.matrix());

        let mut node = Rc::new(RefCell::new(Node {
            name: frame.name(),
            matrix: matrix,
            world_matrix: parent.clone().map(|rc| Node::matrix(&rc) * matrix).unwrap_or(matrix),

            parent: parent.clone().map(|rc| rc.downgrade()),
            childs: Vec::with_capacity(frame.num_childs()),
        }));


        {
            let childs = frame.childs();
            for frameobj in childs.iter() {
                match NodeObject::from_rw(facade, frameobj, Some(node.clone()), dicts) {
                    Some(nodeobj) => node.borrow_mut().childs.push(nodeobj),
                    None => return None,
                }
            }
        }

        Some(node)
    }
}

impl NodeObject {
    pub fn from_rw<F: Facade>(facade: &F, frameobj: &rw::FrameObjectValue,
                   parent: Option<Rc<RefCell<Node>>>, dicts: &NativeDictionaryList) -> Option<NodeObject> {
        match *frameobj {
            rw::FrameObjectValue::Frame(ref rcframe) => {
                Node::from_rw(facade, &*rcframe, parent, dicts).map(|rcnode| {
                    NodeObject::Node(rcnode)
                })
            },
            rw::FrameObjectValue::Atomic(ref rcatomic) => {
                Atomic::from_rw(facade, &*rcatomic, parent, dicts).map(|atomic| {
                    NodeObject::Atomic( Rc::new(RefCell::new(atomic)) )
                })
            },
            rw::FrameObjectValue::Clump(_) => {
                unreachable!()
            },
        }
    }
}

impl Atomic {
    pub fn render<S: Surface>(&self, renderer: &mut Renderer<S>, program: &glium::Program,
                                  proj: &Matrix4<f32>, model_view: &Matrix4<f32>) {

        // TODO REMOVE THIS CHECK
        match self.parent.clone().and_then(|weak| weak.upgrade())
        {
            Some(parent) => {
                if parent.borrow().name.ends_with("vlo")
                || parent.borrow().name.ends_with("dam") {
                    return;
                }
            },
            None => (),
        }

        let model_view2 = self.parent.clone()
                                     .and_then(|weak| weak.upgrade())
                                     .map(|parent| *model_view * parent.borrow().world_matrix)
                                     .unwrap_or(*model_view);
        self.geometry.render(renderer, program, proj, &model_view2)
    }

    pub fn from_rw<F: Facade>(facade: &F, atomic: &rw::Atomic,
                   parent: Option<Rc<RefCell<Node>>>, dicts: &NativeDictionaryList) -> Option<Atomic> {
        Some(Atomic {
            parent: parent.map(|rc| rc.downgrade()),
            geometry: match NativeGeometry::from_rw(facade, &atomic.geometry, dicts) {
                Some(geometry) => Rc::new(geometry),
                None => return None,
            },
        })
    }
}