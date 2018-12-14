
use nalgebra::Vector3;

use fluxcore::color::Color;
use fluxcore::image::Image;
use samplers;

use clap::{App, Arg};

use std::fs::File;
use std::str::FromStr;

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

fn plot(base: Vec<samplers::UnitSquareSample>, basename: &str) {
    let mut i1 = Image::new(100, 100);
    for sample in base.clone() {
        plot_2d_sample(&mut i1, sample);
    }

    let path1 = format!("sampler-debug-{}.ppm", basename);
    let mut output_file = File::create(path1.clone()).unwrap();
    i1.write(&mut output_file);
    println!("Wrote output to {}", path1);

    let mut i2 = Image::new(100, 100);
    let hemi = samplers::to_hemisphere(base, 0.0);
    for sample in hemi {
        plot_hemi_sample(&mut i2, sample);
    }

    let path2 = format!("sampler-debug-{}-hemi.ppm", basename);
    let mut output_file = File::create(path2.clone()).unwrap();
    i2.write(&mut output_file);
    println!("Wrote output to {}", path2);
}

fn main() {
    let config = config_from_args();

    let mut sampler = samplers::Sampler::new();

    plot(samplers::grid_regular(config.sample_root), "r");
    plot(sampler.grid_jittered(config.sample_root), "j");
    plot(sampler.grid_multi_jittered(config.sample_root), "mj");
    plot(sampler.grid_correlated_multi_jittered(config.sample_root), "cmj");
}

struct Config {
    sample_root: usize,
}

fn config_from_args() -> Config {
    let app = App::new("sampler-debug")
        .author("Jonathan Daugherty <cygnus@foobox.com>")
        .about("Sampler debugging utility")
        .arg(Arg::with_name("sample_root")
             .short("r")
             .long("root")
             .help("Sample root")
             .takes_value(true));

    let ms = app.get_matches();

    Config {
        sample_root: match ms.value_of("sample_root") {
            None => 10,
            Some(r) => usize::from_str(r).unwrap(),
        },
    }
}
