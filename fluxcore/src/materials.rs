
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
