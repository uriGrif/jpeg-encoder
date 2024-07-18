pub struct PixelMatrix<T> {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<T>,
}

impl<T: Default> PixelMatrix<T> {
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

    pub fn get_pixel(&self, row: usize, col: usize) -> &T {
        self.pixels.get(row * self.width + col).unwrap()
    }

    pub fn set_pixel(&mut self, row: usize, col: usize, value: T) {
        self.pixels[row * self.width + col] = value;
    }

    pub fn iterate<F>(&self, f: &mut F) where F: FnMut(&T) {
        for p in self.pixels.as_slice() {
            f(p);
        }
    }
}
