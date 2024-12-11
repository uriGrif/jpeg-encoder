use std::fmt::Debug;
use crate::pixel_matrix::block_iterator::PixelMatrixBlockIterator;

pub struct PixelMatrix<T> {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<T>,
}

impl<T: Default + Copy + Debug> PixelMatrix<T> {
    pub fn new(width: usize, height: usize) -> PixelMatrix<T> {
        PixelMatrix {
            width,
            height,
            pixels: Vec::<T>::with_capacity(width * height),
        }
    }

    pub fn new_with_default(width: usize, height: usize) -> PixelMatrix<T> {
        PixelMatrix {
            width,
            height,
            pixels: vec![Default::default(); width * height],
        }
    }

    pub fn new_from_pixels(width: usize, height: usize, pixels: Vec<T>) -> PixelMatrix<T> {
        PixelMatrix {
            width,
            height,
            pixels,
        }
    }

    pub fn push_next(&mut self, value: T) {
        self.pixels.push(value);
    }

    pub fn get_pixel(&self, row: usize, col: usize) -> Option<T> {
        if row >= self.height || col >= self.width {
            return None;
        }
        Some(self.pixels[row * self.width + col])
    }

    pub fn set_pixel(&mut self, row: usize, col: usize, value: T) {
        self.pixels[row * self.width + col] = value;
    }

    pub fn for_each_pixel<F>(&self, f: &mut F) where F: FnMut(&T) {
        for p in self.pixels.as_slice() {
            f(p);
        }
    }

    pub fn get_block_iterator(
        &mut self,
        block_width: usize,
        block_height: usize,
        use_default_padding: bool
    ) -> PixelMatrixBlockIterator<'_, T> {
        PixelMatrixBlockIterator::new(self, block_width, block_height, use_default_padding)
    }

    pub fn pretty_print(&self) {
        for i in 0..self.height {
            for j in 0..self.width {
                print!("{:?}  ", self.pixels[i * self.width + j]);
            }
            println!(";"); // Move to the next line after each row
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
        test matrix:
        [
        1,1,4,2,
        1,2,2,3,
        8,8,8,9
        ]
    */

    fn initialize_matrix() -> PixelMatrix<i32> {
        let mut matrix = PixelMatrix::new(4, 3);
        matrix.push_next(1);
        matrix.push_next(1);
        matrix.push_next(4);
        matrix.push_next(2);
        matrix.push_next(1);
        matrix.push_next(2);
        matrix.push_next(2);
        matrix.push_next(3);
        matrix.push_next(8);
        matrix.push_next(8);
        matrix.push_next(8);
        matrix.push_next(9);
        matrix
    }

    #[test]
    fn push_and_get() {
        let matrix = initialize_matrix();
        let result = matrix.get_pixel(0, 0).unwrap();
        assert_eq!(result, 1);
        let result = matrix.get_pixel(1, 3).unwrap();
        assert_eq!(result, 3);
        let result = matrix.get_pixel(2, 1).unwrap();
        assert_eq!(result, 8);
        let result = matrix.get_pixel(3, 2);
        assert_eq!(result, None);
    }

    #[test]
    fn set_pixel() {
        let mut matrix = initialize_matrix();
        matrix.set_pixel(2, 3, 8);
        let result = matrix.get_pixel(2, 3).unwrap();
        assert_eq!(result, 8);
    }

    #[test]
    fn for_each_pixel() {
        let matrix = initialize_matrix();
        let mut accum = 0;
        matrix.for_each_pixel(
            &mut (|p: &i32| {
                accum += *p;
            })
        );
        assert_eq!(accum, 49);
    }
}
