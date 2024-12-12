use byteorder::{ ByteOrder, LittleEndian };
use std::fs::File;
use std::io::{ Read, Seek, SeekFrom };
use crate::pixel_matrix::pixel_matrix::PixelMatrix;
use crate::utils::colorspace::RGBValue;

const BMP_IMAGE_DATA_START_OFFSET: u64 = 10;
const BMP_PIXEL_WIDTH_OFFSET: u64 = 18;
const BMP_PIXEL_HEIGHT_OFFSET: u64 = 22;

pub struct BmpImage {
    file: Option<File>,
    path: Option<String>,
    pub width: i32,
    pub height: i32,
    image_data_offset: u32,
    pub pixels: PixelMatrix<RGBValue>,
}

impl BmpImage {
    pub fn new(path: &String) -> BmpImage {
        // TODO: add error handling. Note: Check "anyhow"

        // TODO: check if file header corresponds to BMP

        let mut file: File = File::open(&path).expect("Could not open bitmap image file!\n");

        let mut metadata_buffer: [u8; 4] = [0; 4];
        let image_data_offset: u32;
        let width: i32;
        let height: i32;

        _ = file.seek(SeekFrom::Start(BMP_IMAGE_DATA_START_OFFSET));
        _ = file.read_exact(&mut metadata_buffer);
        image_data_offset = LittleEndian::read_u32(&metadata_buffer);

        _ = file.seek(SeekFrom::Start(BMP_PIXEL_WIDTH_OFFSET));
        _ = file.read_exact(&mut metadata_buffer);
        width = LittleEndian::read_i32(&metadata_buffer);

        _ = file.seek(SeekFrom::Start(BMP_PIXEL_HEIGHT_OFFSET));
        _ = file.read_exact(&mut metadata_buffer);
        height = LittleEndian::read_i32(&metadata_buffer);

        let pixels = PixelMatrix::new_with_default(width as usize, height as usize);

        BmpImage {
            file: Some(file),
            path: Some(path.clone()),
            width,
            height,
            image_data_offset,
            pixels,
        }
    }

    pub fn load_pixels(&mut self) {
        _ = self.file
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(self.image_data_offset as u64));

        let mut pixel_buffer: [u8; 3] = [0u8; 3];

        // the file image data in bmp files goes left to right, bottom to top
        // here, it will be stored left to right, top to bottom

        let bytes_to_ignore = ((self.width as u32) * 3).div_ceil(4) * 4 - (self.width as u32) * 3; // the * 3 is to account for 3 bytes for each pixel

        for row in (0..self.height as usize).rev() {
            for col in 0..self.width as usize {
                _ = self.file.as_ref().unwrap().read(&mut pixel_buffer);

                // BMP stores the pixel in BGR order (yeah, not kidding)
                self.pixels.set_pixel(row, col, (
                    pixel_buffer[2],
                    pixel_buffer[1],
                    pixel_buffer[0],
                ));

                // ignore bytes added at the end of each line so that its size in bytes is a multiple of 4 (BMP format)
                if col == (self.width as usize) - 1 {
                    for _ in 0..bytes_to_ignore {
                        let mut aux = [0u8];
                        _ = self.file.as_ref().unwrap().read(&mut aux);
                    }
                }
            }
        }
    }

    pub fn pixel_amount(&self) -> i32 {
        self.width * self.height
    }
}
