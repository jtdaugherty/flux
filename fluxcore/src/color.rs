
use std::ops::AddAssign;
use std::ops::DivAssign;
use std::ops::MulAssign;
use std::ops::Mul;
use std::ops::Add;
use std::cmp::Eq;

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64) -> Color {
        Color { r, g, b }
    }

    pub fn all(v: f64) -> Color {
       Color::new(v, v, v)
    }

    pub fn black() -> Color {
        Color::all(0.0)
    }

    pub fn white() -> Color {
        Color::all(1.0)
    }

    pub fn max_to_one(&mut self) -> () {
        let mx1 = if self.r > self.g { self.r } else { self.g };
        let mx2 = if mx1 > self.b { mx1 } else { self.b };
        if mx2 > 1.0 {
            let i = 1.0 / mx2;
            self.r *= i;
            self.g *= i;
            self.b *= i;
        }
    }
}

impl Mul<Color> for Color {
    type Output = Self;

    fn mul(self, other: Color) -> Color {
        Color {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
        }
    }
}

impl Mul<f64> for Color {
    type Output = Self;

    fn mul(self, other: f64) -> Color {
        Color {
            r: self.r * other,
            g: self.g * other,
            b: self.b * other,
        }
    }
}

impl Add<Color> for Color {
    type Output = Self;

    fn add(self, other: Color) -> Color {
        Color {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
        }
    }
}

impl DivAssign<f64> for Color {
    fn div_assign(&mut self, d: f64) {
        self.r /= d;
        self.g /= d;
        self.b /= d;
    }
}

impl MulAssign<f64> for Color {
    fn mul_assign(&mut self, d: f64) {
        self.r *= d;
        self.g *= d;
        self.b *= d;
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, other: Color) {
        self.r += other.r;
        self.g += other.g;
        self.b += other.b;
    }
}
