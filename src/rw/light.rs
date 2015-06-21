use super::Section;

#[derive(Debug)]
pub struct Light;

impl Light {

}

impl Section for Light {
    fn section_id() -> u32 { 0x0012 }
}
