
// SceneData can contain only data, not heap references to trait
// objects, etc. The idea is that when we're ready to start rendering a
// scene, we'll build a Scene from a SceneData.
#[derive(Clone)]
#[derive(Copy)]
pub struct SceneData {
    pub image_width: usize,
    pub image_height: usize,
}

pub struct Scene {
}
