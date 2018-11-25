
use nalgebra::{Vector3};
use scene::{Sphere, Plane};
use constants::*;
use common::*;

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

pub struct CameraSettings {
    pub eye: Vector3<f64>,
    pub look_at: Vector3<f64>,
    pub up: Vector3<f64>,
    pub u: Vector3<f64>,
    pub v: Vector3<f64>,
    pub w: Vector3<f64>,
}

impl CameraSettings {
    pub fn new(eye: Vector3<f64>, look_at: Vector3<f64>, up: Vector3<f64>) -> CameraSettings {
        let w = (eye - look_at).normalize();
        let u = up.cross(&w).normalize();
        let v = w.cross(&u);
        CameraSettings { eye, look_at, up, u, v, w }
    }
}
