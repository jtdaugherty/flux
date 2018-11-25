
use nalgebra::{Vector3, Point3};

use color::Color;
use common::{Ray, Intersectable, Hit};
use shapes::*;

#[derive(Clone)]
pub struct CameraSettings {
    pub eye: Point3<f64>,
    pub look_at: Point3<f64>,
    pub up: Vector3<f64>,
    pub u: Vector3<f64>,
    pub v: Vector3<f64>,
    pub w: Vector3<f64>,
}

impl CameraSettings {
    pub fn new(eye: Point3<f64>, look_at: Point3<f64>, up: Vector3<f64>) -> CameraSettings {
        let w = (eye - look_at).normalize();
        let u = up.cross(&w).normalize();
        let v = w.cross(&u);
        CameraSettings { eye, look_at, up, u, v, w }
    }
}

// SceneData can contain only data, not heap references to trait
// objects, etc. The idea is that when we're ready to start rendering a
// scene, we'll build a Scene from a SceneData.
#[derive(Clone)]
pub struct SceneData {
    pub scene_name: String,
    pub output_settings: OutputSettings,
    pub background: Color,
    pub shapes: Vec<ShapeData>,
    pub camera_settings: CameraSettings,
    pub camera_data: CameraData,
}

#[derive(Clone)]
pub struct CameraData {
    pub zoom_factor: f64,
    pub view_plane_distance: f64,
    pub focal_distance: f64,
    pub lens_radius: f64,
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
    pub scene_name: String,
    pub output_settings: OutputSettings,
    pub background: Color,
    pub shapes: Vec<Box<Intersectable>>,
    pub camera_settings: CameraSettings,
    pub camera_data: CameraData,
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
            scene_name: sd.scene_name,
            shapes,
            camera_settings: sd.camera_settings,
            camera_data: sd.camera_data,
        }
    }

    fn hit(&self, r: Ray) -> Option<Hit> {
        self.shapes.iter()
            .filter_map(|o| o.hit(&r))
            .min_by(Hit::compare)
    }

    pub fn shade(&self, r: Ray) -> Color {
        match self.hit(r) {
            None => self.background,
            Some(h) => h.color,
        }
    }
}
