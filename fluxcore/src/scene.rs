
use color::Color;

// SceneData can contain only data, not heap references to trait
// objects, etc. The idea is that when we're ready to start rendering a
// scene, we'll build a Scene from a SceneData.
#[derive(Clone)]
#[derive(Copy)]
pub struct SceneData {
    pub output_settings: OutputSettings,
    pub background: Color,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct OutputSettings {
    pub image_width: usize,
    pub image_height: usize,
    pub pixel_size: f64,
}

pub struct Scene {
    pub output_settings: OutputSettings,
    pub background: Color,
}

pub fn scene_from_data(sd: SceneData) -> Scene {
    Scene {
        output_settings: sd.output_settings,
        background: sd.background,
    }
}
