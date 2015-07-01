use super::Section;

// STUB! TODO

/// Lights are used to illuminate atomics and worlds.
///
/// This object represents dynamic lighting in a RenderWare Graphics Retained Mode scene.
/// Lighting models available are: 
///   * Ambient
///   * Directional
///   * Point
///   * Spotlight
///   * Soft Spotlight
///
#[derive(Debug)]
pub struct Light;

impl Light {

}

impl Section for Light {
    fn section_id() -> u32 { 0x0012 }
}
