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
        padding: Option<T>,
        block_buffer: &mut Vec<T>,
        f: &mut F
    )
        where F: FnMut(&T)
    {
        let blocks_per_row = self.width.div_ceil(block_width);
        let block_start_i = (block_idx / blocks_per_row) * block_height;
        let block_start_j = (block_idx % blocks_per_row) * block_width;

        for i in 0..block_height {
            for j in 0..block_width {
                block_buffer.push(
                    *self.get_pixel(block_start_i + i, block_start_j + j).unwrap_or(&T::default())
                );
            }
        }

        for p in block_buffer {
            f(p);
        }
    }

    pub fn for_each_block<F>(
        &self,
        block_width: usize,
        block_height: usize,
        padding: Option<T>,
        f: &mut F
    )
        where F: FnMut(&T) {}
}
