
use nalgebra::Vector3;

use fluxcore::color::Color;
use fluxcore::image::Image;
use samplers;

use std::fs::File;

fn plot_2d_sample(i: &mut Image, sample: samplers::UnitSquareSample) {
    let x = (sample.x * (i.width as f64 - 0.01)) as usize;
    let y = (sample.y * (i.height as f64 - 0.01)) as usize;
    i.set_pixel(y, x, Color::new(1.0, 0.2, 0.2));
}

fn plot_hemi_sample(i: &mut Image, sample: Vector3<f64>) {
    let x = (((sample.x / 2.0) + 0.5) * (i.width as f64 - 0.01)) as usize;
    let y = (((sample.y / 2.0) + 0.5) * (i.height as f64 - 0.01)) as usize;
    i.set_pixel(y, x, Color::new(sample.z, 0.2, 0.2));
}

fn main() {
    let sample_root = 10;

    let mut sampler = samplers::Sampler::new();

    let mut i1 = Image::new(100, 100);
    let base = sampler.grid_correlated_multi_jittered(sample_root);
    for sample in base.clone() {
        plot_2d_sample(&mut i1, sample);
    }

    let path = "sampler-debug-square.ppm";
    let mut output_file = File::create(path).unwrap();
    i1.write(&mut output_file);
    println!("Wrote output to {}", path);

    let mut i2 = Image::new(100, 100);
    let hemi = samplers::to_hemisphere(base, 0.0);
    for sample in hemi {
        plot_hemi_sample(&mut i2, sample);
    }

    let path = "sampler-debug-hemi.ppm";
    let mut output_file = File::create(path).unwrap();
    i2.write(&mut output_file);
    println!("Wrote output to {}", path);
}
