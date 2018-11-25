
use nalgebra::{Vector3, Point3};
use color::Color;

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

#[derive(Clone)]
#[derive(Copy)]
pub struct Sphere {
    pub center: Point3<f64>,
    pub radius: f64,
    pub color: Color,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct Plane {
    pub point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub color: Color,
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
