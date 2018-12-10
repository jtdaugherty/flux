
use brdf::*;
use common::*;
use color::Color;
use scene::Scene;
use sampling::MasterSampleSets;

pub trait Material: Sync + Send {
    fn path_shade(&self, scene: &Scene, hit: &Hit, samples: &MasterSampleSets,
                  set_index: usize, sample_index: usize) -> Color;
}

pub struct Matte {
    pub ambient_brdf: Lambertian,
    pub diffuse_brdf: Lambertian,
}

impl Material for Matte {
    fn path_shade(&self, scene: &Scene, hit: &Hit, samples: &MasterSampleSets,
                  set_index: usize, sample_index: usize) -> Color {
        let wo = -1.0 * hit.ray.direction;
        let hemi_sample = &samples.hemi_sets[set_index][hit.depth - 1][sample_index];
        let (wi, pdf, f) = self.diffuse_brdf.sample_f(hit, &wo, &hemi_sample);
        let ndotwi = hit.normal.dot(&wi);
        let reflected_ray = Ray {
            origin: hit.local_hit_point,
            direction: wi,
        };

        f * scene.shade(reflected_ray, hit.depth + 1, &samples, set_index, sample_index) *
            (ndotwi / pdf)
    }
}

pub struct Emissive {
    pub color: Color,
    pub power: f64,
}

impl Material for Emissive {
    fn path_shade(&self, scene: &Scene, hit: &Hit, samples: &MasterSampleSets,
                  set_index: usize, sample_index: usize) -> Color {
        if (hit.normal * -1.0).dot(&hit.ray.direction) > 0.0 {
            self.color * self.power
        } else {
            Color::black()
        }
    }
}

pub struct Reflective {
    pub reflective_brdf: PerfectSpecular,
}

impl Material for Reflective {
    fn path_shade(&self, scene: &Scene, hit: &Hit, samples: &MasterSampleSets,
                  set_index: usize, sample_index: usize) -> Color {
        let wo = hit.ray.direction * -1.0;
        let hemi_sample = &samples.hemi_sets[set_index][hit.depth - 1][sample_index];
        let (wi, pdf, fr) = self.reflective_brdf.sample_f(hit, &wo, &hemi_sample);

        let reflected_ray = Ray {
            origin: hit.local_hit_point,
            direction: wi,
        };

        fr * scene.shade(reflected_ray, hit.depth + 1, &samples, set_index, sample_index) *
            (hit.normal.dot(&wi) / pdf)
    }
}
