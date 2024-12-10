use std::fmt::Debug;

use crate::pixel_matrix::pixel_matrix::PixelMatrix;
pub struct PixelMatrixBlockIterator<'a, T> {
    pixel_matrix: &'a mut PixelMatrix<T>,
    block_width: usize,
    block_height: usize,
    block_idx: usize,
    row_in_block_idx: usize,
    col_in_block_idx: usize,
    use_default_padding: bool,
}

impl<'a, T: Default + Copy + Debug> PixelMatrixBlockIterator<'a, T> {
    pub fn new(
        pixel_matrix: &'a mut PixelMatrix<T>,
        block_width: usize,
        block_height: usize,
        use_default_padding: bool
    ) -> PixelMatrixBlockIterator<'a, T> {
        PixelMatrixBlockIterator {
            pixel_matrix,
            block_width,
            block_height,
            use_default_padding,
            block_idx: 0,
            row_in_block_idx: 0,
            col_in_block_idx: 0,
        }
    }

    pub fn reset(&mut self) {
        self.block_idx = 0;
        self.row_in_block_idx = 0;
        self.col_in_block_idx = 0;
    }

    pub fn get_blocks_per_row(&self) -> usize {
        self.pixel_matrix.width.div_ceil(self.block_width)
    }

    pub fn get_blocks_per_column(&self) -> usize {
        self.pixel_matrix.height.div_ceil(self.block_height)
    }

    pub fn get_blocks_amount(&self) -> usize {
        self.get_blocks_per_row() * self.get_blocks_per_column()
    }

    fn increment_idx(&mut self) {
        if
            self.row_in_block_idx == self.block_height - 1 &&
            self.col_in_block_idx == self.block_width - 1
        {
            self.increment_block_idx();
        }
        self.col_in_block_idx = (self.col_in_block_idx + 1) % self.block_width;
        if self.col_in_block_idx == 0 {
            self.row_in_block_idx = (self.row_in_block_idx + 1) % self.block_height;
        }
    }

    pub fn get_next_pixel(&mut self) -> Option<T> {
        let blocks_per_row = self.get_blocks_per_row();
        let block_start_i = (self.block_idx / blocks_per_row) * self.block_height;
        let block_start_j = (self.block_idx % blocks_per_row) * self.block_width;

        let pixel = self.pixel_matrix.get_pixel(
            block_start_i + self.row_in_block_idx,
            block_start_j + self.col_in_block_idx
        );

        self.increment_idx();

        pixel
    }

    pub fn set_next_pixel(&mut self, value: T) {
        let blocks_per_row = self.get_blocks_per_row();
        let block_start_i = (self.block_idx / blocks_per_row) * self.block_height;
        let block_start_j = (self.block_idx % blocks_per_row) * self.block_width;

        self.pixel_matrix.set_pixel(
            block_start_i + self.row_in_block_idx,
            block_start_j + self.col_in_block_idx,
            value
        );

        self.increment_idx();
    }

    pub fn increment_block_idx(&mut self) {
        if self.block_idx == self.get_blocks_amount() - 1 {
            self.block_idx = 0;
        } else {
            self.block_idx += 1;
        }
    }

    pub fn get_block(&self, block_buffer: &mut Vec<T>) {
        block_buffer.clear();
        let blocks_per_row = self.get_blocks_per_row();
        let block_start_i = (self.block_idx / blocks_per_row) * self.block_height;
        let block_start_j = (self.block_idx % blocks_per_row) * self.block_width;

        for i in 0..self.block_height {
            for j in 0..self.block_width {
                match self.pixel_matrix.get_pixel(block_start_i + i, block_start_j + j) {
                    Some(p) => {
                        block_buffer.push(p);
                    }
                    None => {
                        if self.use_default_padding {
                            block_buffer.push(T::default());
                        }
                    }
                }
            }
        }
    }

    pub fn block_operation<F>(&self, block_buffer: &mut Vec<T>, f: &mut F)
        where F: FnMut(&mut Vec<T>)
    {
        self.get_block(block_buffer);

        f(block_buffer);
    }

    pub fn for_each_block<F>(&mut self, f: &mut F) where F: FnMut(&mut Vec<T>) {
        let mut block_buffer = Vec::<T>::with_capacity(self.block_width * self.block_height);

        for i in 0..self.get_blocks_amount() {
            self.block_idx = i;
            self.row_in_block_idx = 0;
            self.col_in_block_idx = 0;
            self.block_operation(&mut block_buffer, f);
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
    fn for_each_block() {
        let mut matrix = initialize_matrix();
        let block_width = 2;
        let block_height = 2;
        let mut biggest_of_each_block = Vec::<i32>::new();
        let mut get_biggest = |block: &mut Vec<i32>| {
            biggest_of_each_block.push(*block.iter().max().unwrap());
        };
        let mut iterator = matrix.get_block_iterator(block_width, block_height, false);
        iterator.for_each_block(&mut get_biggest);

        let block_width = 3;
        let block_height = 1;
        matrix.set_pixel(2, 3, -10);
        let mut iterator = matrix.get_block_iterator(block_width, block_height, true);
        iterator.for_each_block(&mut get_biggest);

        assert_eq!(biggest_of_each_block, vec![2, 4, 8, 9, 4, 2, 2, 3, 8, i32::default()]);
    }
}
