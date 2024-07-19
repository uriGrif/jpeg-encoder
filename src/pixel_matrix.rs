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

    pub fn initialize_pixels(&mut self, len: usize) {
        self.pixels.resize_with(len, Default::default);
    }

    pub fn push_next(&mut self, value: T) {
        self.pixels.push(value);
    }

    pub fn get_pixel(&self, row: usize, col: usize) -> Option<&T> {
        if row >= self.height || col >= self.width {
            return None;
        }
        self.pixels.get(row * self.width + col)
    }

    pub fn set_pixel(&mut self, row: usize, col: usize, value: T) {
        self.pixels[row * self.width + col] = value;
    }

    pub fn for_each_pixel<F>(&self, f: &mut F) where F: FnMut(&T) {
        for p in self.pixels.as_slice() {
            f(p);
        }
    }

    pub fn block_operation<F>(
        &self,
        block_idx: usize,
        block_width: usize,
        block_height: usize,
        use_default_padding: bool,
        block_buffer: &mut Vec<T>,
        f: &mut F
    )
        where F: FnMut(&mut Vec<T>)
    {
        let blocks_per_row = self.width.div_ceil(block_width);
        let block_start_i = (block_idx / blocks_per_row) * block_height;
        let block_start_j = (block_idx % blocks_per_row) * block_width;

        for i in 0..block_height {
            for j in 0..block_width {
                match self.get_pixel(block_start_i + i, block_start_j + j) {
                    Some(p) => {
                        block_buffer.push(*p);
                    }
                    None => {
                        if use_default_padding {
                            block_buffer.push(T::default());
                        }
                    }
                }
            }
        }

        f(block_buffer);
    }

    pub fn for_each_block<F>(
        &self,
        block_width: usize,
        block_height: usize,
        use_default_padding: bool,
        f: &mut F
    )
        where F: FnMut(&mut Vec<T>)
    {
        let blocks_per_row = self.width.div_ceil(block_width);
        let blocks_per_col = self.height.div_ceil(block_height);
        let mut block_buffer = Vec::<T>::with_capacity(block_width * block_height);

        for i in 0..blocks_per_row * blocks_per_col {
            block_buffer.clear();
            self.block_operation(
                i,
                block_width,
                block_height,
                use_default_padding,
                &mut block_buffer,
                f
            );
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
        assert_eq!(*result, 1);
        let result = matrix.get_pixel(1, 3).unwrap();
        assert_eq!(*result, 3);
        let result = matrix.get_pixel(2, 1).unwrap();
        assert_eq!(*result, 8);
        let result = matrix.get_pixel(3, 2);
        assert_eq!(result, None);
    }

    #[test]
    fn set_pixel() {
        let mut matrix = initialize_matrix();
        matrix.set_pixel(2, 3, 8);
        let result = matrix.get_pixel(2, 3).unwrap();
        assert_eq!(*result, 8);
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
        matrix.for_each_block(block_width, block_height, false, &mut get_biggest);

        let block_width = 3;
        let block_height = 1;
        matrix.set_pixel(2, 3, -10);
        matrix.for_each_block(block_width, block_height, true, &mut get_biggest);
        assert_eq!(biggest_of_each_block, vec![2, 4, 8, 9, 4, 2, 2, 3, 8, i32::default()]);
    }
}
