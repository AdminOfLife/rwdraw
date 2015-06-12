use std::ops::Neg;
use std::collections::HashMap;
use glium::glutin::{self, Event, ElementState, VirtualKeyCode, MouseButton};
use cgmath::{Point, Vector, Vector2, Vector3, Point3, Matrix4};



pub struct UserControl {
    keyboard: HashMap<glutin::VirtualKeyCode, (bool, glutin::ElementState)>,
    mouse: HashMap<glutin::MouseButton, (bool, glutin::ElementState)>,
    pub wheel_motion: (f32, f32),
    pub mouse_motion: (f32, f32),
    pub mouse_pos: (i32, i32),
}

pub struct UserCamera {
    angle: Vector2<f32>,    // (horizontal, vertical) angles
    position: Point3<f32>,
}

impl UserCamera {
    pub fn new() -> UserCamera {
        UserCamera {
            angle:  Vector2::new(-2.190002f32, -0.52999985f32),
            position: Point3::new(4.0, 3.0, 3.0),
        }
    }

    pub fn process_view_matrix(&mut self, user: &UserControl, delta_time: f32) -> Matrix4<f32> {

        // TODO optimize cos sin calls here
        let direction = Vector3::new(
            self.angle.y.cos() * self.angle.x.sin(), 
            self.angle.y.sin(),
            self.angle.y.cos() * self.angle.x.cos()
        );

        let right = Vector3::new(
            (self.angle.x - 3.14 / 2.0).sin(), 
            0.0,
            (self.angle.x - 3.14 / 2.0).cos()
        );

        let up = right.cross(&direction);

        // TODO P+V and V*S in cgmath
        if user.is_pressed(VirtualKeyCode::W) {
            self.position = self.position.add_v(&direction.mul_s(delta_time * 10.0));
        }
        if user.is_pressed(VirtualKeyCode::S) {
            self.position = self.position.add_v(&direction.mul_s(delta_time * 10.0).neg());
        }
        if user.is_pressed(VirtualKeyCode::D) {
            self.position = self.position.add_v(&right.mul_s(delta_time * 10.0));
        }
        if user.is_pressed(VirtualKeyCode::A) {
            self.position = self.position.add_v(&right.mul_s(delta_time * 10.0).neg());
        }
        if user.is_mouse_pressed(MouseButton::Left) {
            let mouse_speed = 0.005f32;
            self.angle.x = self.angle.x + mouse_speed * user.mouse_motion.0;
            self.angle.y = self.angle.y + mouse_speed * user.mouse_motion.1;
        }

        // TODO add P+V in cgmath
       Matrix4::look_at(&self.position, 
                        &(self.position.add_v(&direction)),
                        &up)
    }
}

impl UserControl {
    pub fn new(window: Option<&glutin::Window>, res: (i32, i32)) -> UserControl {
        let mouse_pos = (res.0 / 2, res.1 / 2);
        window.and_then(|w| w.set_cursor_position(mouse_pos.0, mouse_pos.1).ok());
        UserControl {
            keyboard: HashMap::with_capacity(256),
            mouse: HashMap::with_capacity(8),
            wheel_motion: (0.0, 0.0),
            mouse_motion: (0.0, 0.0),
            mouse_pos: mouse_pos,
        }
    }

    // None should be sent once per frame before processing any event generated there
    // this is to clean up old information
    pub fn process(&mut self, event: Option<glutin::Event>) -> &mut UserControl {
        use itertools::Itertools;
        match event {
            None => {
                // Remove just changed flag
                self.keyboard.iter_mut().foreach(|(_, v)| *v = (false, v.1));
                self.mouse.iter_mut().foreach(|(_, v)| *v = (false, v.1));
                // Reset motioness
                self.wheel_motion = (0.0, 0.0);
                self.mouse_motion = (0.0, 0.0);
            },
            Some(Event::MouseMoved((x, y))) => {
                self.mouse_motion = ((self.mouse_pos.0 - x) as f32, (self.mouse_pos.1 - y) as f32);
                self.mouse_pos = (x, y);
            },
            Some(Event::KeyboardInput(state, _, Some(vkey))) => {
                let just_changed = self.keyboard.remove(&vkey)
                                                .map(|(_, oldstate)| state == oldstate)
                                                .unwrap_or(true);
                self.keyboard.insert(vkey, (just_changed, state));
            },
            Some(Event::MouseInput(state, button)) => {
                let just_changed = self.mouse.remove(&button)
                                              .map(|(_, oldstate)| state == oldstate)
                                              .unwrap_or(true);
                self.mouse.insert(button, (just_changed, state));
            },
            _ => {},
        };
        self
    }

    pub fn is_pressed(&self, vkey: glutin::VirtualKeyCode) -> bool {
        match self.keyboard.get(&vkey) {
            Some(&(_, ElementState::Pressed)) => true,
            _ => false,
        }
    }

    pub fn is_mouse_pressed(&self, button: glutin::MouseButton) -> bool {
        match self.mouse.get(&button) {
            Some(&(_, ElementState::Pressed)) => true,
            _ => false,
        }
    }
}
