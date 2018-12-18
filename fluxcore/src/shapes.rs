
use nalgebra::{Vector3, Point3};

use crate::constants::*;
use crate::common::*;
use crate::materials::*;
use crate::color::Color;

pub struct Sphere {
    pub data: SphereData,
    pub material: Box<Material>,
    pub bbox: BoundingBox,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct SphereData {
    pub center: Point3<f64>,
    pub radius: f64,
    pub material: MaterialData,
    pub invert: bool,
}

pub struct Plane {
    pub data: PlaneData,
    pub material: Box<Material>,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct PlaneData {
    pub point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub material: MaterialData,
}

#[derive(Copy)]
#[derive(Clone)]
#[derive(Serialize, Deserialize, Debug)]
pub enum MaterialData {
    Matte(MatteData),
    Emissive(EmissiveData),
    Reflective(ReflectiveData),
    GlossyReflective(GlossyReflectiveData),
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct MatteData {
    pub diffuse_color: Color,
    pub ambient_color: Color,
    pub diffuse_coefficient: f64,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct EmissiveData {
    pub color: Color,
    pub power: f64,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct ReflectiveData {
    pub reflect_amount: f64,
    pub reflect_color: Color,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct GlossyReflectiveData {
    pub reflect_amount: f64,
    pub reflect_color: Color,
    pub reflect_exponent: f64,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct BoundingBox {
    pub corner0: Point3<f64>,
    pub corner1: Point3<f64>,
}

fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

impl BoundingBox {
    fn hit<'a>(&'a self, r: &Ray) -> bool {
        let ox = r.origin.x;
        let oy = r.origin.y;
        let oz = r.origin.z;
        let dx = r.direction.x;
        let dy = r.direction.y;
        let dz = r.direction.z;

        let a = 1.0 / dx;
        let (tx_min, tx_max) = if a >= 0.0 {
            ((self.corner0.x - ox) * a, (self.corner1.x - ox) * a)
        } else {
            ((self.corner1.x - ox) * a, (self.corner0.x - ox) * a)
        };

        let b = 1.0 / dy;
        let (ty_min, ty_max) = if b >= 0.0 {
            ((self.corner0.y - oy) * b, (self.corner1.y - oy) * b)
        } else {
            ((self.corner1.y - oy) * b, (self.corner0.y - oy) * b)
        };

        let c = 1.0 / dz;
        let (tz_min, tz_max) = if c >= 0.0 {
            ((self.corner0.z - oz) * c, (self.corner1.z - oz) * c)
        } else {
            ((self.corner1.z - oz) * c, (self.corner0.z - oz) * c)
        };

        let t0 = max(tx_min, max(ty_min, tz_min));
        let t1 = min(tx_max, min(ty_max, tz_max));

        (t0 < t1 && t1 > T_MIN)
    }
}

impl Intersectable for Plane {
    fn hit<'a>(&'a self, r: &Ray, depth: usize) -> Option<Hit<'a>> {
        let t = (self.data.point - r.origin).dot(&self.data.normal) / (r.direction.dot(&self.data.normal));

        if t > T_MIN {
            Some(Hit {
                ray: r.clone(),
                depth,
                distance: t,
                normal: self.data.normal,
                local_hit_point: r.origin + t * r.direction,
                material: self.material.as_ref(),
            })
        } else {
            None
        }
    }
}

impl Sphere {
    pub fn new(data: SphereData, material: Box<Material>) -> Self {
        let delta = Vector3::new(data.radius, data.radius, data.radius);
        let corner0 = data.center - delta;
        let corner1 = data.center + delta;
        let bbox = BoundingBox {
            corner0, corner1,
        };

        Self {
            data,
            material,
            bbox,
        }
    }
}

impl Intersectable for Sphere {
    fn hit<'a>(&'a self, r: &Ray, depth: usize) -> Option<Hit<'a>> {
        if !self.bbox.hit(&r) {
            None
        } else {
            let temp = r.origin - self.data.center;
            let a = r.direction.dot(&r.direction);
            let b = 2.0 * temp.dot(&r.direction);
            let c = temp.dot(&temp) - self.data.radius * self.data.radius;
            let disc = b * b - 4.0 * a * c;
            let invert_val = if self.data.invert { -1.0 } else { 1.0 };

            if disc < 0.0 {
                None
            } else {
                let e = disc.sqrt();
                let denom = 2.0 * a;
                let t = (-b - e) / denom;

                if t > T_MIN {
                    Some(Hit {
                        ray: r.clone(),
                        distance: t,
                        depth,
                        normal: (temp + t * r.direction) * invert_val / self.data.radius,
                        local_hit_point: r.origin + t * r.direction,
                        material: self.material.as_ref(),
                    })
                } else {
                    let t2 = (-b + e) / denom;
                    if t2 > T_MIN {
                        Some(Hit {
                            ray: r.clone(),
                            distance: t2,
                            depth,
                            normal: (temp + t2 * r.direction) * invert_val / self.data.radius,
                            local_hit_point: r.origin + t2 * r.direction,
                            material: self.material.as_ref(),
                        })
                    } else {
                        None
                    }
                }
            }
        }
    }
}
