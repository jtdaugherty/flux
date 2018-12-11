
extern crate nalgebra;
use nalgebra::{Vector3, normalize};

extern crate rand;
use rand::IsaacRng;
use rand::Rng;
use rand::distributions::{Distribution, Uniform};

#[macro_use] extern crate itertools;

#[derive(Debug)]
pub struct UnitDiscSample {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct UnitSquareSample {
    pub x: f64,
    pub y: f64,
}

pub struct Sampler {
    pub rng: IsaacRng,
}

impl Sampler {
    pub fn new() -> Sampler {
        let mut trng = rand::thread_rng();

        Sampler {
            rng: IsaacRng::new_from_u64(trng.gen())
        }
    }

    pub fn grid_jittered(&mut self, root: usize) -> Vec<UnitSquareSample> {
        let between = Uniform::from(0.0..1.0);
        let increment = 1.0 / (root as f64);
        let regular = grid_regular(root);
        regular.iter().map(
            |p| UnitSquareSample {
                x: p.x + (between.sample(&mut self.rng) - 0.5) * increment,
                y: p.y + (between.sample(&mut self.rng) - 0.5) * increment,
            }).collect()
    }
}

pub fn to_hemisphere(points: Vec<UnitSquareSample>, e: f64) -> Vec<Vector3<f64>> {
    points.iter().map(
        |p| {
            let cos_phi = (2.0 * std::f64::consts::PI * p.x).cos();
            let sin_phi = (2.0 * std::f64::consts::PI * p.x).sin();
            let cos_theta = (1.0 - p.y).powf(1.0 / (e + 1.0));
            let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
            let pu = sin_theta * cos_phi;
            let pv = sin_theta * sin_phi;
            let pw = cos_theta;
            normalize(&Vector3::new(pu, pv, pw))
        }).collect()
}

pub fn to_poisson_disc(points: Vec<UnitSquareSample>) -> Vec<UnitDiscSample> {
    points.iter().map(
        |p| {
            let spx = 2.0 * p.x - 1.0;
            let spy = 2.0 * p.y - 1.0;
            let mut phi: f64;
            let r: f64;

            if spx > -spy {
                if spx > spy {
                    r = spx;
                    phi = spy / spx;
                } else {
                    r = spy;
                    phi = 2.0 - spx / spy;
                }
            } else {
                if spx < spy {
                    r = -spx;
                    phi = 4.0 + spy / spx;
                } else {
                    r = -spy;
                    if spy != 0.0 {
                        phi = 6.0 - spx / spy;
                    } else {
                        phi = 0.0;
                    }
                }
            }

            phi *= std::f64::consts::PI / 4.0;

            UnitDiscSample {
                x: r * phi.cos(),
                y: r * phi.sin(),
            }
        }
        ).collect()
}

pub fn grid_regular(root: usize) -> Vec<UnitSquareSample> {
    let increment = 1.0 / (root as f64);
    let start = 0.5 * increment;
    let range: Vec<f64> = (0..root).map(|i| start + increment * (i as f64)).collect();

    iproduct!(&range, &range).map(
        |(x, y)| UnitSquareSample { x: x.clone(), y: y.clone(), }).collect()
}
