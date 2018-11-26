
use nalgebra::{Vector3, Point3};
use constants::*;
use common::*;
use materials::*;
use color::Color;

pub struct Sphere {
    pub data: SphereData,
    pub material: Box<Material>,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct SphereData {
    pub center: Point3<f64>,
    pub radius: f64,
    pub material: MaterialData,
}

pub struct Plane {
    pub data: PlaneData,
    pub material: Box<Material>,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct PlaneData {
    pub point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub material: MaterialData,
}

#[derive(Copy)]
pub struct MaterialData {
    pub material_type: MaterialType,
    pub content: MaterialContent,
}

#[derive(Clone)]
#[derive(Copy)]
pub enum MaterialType {
    Matte,
}

#[derive(Copy)]
#[derive(Clone)]
pub union MaterialContent {
    pub matte: MatteData,
}

impl Clone for MaterialData {
    fn clone(&self) -> MaterialData {
        let content = match self.material_type {
            MaterialType::Matte => unsafe { MaterialContent { matte: self.content.matte.clone() } },
        };

        MaterialData {
            material_type: self.material_type,
            content,
        }
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct MatteData {
    pub diffuse_color: Color,
    pub ambient_color: Color,
    pub diffuse_coefficient: f64,
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

impl Intersectable for Sphere {
    fn hit<'a>(&'a self, r: &Ray, depth: usize) -> Option<Hit<'a>> {
        let temp = r.origin - self.data.center;
        let a = r.direction.dot(&r.direction);
        let b = 2.0 * temp.dot(&r.direction);
        let c = temp.dot(&temp) - self.data.radius * self.data.radius;
        let disc = b * b - 4.0 * a * c;

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
                    normal: (temp + t * r.direction) / self.data.radius,
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
                        normal: (temp + t2 * r.direction) / self.data.radius,
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
