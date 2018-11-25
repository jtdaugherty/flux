
use std::cmp::Ordering;
use nalgebra::{Vector3, Point3};

use color::Color;

pub struct Hit {
    pub local_hit_point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub color: Color,
    pub distance: f64,
}

impl Hit {
    pub fn compare(&self, other: &Hit) -> Ordering {
        if self.distance.le(&other.distance) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Vector3<f64>,
}

pub trait Intersectable: Sync + Send {
    fn hit(&self, r: &Ray) -> Option<Hit>;
}
