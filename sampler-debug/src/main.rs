
use fluxcore::color::Color;
use fluxcore::image::Image;
use samplers;

use std::fs::File;

fn main() {
    let sz = 100;
    let sample_root = 100;
    let mut i = Image::new(sz, sz);

    let mut sampler = samplers::Sampler::new();
    let hemi = samplers::to_hemisphere(sampler.grid_jittered(sample_root), 0.0);

    for sample in hemi {
        let x = (((sample.x / 2.0) + 0.5) * (sz as f64 - 0.01)) as usize;
        let y = (((sample.y / 2.0) + 0.5) * (sz as f64 - 0.01)) as usize;
        println!("{:?} {} {}", &sample, &x, &y);
        i.set_pixel(x, y, Color::new(sample.z, 0.2, 0.2));
    }

    let mut output_file = File::create("sampler-debug.ppm").unwrap();
    i.write(&mut output_file);
}
