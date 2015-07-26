use glium::Surface;
use std::rc::Rc;
use super::{NativeTexture};

pub struct Renderer<T> where T: Surface {
    pub target: T,
    pub tex_blank: Rc<NativeTexture>,
}

impl<T> Renderer<T> where T: Surface {
    pub fn new(target: T, tex_blank: Rc<NativeTexture>) -> Renderer<T> {
        Renderer {
            target: target,
            tex_blank: tex_blank,
        }
    }

    pub fn into_surface(self) -> T {
        self.target
    }
}
