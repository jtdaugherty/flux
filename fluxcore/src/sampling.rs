
use nalgebra::{Vector3};
use rand::Rng;

pub struct MasterSampleSets {
    num_sets: usize,
    pub pixel_sets: Vec<Vec<samplers::UnitSquareSample>>,
    pub disc_sets: Vec<Vec<samplers::UnitDiscSample>>,
    pub hemi_sets: Vec<Vec<Vec<Vector3<f64>>>>,
}

impl MasterSampleSets {
    pub fn new(sampler: &mut samplers::Sampler, sample_root: usize,
               max_depth: usize, num_sets: usize) -> MasterSampleSets {
        MasterSampleSets {
            pixel_sets: (0..num_sets).map(|_|
                sampler.grid_jittered(sample_root)).collect(),

            disc_sets: (0..num_sets).map(|_|
                samplers::to_poisson_disc(
                    sampler.grid_jittered(sample_root))).collect(),

            hemi_sets: (0..num_sets).map(|_|
                (0..max_depth).map(|_|
                    samplers::to_hemisphere(
                        sampler.grid_jittered(sample_root),
                        0.0)
                    ).collect()
                ).collect(),

            num_sets,
        }
    }

    pub fn shuffle_indices(&self) -> Vec<usize> {
        let mut sample_set_indexes: Vec<usize> = (0..self.num_sets).collect();
        let mut sampler = samplers::Sampler::new();
        sampler.rng.shuffle(&mut sample_set_indexes);
        sample_set_indexes
    }
}
