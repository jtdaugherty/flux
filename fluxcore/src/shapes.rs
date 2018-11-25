
use nalgebra::{Vector3, Point3};
use constants::*;
use common::*;
use color::Color;

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

impl Intersectable for Plane {
    fn hit(&self, r: &Ray) -> Option<Hit> {
        let t = (self.point - r.origin).dot(&self.normal) / (r.direction.dot(&self.normal));

        if t > T_MIN {
            Some(Hit {
                tmin: t,
                normal: self.normal,
                local_hit_point: r.origin + t * r.direction,
                color: self.color,
            })
        } else {
            None
        }
    }
}

impl Intersectable for Sphere {
    fn hit(&self, r: &Ray) -> Option<Hit> {
        let temp = r.origin - self.center;
        let a = r.direction.dot(&r.direction);
        let b = 2.0 * temp.dot(&r.direction);
        let c = temp.dot(&temp) - self.radius * self.radius;
        let disc = b * b - 4.0 * a * c;

        if disc < 0.0 {
            None
        } else {
            let e = disc.sqrt();
            let denom = 2.0 * a;
            let t = (-b - e) / denom;

            if t > T_MIN {
                Some(Hit {
                    tmin: t,
                    normal: (temp + t * r.direction) / self.radius,
                    local_hit_point: r.origin + t * r.direction,
                    color: self.color,
                })
            } else {
                let t2 = (-b + e) / denom;
                if t2 > T_MIN {
                    Some(Hit {
                        tmin: t2,
                        normal: (temp + t2 * r.direction) / self.radius,
                        local_hit_point: r.origin + t2 * r.direction,
                        color: self.color,
                    })
                } else {
                    None
                }
            }
        }
    }
}
