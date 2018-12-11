
use nalgebra::{Vector3};

use samplers::{to_unit_hemi, UnitSquareSample};
use common::Hit;
use color::Color;
use constants::INV_PI;

pub trait BRDF: Send + Sync {
    fn sample_f(&self, hit: &Hit, wo: &Vector3<f64>, hemi_sample: &Vector3<f64>, square_sample: &UnitSquareSample) -> (Vector3<f64>, f64, Color);
}

pub struct Lambertian {
    pub diffuse_coefficient: f64,
    pub diffuse_color: Color,
}

impl BRDF for Lambertian {
    fn sample_f(&self, hit: &Hit, _wo: &Vector3<f64>, hemi_sample: &Vector3<f64>, _square_sample: &UnitSquareSample) -> (Vector3<f64>, f64, Color) {
        let w = hit.normal;
        let v = Vector3::new(0.0034, 1.0, 0.0071).cross(&w).normalize();
        let u = v.cross(&w);

        let wi = (hemi_sample.x * u + hemi_sample.y * v + hemi_sample.z * w).normalize();
        let pdf = hit.normal.dot(&wi) * INV_PI;

        (wi, pdf, self.diffuse_color * self.diffuse_coefficient * INV_PI)
    }
}

pub struct PerfectSpecular {
    pub kr: f64,
    pub cr: Color,
}

impl BRDF for PerfectSpecular {
    fn sample_f(&self, hit: &Hit, wo: &Vector3<f64>, _hemi_sample: &Vector3<f64>, _square_sample: &UnitSquareSample) -> (Vector3<f64>, f64, Color) {
        let ndotwo = hit.normal.dot(&wo);
        let wi = -wo + hit.normal * ndotwo * 2.0;
        let pdf = hit.normal.dot(&wi);
        (wi, pdf, self.cr * self.kr)
    }
}

pub struct GlossySpecular {
    pub ks: f64,
    pub cs: Color,
    pub exp: f64,
}

impl BRDF for GlossySpecular {
    fn sample_f(&self, hit: &Hit, wo: &Vector3<f64>, _hemi_sample: &Vector3<f64>, pixel_sample: &UnitSquareSample) -> (Vector3<f64>, f64, Color) {
        let ndotwo = hit.normal.dot(&wo);
        let r = -wo + hit.normal * ndotwo * 2.0;

        let w = r;
        let u = Vector3::new(0.00424, 1.0, 0.00764).cross(&w).normalize();
        let v = u.cross(&w);

        let hemi_sample = to_unit_hemi(&pixel_sample, self.exp);
        let wi0 = u * hemi_sample.x + v * hemi_sample.y + w * hemi_sample.z;

        let wi = if hit.normal.dot(&wi0) < 0.0 {
            u * -hemi_sample.x - v * hemi_sample.y + w * hemi_sample.z
        } else {
            wi0
        };

        let phong_lobe = r.dot(&wi).powf(self.exp);
        let pdf = phong_lobe * hit.normal.dot(&wi);
        let color = self.cs * self.ks * phong_lobe;

        (wi, pdf, color)
    }
}
