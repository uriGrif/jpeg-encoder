pub struct PixelMatrix<T> {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<T>,
}

impl<T: Default + Copy> PixelMatrix<T> {
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
        let idx = row * self.width + col;
        if idx >= self.pixels.len() {
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
}

pub struct PixelMatrixBlockIterator<'a, T> {
    pixel_matrix: &'a mut PixelMatrix<T>,
    block_width: usize,
    block_height: usize,
    block_idx: usize,
    row_in_block_idx: usize,
    col_in_block_idx: usize,
    use_default_padding: bool,
}

impl<'a, T: Default + Copy> PixelMatrixBlockIterator<'a, T> {
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
            self.block_idx += 1;
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

    pub fn get_block(&self, block_buffer: &mut Vec<T>) {
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
            block_buffer.clear();
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
