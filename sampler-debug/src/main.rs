
use nalgebra::Vector3;

use fluxcore::color::Color;
use fluxcore::image::Image;
use samplers;

use std::fs::File;

fn plot_hemi_sample(i: &mut Image, sample: Vector3<f64>) {
    let x = (((sample.x / 2.0) + 0.5) * (i.width as f64 - 0.01)) as usize;
    let y = (((sample.y / 2.0) + 0.5) * (i.height as f64 - 0.01)) as usize;
    i.set_pixel(y, x, Color::new(sample.z, 0.2, 0.2));
}

fn main() {
    let sample_root = 100;
    let mut i = Image::new(100, 100);

    let mut sampler = samplers::Sampler::new();
    let hemi = samplers::to_hemisphere(sampler.grid_jittered(sample_root), 0.0);

    for sample in hemi {
        plot_hemi_sample(&mut i, sample);
    }

    let path = "sampler-debug.ppm";
    let mut output_file = File::create(path).unwrap();
    i.write(&mut output_file);
    println!("Wrote output to {}", path);
}
