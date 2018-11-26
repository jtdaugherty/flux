
use nalgebra::{Vector3};

use common::Hit;
use color::Color;
use constants::INV_PI;

pub trait BRDF {
    fn sample_f(&self, hit: &Hit, wo: &Vector3<f64>, hemi_sample: &Vector3<f64>) -> (Vector3<f64>, f64, Color);
}

pub struct Lambertian {
    pub diffuse_coefficient: f64,
    pub diffuse_color: Color,
}

impl BRDF for Lambertian {
    fn sample_f(&self, hit: &Hit, _wo: &Vector3<f64>, hemi_sample: &Vector3<f64>) -> (Vector3<f64>, f64, Color) {
        let w = hit.normal;
        let v = Vector3::new(0.0034, 1.0, 0.0071).cross(&w).normalize();
        let u = v.cross(&w);

        let wi = (hemi_sample.x * u + hemi_sample.y * v + hemi_sample.z * w).normalize();
        let pdf = hit.normal.dot(&wi) * INV_PI;

        (wi, pdf, self.diffuse_color * self.diffuse_coefficient * INV_PI)
    }
}
