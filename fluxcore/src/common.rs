
use nalgebra::{Vector3, Point3};
use color::Color;

pub struct Hit {
    pub local_hit_point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub color: Color,
    pub tmin: f64,
}

pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Vector3<f64>,
}

pub trait Intersectable {
    fn hit(&self, r: &Ray) -> Option<Hit>;
}
