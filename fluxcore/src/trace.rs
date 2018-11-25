
use nalgebra::{Vector3, Point3};
use sampling::MasterSampleSets;
use samplers::Sampler;
use color::Color;
use scene::Scene;
use common::Ray;
use manager::WorkUnitResult;
use job::{JobConfiguration, WorkUnit};
use rayon::prelude::*;

pub struct CameraSettings {
    pub eye: Point3<f64>,
    pub look_at: Point3<f64>,
    pub up: Vector3<f64>,
    pub u: Vector3<f64>,
    pub v: Vector3<f64>,
    pub w: Vector3<f64>,
}

impl CameraSettings {
    pub fn new(eye: Point3<f64>, look_at: Point3<f64>, up: Vector3<f64>) -> CameraSettings {
        let w = (eye - look_at).normalize();
        let u = up.cross(&w).normalize();
        let v = w.cross(&u);
        CameraSettings { eye, look_at, up, u, v, w }
    }
}

pub struct ThinLensCamera {
    pub settings: CameraSettings,
    samples: MasterSampleSets,
    config: JobConfiguration,
    pub zoom_factor: f64,
    pub view_plane_distance: f64,
    pub focal_distance: f64,
    pub lens_radius: f64,
}

impl ThinLensCamera {
    pub fn new(settings: CameraSettings, config: JobConfiguration, num_sets: usize,
               zoom_factor: f64, view_plane_distance: f64, focal_distance: f64,
               lens_radius: f64) -> ThinLensCamera {
        let mut s = Sampler::new();

        ThinLensCamera {
            settings,
            config,
            zoom_factor,
            view_plane_distance,
            focal_distance,
            lens_radius,
            samples: MasterSampleSets::new(&mut s, config.sample_root,
                                           config.max_trace_depth, num_sets),
        }
    }

    fn ray_direction(&self, px: f64, py: f64, lx: f64, ly: f64) -> Vector3<f64> {
        let factor = self.focal_distance / self.view_plane_distance;
        let px2 = px * factor;
        let py2 = py * factor;
        ((px2 - lx) * self.settings.u +
            (py2 - ly) * self.settings.v -
            self.focal_distance * self.settings.w).normalize()
    }

    pub fn render(&self, s: &Scene, work: WorkUnit) -> WorkUnitResult {
        let img_h = s.output_settings.image_height;
        let img_w = s.output_settings.image_width;
        let half_img_h = img_h as f64 * 0.5;
        let half_img_w = img_w as f64 * 0.5;

        let pixel_denom = 1.0 / ((self.config.sample_root * self.config.sample_root) as f64);
        let adjusted_pixel_size = s.output_settings.pixel_size / self.zoom_factor;

        let rows: Vec<usize> = (work.row_start..work.row_end+1).collect();
        let row_pixel_vecs: Vec<Vec<Color>> = rows.par_iter().map(|row| {
            let sample_set_indexes = self.samples.shuffle_indices();

            let row_pixels = (0..img_w).map(|col| {
                let mut color = Color::black();
                let pixel_samples = &self.samples.pixel_sets[sample_set_indexes[col] % self.samples.pixel_sets.len()];
                let disc_samples = &self.samples.disc_sets[sample_set_indexes[col] % self.samples.disc_sets.len()];

                for (index, point) in pixel_samples.iter().enumerate() {
                    let u = adjusted_pixel_size * (col as f64 - half_img_w + point.x);
                    let v = adjusted_pixel_size * ((img_h - *row) as f64 - half_img_h + point.y);
                    let lens_sample = &disc_samples[index];
                    let lpx = lens_sample.x * self.lens_radius;
                    let lpy = lens_sample.y * self.lens_radius;
                    let r = Ray {
                        direction: self.ray_direction(u, v, lpx, lpy),
                        origin: self.settings.eye + lpx * self.settings.u + lpy * self.settings.v,
                    };

                    color += s.shade(r);
                }

                color *= pixel_denom;
                color.max_to_one();
                color
            }).collect();

            row_pixels
        }).collect();

        WorkUnitResult {
            work_unit: work,
            rows: row_pixel_vecs,
        }
    }
}
