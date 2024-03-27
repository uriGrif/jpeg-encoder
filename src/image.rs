use std::f32::consts::{ PI, SQRT_2 };
use std::io::Read;
use std::process::exit;
use std::{ fs::File, io::Seek };
use std::os::windows::fs::FileExt;
use byteorder::{ ByteOrder, LittleEndian };
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };

const BMP_IMAGE_DATA_START_OFFSET: u64 = 10;
const BMP_PIXEL_WIDTH_OFFSET: u64 = 18;
const BMP_PIXEL_HEIGHT_OFFSET: u64 = 22;

const DEFAULT_Y_QUANTIZATION_TABLE: [i8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];
const DEFAULT_CH_QUANTIZATION_TABLE: [i8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];

// const uint8_t DefaultQuantLuminance[8*8] =
//     { 16, 11, 10, 16, 24, 40, 51, 61, // there are a few experts proposing slightly more efficient values,
//       12, 12, 14, 19, 26, 58, 60, 55, // e.g. https://www.imagemagick.org/discourse-server/viewtopic.php?t=20333
//       14, 13, 16, 24, 40, 57, 69, 56, // btw: Google's Guetzli project optimizes the quantization tables per image
//       14, 17, 22, 29, 51, 87, 80, 62,
//       18, 22, 37, 56, 68,109,103, 77,
//       24, 35, 55, 64, 81,104,113, 92,
//       49, 64, 78, 87,103,121,120,101,
//       72, 92, 95, 98,112,100,103, 99 };
// const uint8_t DefaultQuantChrominance[8*8] =
//     { 17, 18, 24, 47, 99, 99, 99, 99,
//       18, 21, 26, 66, 99, 99, 99, 99,
//       24, 26, 56, 99, 99, 99, 99, 99,
//       47, 66, 99, 99, 99, 99, 99, 99,
//       99, 99, 99, 99, 99, 99, 99, 99,
//       99, 99, 99, 99, 99, 99, 99, 99,
//       99, 99, 99, 99, 99, 99, 99, 99,
//       99, 99, 99, 99, 99, 99, 99, 99 };

pub struct JpegImage {
    width: i32,
    height: i32,
    y_channel: Vec<u8>,
    cb_channel: Vec<u8>,
    cr_channel: Vec<u8>,
    y_dct_coeffs: Vec<i8>,
    cb_dct_coeffs: Vec<i8>,
    cr_dct_coeffs: Vec<i8>,
}

impl JpegImage {
    pub fn create_from_bmp(file_path: &String) -> JpegImage {
        let mut file: File = File::open(file_path).expect("Could not open bitmap image file!\n");

        let mut metadata_buffer: [u8; 4] = [0; 4];
        let image_data_offset: u32;
        let width: i32;
        let height: i32;

        _ = file.seek_read(&mut metadata_buffer, BMP_IMAGE_DATA_START_OFFSET);
        image_data_offset = LittleEndian::read_u32(&metadata_buffer);

        _ = file.seek_read(&mut metadata_buffer, BMP_PIXEL_WIDTH_OFFSET);
        width = LittleEndian::read_i32(&metadata_buffer);

        _ = file.seek_read(&mut metadata_buffer, BMP_PIXEL_HEIGHT_OFFSET);
        height = LittleEndian::read_i32(&metadata_buffer);

        let channel_len = (width * height) as usize;

        let mut y_channel: Vec<u8> = vec![0u8; channel_len];
        let mut cb_channel: Vec<u8> = vec![0u8; channel_len];
        let mut cr_channel: Vec<u8> = vec![0u8; channel_len];

        JpegImage::bmp_rgb_to_jpeg_ycbcr(
            &mut file,
            width,
            height,
            image_data_offset,
            &mut y_channel,
            &mut cb_channel,
            &mut cr_channel
        );

        let dct_len = ((width + (width % 8)) * (height + (height % 8))) as usize; // dct works in blocks of 8x8 pixels, so a padding is needed in order to make it divisible by 8

        let y_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len);
        let cb_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len / 2); // smaller, because these channels will later be downsampled
        let cr_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len / 2);

        JpegImage {
            width,
            height,
            y_channel,
            cb_channel,
            cr_channel,
            y_dct_coeffs,
            cb_dct_coeffs,
            cr_dct_coeffs,
        }
    }

    fn bmp_rgb_to_jpeg_ycbcr(
        file: &mut File,
        width: i32,
        height: i32,
        image_data_offset: u32,
        y_channel: &mut Vec<u8>,
        cb_channel: &mut Vec<u8>,
        cr_channel: &mut Vec<u8>
    ) {
        // set cursor to begining of image data
        _ = file.seek(std::io::SeekFrom::Start(image_data_offset as u64));

        let mut pixel_buffer: [u8; 3] = [0u8; 3];

        // the file image data in bmp files goes left to right, bottom to top
        // in our jpeg, it will be stored left to right, top to bottom

        for vertical_idx in (0..height).rev() {
            for horizontal_idx in 0..width {
                _ = file.read(&mut pixel_buffer);

                let rgb: RGBValue = (pixel_buffer[0], pixel_buffer[1], pixel_buffer[2]);

                let ycbcr: YCbCrValue = rgb_to_ycbcr(rgb);

                y_channel[(width * vertical_idx + horizontal_idx) as usize] = ycbcr.0;
                cb_channel[(width * vertical_idx + horizontal_idx) as usize] = ycbcr.1;
                cr_channel[(width * vertical_idx + horizontal_idx) as usize] = ycbcr.2;
            }
        }
    }

    pub fn chrominance_downsampling(&mut self) {
        let new_width = self.width / 2 + (self.width % 2);
        let new_height = self.height / 2 + (self.height % 2);

        let mut new_cb = Vec::<u8>::with_capacity((new_width * new_height) as usize);
        let mut new_cr = Vec::<u8>::with_capacity((new_width * new_height) as usize);

        let width_iter = (0..self.width as i32).filter(|x| x % 2 == 0);
        let height_iter = (0..self.height as i32).filter(|x| x % 2 == 0);

        for i in height_iter {
            for j in width_iter.clone() {
                let mut cb_values_amount: u16 = 0;
                let mut cr_values_amount: u16 = 0;
                let mut cb_values_sum: u16 = 0;
                let mut cr_values_sum: u16 = 0;

                // TODO
                // modularizar esto

                cb_values_sum += self.cb_channel[(self.height * i + j) as usize] as u16;
                cb_values_amount += 1;
                cr_values_sum += self.cb_channel[(self.height * i + j) as usize] as u16;
                cr_values_amount += 1;

                if j < self.width - 1 {
                    cb_values_sum += self.cb_channel[(self.height * i + j + 1) as usize] as u16;
                    cb_values_amount += 1;
                    cr_values_sum += self.cb_channel[(self.height * i + j + 1) as usize] as u16;
                    cr_values_amount += 1;
                }

                if i < self.height - 1 {
                    cb_values_sum += self.cb_channel[(self.height * (i + 1) + j) as usize] as u16;
                    cb_values_amount += 1;
                    cr_values_sum += self.cb_channel[(self.height * (i + 1) + j) as usize] as u16;
                    cr_values_amount += 1;
                }

                if j < self.width - 1 && i < self.height - 1 {
                    cb_values_sum += self.cb_channel
                        [(self.height * (i + 1) + j + 1) as usize] as u16;
                    cb_values_amount += 1;
                    cr_values_sum += self.cb_channel
                        [(self.height * (i + 1) + j + 1) as usize] as u16;
                    cr_values_amount += 1;
                }

                // push average value of 2x2 pixel block
                new_cb.push((cb_values_sum / cb_values_amount) as u8);
                new_cr.push((cr_values_sum / cr_values_amount) as u8);
            }
        }

        self.cb_channel = new_cb;
        self.cr_channel = new_cr;
    }

    pub fn dct_and_quant(&mut self) {
        let mut buffer: [i8; 64] = [0; 64]; // stores a block of 8x8 pixels, with their values shifted to [-128;127]
        let mut dct_buffer: [f32; 64] = [0.0; 64]; // stores the dct coefficients for a block

        // TODO
        // - mucho codigo repetido -> ordenar
        // - procesar canales en paralelo

        // luminance dct
        let width_iter = (0..self.width as i32).filter(|x| x % 8 == 0);
        let height_iter = (0..self.height as i32).filter(|x| x % 8 == 0);

        for row in height_iter {
            for column in width_iter.clone() {
                Self::block_to_buffer(&mut buffer, row, column, &self.y_channel);
                // perform DCT
                Self::discrete_cosine_transform(buffer, &mut dct_buffer);
                // perform Quantization
                Self::quantization(&mut dct_buffer, true);
                // add buffer to dct_coeffs Vec
                for i in 0..64 {
                    self.y_dct_coeffs.push(dct_buffer[i] as i8);
                }
            }
        }

        // chrominance dct

        let ch_width = self.width / 2 + (self.width % 2);
        let ch_height = self.height / 2 + (self.height % 2);

        let width_iter = (0..ch_width).filter(|x| x % 8 == 0);
        let height_iter = (0..ch_height).filter(|x| x % 8 == 0);

        for row in height_iter {
            for column in width_iter.clone() {
                // Blue Chrominance Channel

                Self::block_to_buffer(&mut buffer, row, column, &self.cb_channel);
                // perform DCT
                Self::discrete_cosine_transform(buffer, &mut dct_buffer);
                // perform Quantization
                Self::quantization(&mut dct_buffer, false);
                // add buffer to dct_coeffs Vec
                for i in 0..64 {
                    self.cb_dct_coeffs.push(dct_buffer[i] as i8);
                }

                // Red Chrominance Channel

                Self::block_to_buffer(&mut buffer, row, column, &self.cr_channel);
                // perform DCT
                Self::discrete_cosine_transform(buffer, &mut dct_buffer);
                // perform Quantization
                Self::quantization(&mut dct_buffer, false);
                // add buffer to dct_coeffs Vec
                for i in 0..64 {
                    self.cr_dct_coeffs.push(dct_buffer[i] as i8);
                }
            }
        }
    }

    fn dct_shift_range(n: u8) -> i8 {
        if n <= 127 { (n | 128u8) as i8 } else { (n & 127u8) as i8 }
    }

    fn block_to_buffer(buffer: &mut [i8; 64], row: i32, column: i32, channel: &Vec<u8>) {
        for i in 0..8 {
            for j in 0..8 {
                let buffer_index = (8 * i + j) as usize;
                let index = (8 * (column + i) + row + j) as usize;
                if index < channel.len() {
                    buffer[buffer_index] = Self::dct_shift_range(channel[index]);
                } else {
                    buffer[buffer_index] = 0; // Padding of zeros may not be the best approach, but it's the easiest
                }
            }
        }
    }

    fn discrete_cosine_transform(block: [i8; 64], result: &mut [f32; 64]) {
        let mut alpha_u: f32;
        let mut alpha_v: f32;
        let mut sum: f32;
        for u in 0..8 {
            if u == 0 {
                alpha_u = 1.0 / SQRT_2;
            } else {
                alpha_u = 1.0;
            }
            for v in 0..8 {
                sum = 0.0;
                if v == 0 {
                    alpha_v = 1.0 / SQRT_2;
                } else {
                    alpha_v = 1.0;
                }

                for x in 0..8 {
                    for y in 0..8 {
                        sum +=
                            (block[x * 8 + y] as f32) *
                            ((((2 * x + 1) as f32) * (u as f32) * PI) / 16.0).cos() *
                            ((((2 * y + 1) as f32) * (v as f32) * PI) / 16.0).cos();
                    }
                }

                result[u * 8 + v] = 0.25 * alpha_u * alpha_v * sum;
            }
        }
    }

    fn quantization(buffer: &mut [f32; 64], is_luminance: bool) {
        for i in 0..64 {
            buffer[i] = f32::round(
                buffer[i] /
                    (
                        (if is_luminance {
                            DEFAULT_Y_QUANTIZATION_TABLE[i]
                        } else {
                            DEFAULT_CH_QUANTIZATION_TABLE[i]
                        }) as f32
                    )
            );
        }
    }
}
