
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use color::Color;

pub struct Image {
    pub height: usize,
    pub width: usize,
    pub pixels: Vec<Vec<Color>>,
}

impl Image {
    pub fn new(w: usize, h: usize) -> Image {
        Image {
            pixels: (0..h).map(|_| vec![]).collect(),
            width: w,
            height: h,
        }
    }

    pub fn set_row(&mut self, row_index: usize, values: Vec<Color>) {
        self.pixels[row_index] = values;
    }

    pub fn set_pixel(&mut self, row_index: usize, col_index: usize, value: Color) {
        if col_index >= self.pixels[row_index].len() {
            if col_index >= self.width {
                panic!("set_pixel col_index {} too big", col_index);
            }

            if row_index >= self.height {
                panic!("set_pixel row_index {} too big", row_index);
            }

            self.pixels[row_index].resize(col_index + 1, Color::black());
        }

        self.pixels[row_index][col_index] = value;
    }

    pub fn write(&self, f: &mut File) {
        let mut buf = BufWriter::new(f);

        write!(buf, "P3\n{} {}\n65535\n", self.width, self.height).unwrap();
        for row in &self.pixels {
            for pixel in row {
                write!(buf, "{} {} {}\n",
                       (pixel.r * 65535.99) as u16,
                       (pixel.g * 65535.99) as u16,
                       (pixel.b * 65535.99) as u16).unwrap();
            }

            // Since this row could have been incomplete/missing, emit
            // enough blank pixels to compensate.
            for _ in 0..(self.width - row.len()) {
                write!(buf, "{} {} {}\n", 0, 0, 0).unwrap();
            }
        }
    }
}
