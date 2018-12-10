
use nalgebra::{Vector3, Point3};

use color::Color;
use common::{Ray, Intersectable, Hit};
use shapes::*;
use job::JobConfiguration;
use materials::*;
use brdf::*;
use sampling::MasterSampleSets;

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

#[derive(Copy)]
#[derive(Clone)]
pub enum ShapeData {
    Sphere(SphereData),
    Plane(PlaneData),
}

pub struct Scene {
    pub scene_name: String,
    pub output_settings: OutputSettings,
    pub background: Color,
    pub shapes: Vec<Box<Intersectable>>,
    pub camera_settings: CameraSettings,
    pub camera_data: CameraData,
    pub job_config: JobConfiguration,
}

pub fn material_from_data(d: &MaterialData) -> Box<Material> {
    match d {
        MaterialData::Emissive(e) => {
            Box::new(Emissive {
                color: e.color,
                power: e.power,
            })
        },
        MaterialData::Reflective(p) => {
            Box::new(Reflective {
                reflective_brdf: Box::new(PerfectSpecular {
                    kr: p.reflect_amount,
                    cr: p.reflect_color,
                }),
            })
        },
        MaterialData::GlossyReflective(p) => {
            Box::new(Reflective {
                reflective_brdf: Box::new(GlossySpecular {
                    ks: p.reflect_amount,
                    cs: p.reflect_color,
                    exp: p.reflect_exponent,
                }),
            })
        },
        MaterialData::Matte(m) => {
            Box::new(Matte {
                ambient_brdf: Lambertian {
                    diffuse_coefficient: m.diffuse_coefficient,
                    diffuse_color: m.ambient_color,
                },
                diffuse_brdf: Lambertian {
                    diffuse_coefficient: m.diffuse_coefficient,
                    diffuse_color: m.diffuse_color,
                }
            })
        },
    }
}

impl Scene {
    pub fn from_data(sd: SceneData, config: JobConfiguration) -> Scene {
        let shapes: Vec<Box<Intersectable>> = sd.shapes.into_iter().map(|sd| {
            match sd {
                ShapeData::Sphere(s) => {
                    let m = material_from_data(&s.material);
                    let b: Box<Intersectable> = Box::new(Sphere::new(s, m));
                    b
                },
                ShapeData::Plane(p) => {
                    let m = material_from_data(&p.material);
                    let b: Box<Intersectable> = Box::new(Plane { data: p, material: m });
                    b
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
            job_config: config,
        }
    }

    fn hit(&self, r: Ray, depth: usize) -> Option<Hit> {
        self.shapes.iter()
            .filter_map(|o| o.hit(&r, depth))
            .min_by(Hit::compare)
    }

    pub fn shade(&self, r: Ray, depth: usize, samples: &MasterSampleSets,
                 set_index: usize, sample_index: usize) -> Color {
        if depth > self.job_config.max_trace_depth {
            Color::black()
        } else {
            match self.hit(r, depth) {
                None => self.background,
                Some(h) => h.material.path_shade(&self, &h, &samples, set_index, sample_index),
            }
        }
    }
}
