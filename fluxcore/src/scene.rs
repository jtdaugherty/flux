
use color::Color;
use common::Intersectable;
use shapes::*;

// SceneData can contain only data, not heap references to trait
// objects, etc. The idea is that when we're ready to start rendering a
// scene, we'll build a Scene from a SceneData.
#[derive(Clone)]
pub struct SceneData {
    pub output_settings: OutputSettings,
    pub background: Color,
    pub shapes: Vec<ShapeData>,
}

#[derive(Clone)]
pub struct OutputSettings {
    pub image_width: usize,
    pub image_height: usize,
    pub pixel_size: f64,
}

#[derive(Clone)]
#[derive(Copy)]
pub enum ShapeType {
    Sphere,
    Plane,
}

pub struct ShapeData {
    pub shape_type: ShapeType,
    pub content: ShapeContent,
}

impl Clone for ShapeData {
    fn clone(&self) -> ShapeData {
        let content = match self.shape_type {
            ShapeType::Sphere => unsafe { ShapeContent { sphere: self.content.sphere.clone() } },
            ShapeType::Plane => unsafe { ShapeContent { plane: self.content.plane.clone() } },
        };

        ShapeData {
            shape_type: self.shape_type,
            content,
        }
    }
}

pub union ShapeContent {
    pub sphere: Sphere,
    pub plane: Plane,
}

pub struct Scene {
    pub output_settings: OutputSettings,
    pub background: Color,
    pub shapes: Vec<Box<Intersectable>>,
}

impl Scene {
    pub fn from_data(sd: SceneData) -> Scene {
        let shapes: Vec<Box<Intersectable>> = sd.shapes.iter().map(|sd| {
            match sd.shape_type {
                ShapeType::Sphere => {
                    unsafe {
                        let b: Box<Intersectable> = Box::new(sd.content.sphere);
                        b
                    }
                },
                ShapeType::Plane => {
                    unsafe {
                        let b: Box<Intersectable> = Box::new(sd.content.plane);
                        b
                    }
                },
            }
        }).collect();

        Scene {
            output_settings: sd.output_settings,
            background: sd.background,
            shapes,
        }
    }
}
