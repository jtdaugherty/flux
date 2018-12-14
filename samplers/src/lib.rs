
use nalgebra::{Vector3, normalize};

use rand::IsaacRng;
use rand::Rng;
use rand::distributions::{Distribution, Uniform};

#[macro_use] extern crate itertools;

#[derive(Debug)]
pub struct UnitDiscSample {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Copy, Clone)]
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

    fn grid_multi_jittered_base(&mut self, root: usize) -> Vec<Vec<UnitSquareSample>> {
        let r2 = (root * root) as f64;
        let r_float = root as f64;
        let between = Uniform::from(0.0..1.0);
        let range: Vec<(f64, f64)> = (0..root).map(|v| (v as f64, (root - 1 - v) as f64)).collect();

        range.iter().map(|(big_row, little_col)| {
            range.iter().map(|(big_col, little_row)| {
                let a = between.sample(&mut self.rng);
                let b = between.sample(&mut self.rng);
                UnitSquareSample {
                    x: (big_row / r_float) + (little_row + a) / r2,
                    y: (big_col / r_float) + (little_col + b) / r2,
                }
            }).collect()
        }).collect()
    }

    pub fn grid_correlated_multi_jittered(&mut self, root: usize) -> Vec<UnitSquareSample> {
        let samples = self.grid_multi_jittered_base(root);

        let mut x_idxs: Vec<usize> = (0..root).collect();
        let mut y_idxs: Vec<usize> = (0..root).collect();

        self.rng.shuffle(&mut x_idxs);
        self.rng.shuffle(&mut y_idxs);

        let y_shuffled: Vec<Vec<UnitSquareSample>> = samples.iter().map(|vec| self.shuffle_y(&y_idxs, &vec)).collect();
        let x_shuffled: Vec<Vec<UnitSquareSample>> = transpose(
            transpose(y_shuffled).iter().map(|v| self.shuffle_x(&x_idxs, &v)).collect()
            );

        concat_vec(x_shuffled)
    }

    fn shuffle_y(&self, idxs: &Vec<usize>, vals: &Vec<UnitSquareSample>) -> Vec<UnitSquareSample> {
        idxs.iter().zip(vals).map(|(idx, sample)| {
            let other = &vals[*idx];
            UnitSquareSample {
                x: sample.x,
                y: other.y,
            }
        }).collect()
    }

    fn shuffle_x(&self, idxs: &Vec<usize>, vals: &Vec<UnitSquareSample>) -> Vec<UnitSquareSample> {
        idxs.iter().zip(vals).map(|(idx, sample)| {
            let other = &vals[*idx];
            UnitSquareSample {
                x: other.x,
                y: sample.y,
            }
        }).collect()
    }
}

pub fn to_hemisphere(points: Vec<UnitSquareSample>, e: f64) -> Vec<Vector3<f64>> {
    points.iter().map(|p| to_unit_hemi(p, e)).collect()
}

pub fn to_unit_hemi(p: &UnitSquareSample, e: f64) -> Vector3<f64> {
    let cos_phi = (2.0 * std::f64::consts::PI * p.x).cos();
    let sin_phi = (2.0 * std::f64::consts::PI * p.x).sin();
    let cos_theta = (1.0 - p.y).powf(1.0 / (e + 1.0));
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
    let pu = sin_theta * cos_phi;
    let pv = sin_theta * sin_phi;
    let pw = cos_theta;
    normalize(&Vector3::new(pu, pv, pw))
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

pub fn concat_vec<T>(v: Vec<Vec<T>>) -> Vec<T> {
    let mut v0 = vec![];
    let mut vs = v;

    for i in 0..vs.len() {
        v0.append(&mut vs[i]);
    }

    v0
}

pub fn transpose<T: Copy>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    match v.iter().map(|v2| v2.len()).max() {
        None => panic!("transpose: input vector empty"),
        Some(max_length) => {
            (0..max_length).map(|i| {
                (0..v.len()).map(|j| {
                    v[j][i]
                }).collect()
            }).collect()
        },
    }
}
