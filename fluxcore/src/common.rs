
use std::cmp::Ordering;
use nalgebra::{Vector3, Point3};
use materials::Material;

pub struct Hit<'a> {
    pub local_hit_point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub material: &'a Material,
    pub distance: f64,
    pub ray: Ray,
    pub depth: usize,
}

impl<'a> Hit<'a> {
    pub fn compare(&self, other: &Hit<'a>) -> Ordering {
        if self.distance.le(&other.distance) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

#[derive(Clone)]
pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Vector3<f64>,
}

pub trait Intersectable: Sync + Send {
    fn hit<'a>(&'a self, r: &Ray, depth: usize) -> Option<Hit<'a>>;
}
