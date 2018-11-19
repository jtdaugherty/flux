
use nalgebra::{Vector3};
use rand::Rng;

use samplers::{Sampler, to_poisson_disc, to_hemisphere};

pub struct MasterSampleSets {
    image_width: usize,
    pub pixel_sets: Vec<Vec<samplers::UnitSquareSample>>,
    pub disc_sets: Vec<Vec<samplers::UnitDiscSample>>,
    pub hemi_sets: Vec<Vec<Vec<Vector3<f64>>>>,
}

impl MasterSampleSets {
    pub fn new(sampler: &mut Sampler, sample_root: usize,
               max_depth: usize, width: usize) -> MasterSampleSets {
        MasterSampleSets {
            pixel_sets: (0..width).map(|_|
                sampler.grid_jittered(sample_root)).collect(),

            disc_sets: (0..width).map(|_|
                to_poisson_disc(
                    sampler.grid_jittered(sample_root))).collect(),

            hemi_sets: (0..width).map(|_|
                (0..max_depth).map(|_|
                    to_hemisphere(
                        sampler.grid_jittered(sample_root),
                        0.0)
                    ).collect()
                ).collect(),

            image_width: width,
        }
    }

    pub fn shuffle_indices(&self) -> Vec<usize> {
        let mut sample_set_indexes: Vec<usize> = (0..self.image_width).collect();
        let mut sampler = samplers::Sampler::new();
        sampler.rng.shuffle(&mut sample_set_indexes);
        sample_set_indexes
    }
}
