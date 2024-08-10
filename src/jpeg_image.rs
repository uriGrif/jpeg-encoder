use std::f32::consts::{ PI, SQRT_2 };
use std::thread;
use std::fs::File;
use crate::bmp_image::BmpImage;
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };
use crate::bit_vec::BitVec;
use crate::pixel_matrix::PixelMatrix;
use crate::quant_tables::*;
use crate::huffman_tables::*;

const DEFAULT_DOWNSAMPLING_RATIO: (u8, u8, u8) = (4, 2, 0);

pub enum DctAlgorithm {
    RealDct,
    BinDct,
}

pub struct JpegImage {
    path: Option<String>,
    file: Option<File>,
    width: i32,
    height: i32,
    chrominance_downsampling_ratio: (u8, u8, u8),
    dct_algorithm: DctAlgorithm,
    y_channel: PixelMatrix<u8>,
    cb_channel: PixelMatrix<u8>,
    cr_channel: PixelMatrix<u8>,
    y_dct_coeffs: Vec<i8>,
    cb_dct_coeffs: Vec<i8>,
    cr_dct_coeffs: Vec<i8>,
}

impl JpegImage {
    pub fn from_bmp(bmp_path: &String) -> JpegImage {
        let mut bmp_image: BmpImage = BmpImage::new(bmp_path);
        bmp_image.load_pixels();

        let y_channel = PixelMatrix::new(bmp_image.width as usize, bmp_image.height as usize);
        let cb_channel = PixelMatrix::new(bmp_image.width as usize, bmp_image.height as usize);
        let cr_channel = PixelMatrix::new(bmp_image.width as usize, bmp_image.height as usize);

        let dct_len = ((bmp_image.width + (bmp_image.width % 8)) *
            (bmp_image.height + (bmp_image.height % 8))) as usize; // dct works in blocks of 8x8 pixels, so a padding is needed in order to make it divisible by 8

        let y_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len);
        let cb_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len / 2); // smaller, because these channels will later be downsampled
        let cr_dct_coeffs: Vec<i8> = Vec::<i8>::with_capacity(dct_len / 2);

        let mut image: JpegImage = JpegImage {
            file: None,
            path: None,
            width: bmp_image.width,
            height: bmp_image.height,
            chrominance_downsampling_ratio: DEFAULT_DOWNSAMPLING_RATIO,
            dct_algorithm: DctAlgorithm::BinDct,
            y_channel,
            cb_channel,
            cr_channel,
            y_dct_coeffs,
            cb_dct_coeffs,
            cr_dct_coeffs,
        };

        image.load_bmp_rgb_to_jpeg_ycbcr(bmp_image);

        image
    }

    fn load_bmp_rgb_to_jpeg_ycbcr(&mut self, bmp_image: BmpImage) {
        let mut f = |rgb_pixel: &RGBValue| {
            let ycbcr: YCbCrValue = rgb_to_ycbcr(rgb_pixel);

            self.y_channel.push_next(ycbcr.0);
            self.cb_channel.push_next(ycbcr.1);
            self.cr_channel.push_next(ycbcr.2);
        };

        bmp_image.pixels.for_each_pixel(&mut f);
    }

    fn get_downsampling_block_dimensions(
        downsampling_ratio: (u8, u8, u8),
        block_width: &mut usize,
        block_height: &mut usize
    ) {
        match downsampling_ratio {
            (4, 4, 4) => {
                *block_width = 1;
                *block_height = 1;
                return;
            }
            (4, 2, 0) => {
                *block_width = 2;
                *block_height = 2;
            }
            (4, 2, 2) => {
                *block_width = 2;
                *block_height = 1;
            }
            _ => {
                panic!("Invalid chrominance downsampling ratio!");
            }
        }
    }

    pub fn chrominance_downsampling(&mut self) {
        let mut block_width: usize = 1;
        let mut block_height: usize = 1;

        Self::get_downsampling_block_dimensions(
            self.chrominance_downsampling_ratio,
            &mut block_width,
            &mut block_height
        );

        let new_channel_width = (self.width as usize).div_ceil(block_width);
        let new_channel_height = (self.height as usize).div_ceil(block_height);

        let mut new_cb = PixelMatrix::<u8>::new(new_channel_width, new_channel_height);
        let mut new_cr = PixelMatrix::<u8>::new(new_channel_width, new_channel_height);

        let add_average = |new_channel: &mut PixelMatrix<u8>, block_buffer: &mut Vec<u8>| {
            new_channel.push_next(
                (block_buffer
                    .iter()
                    .map(|x| *x as usize)
                    .sum::<usize>() / block_buffer.len()) as u8
            );
        };

        let mut add_average_cb = |block_buffer: &mut Vec<u8>| {
            add_average(&mut new_cb, block_buffer);
        };

        let mut add_average_cr = |block_buffer: &mut Vec<u8>| {
            add_average(&mut new_cr, block_buffer);
        };

        thread::scope(|s| {
            let cb_handle = s.spawn(|| {
                self.cb_channel.for_each_block(
                    block_width,
                    block_height,
                    false,
                    &mut add_average_cb
                );
            });

            let cr_handle = s.spawn(|| {
                self.cr_channel.for_each_block(
                    block_width,
                    block_height,
                    false,
                    &mut add_average_cr
                );
            });

            _ = cb_handle.join();
            _ = cr_handle.join();
        });

        self.cb_channel = new_cb;
        self.cr_channel = new_cr;
    }

    pub fn dct_and_quantization(&mut self) {
        let dct_algorithm = match self.dct_algorithm {
            DctAlgorithm::RealDct => Self::forward_real_dct_and_quant,
            DctAlgorithm::BinDct => Self::forward_bin_dct_and_quant,
        };

        let f = |
            final_dct_coeffs: &mut Vec<i8>,
            quantization_table: [i32; 64],
            block_buffer: &mut Vec<u8>
        | {
            dct_algorithm(block_buffer, quantization_table, final_dct_coeffs)
        };

        let mut f_for_y_channel = |block_buffer: &mut Vec<u8>|
            f(&mut self.y_dct_coeffs, DEFAULT_Y_QUANTIZATION_TABLE, block_buffer);

        let mut f_for_cb_channel = |block_buffer: &mut Vec<u8>|
            f(&mut self.cb_dct_coeffs, DEFAULT_CH_QUANTIZATION_TABLE, block_buffer);

        let mut f_for_cr_channel = |block_buffer: &mut Vec<u8>|
            f(&mut self.cr_dct_coeffs, DEFAULT_CH_QUANTIZATION_TABLE, block_buffer);

        thread::scope(|s| {
            let y_handle = s.spawn(|| {
                self.cb_channel.for_each_block(8, 8, true, &mut f_for_y_channel);
            });
            let cb_handle = s.spawn(|| {
                self.cb_channel.for_each_block(8, 8, true, &mut f_for_cb_channel);
            });
            let cr_handle = s.spawn(|| {
                self.cb_channel.for_each_block(8, 8, true, &mut f_for_cr_channel);
            });

            _ = y_handle.join();
            _ = cb_handle.join();
            _ = cr_handle.join();
        });
    }

    fn dct_shift_range(n: u8) -> i8 {
        if n <= 127 { (n | 128u8) as i8 } else { (n & 127u8) as i8 }
    }

    fn forward_bin_dct_and_quant(
        block_buffer: &mut Vec<u8>,
        quantization_table: [i32; 64],
        coeffs_buffer: &mut Vec<i8>
    ) {
        // Version C of the BinDCT in this paper:
        // https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=a16a78322dfdfc8c6ad1a38ba05caafe97a56254
        let mut aux_buffer: [i32; 64] = [0; 64];
        block_buffer
            .iter()
            .enumerate()
            .for_each(|(i, val)| {
                aux_buffer[i] = Self::dct_shift_range(*val) as i32;
            });

        // transform rows
        for i in 0..8 {
            let x0 = aux_buffer[i * 8 + 0];
            let x1 = aux_buffer[i * 8 + 1];
            let x2 = aux_buffer[i * 8 + 2];
            let x3 = aux_buffer[i * 8 + 3];
            let x4 = aux_buffer[i * 8 + 4];
            let x5 = aux_buffer[i * 8 + 5];
            let x6 = aux_buffer[i * 8 + 6];
            let x7 = aux_buffer[i * 8 + 7];

            let mut x0_1 = x0 + x7;
            let mut x1_1 = x1 + x6;
            let mut x2_1 = x2 + x5;
            let mut x3_1 = x3 + x4;
            let mut x4_1 = x3 - x4;
            let mut x5_1 = x2 - x5;
            let mut x6_1 = x6 - x1;
            let mut x7_1 = x0 - x7;

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = -x6_1 + x7_1;
            let mut x7_2 = x7_1 + x6_1;

            x4_2 = x4_2 - (x7_2 >> 3);
            x0_2 = x0_2 + x1_2;
            x1_2 = -x1_2 + (x0_2 >> 1);
            x2_2 = x2_2 - ((x3_2 * 3) >> 3);
            x3_2 = x3_2 + ((x2_2 * 3) >> 3);
            x5_2 = x5_2 + ((x6_2 * 7) >> 3);
            x6_2 = x6_2 - (x5_2 >> 1);

            aux_buffer[i * 8 + 0] = x0_2;
            aux_buffer[i * 8 + 4] = x1_2;
            aux_buffer[i * 8 + 6] = x2_2;
            aux_buffer[i * 8 + 2] = x3_2;
            aux_buffer[i * 8 + 7] = x4_2;
            aux_buffer[i * 8 + 5] = x5_2;
            aux_buffer[i * 8 + 3] = x6_2;
            aux_buffer[i * 8 + 1] = x7_2;
        }

        // transform columns
        for i in 0..8 {
            let x0 = aux_buffer[0 * 8 + i];
            let x1 = aux_buffer[1 * 8 + i];
            let x2 = aux_buffer[2 * 8 + i];
            let x3 = aux_buffer[3 * 8 + i];
            let x4 = aux_buffer[4 * 8 + i];
            let x5 = aux_buffer[5 * 8 + i];
            let x6 = aux_buffer[6 * 8 + i];
            let x7 = aux_buffer[7 * 8 + i];

            let mut x0_1 = x0 + x7;
            let mut x1_1 = x1 + x6;
            let mut x2_1 = x2 + x5;
            let mut x3_1 = x3 + x4;
            let mut x4_1 = x3 - x4;
            let mut x5_1 = x2 - x5;
            let mut x6_1 = x6 - x1;
            let mut x7_1 = x0 - x7;

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = -x6_1 + x7_1;
            let mut x7_2 = x7_1 + x6_1;

            x4_2 = x4_2 - (x7_2 >> 3);
            x0_2 = x0_2 + x1_2;
            x1_2 = -x1_2 + (x0_2 >> 1);
            x2_2 = x2_2 - ((x3_2 * 3) >> 3);
            x3_2 = x3_2 + ((x2_2 * 3) >> 3);
            x5_2 = x5_2 + ((x6_2 * 7) >> 3);
            x6_2 = x6_2 - (x5_2 >> 1);

            aux_buffer[0 * 8 + i] = x0_2;
            aux_buffer[4 * 8 + i] = x1_2;
            aux_buffer[6 * 8 + i] = x2_2;
            aux_buffer[2 * 8 + i] = x3_2;
            aux_buffer[7 * 8 + i] = x4_2;
            aux_buffer[5 * 8 + i] = x5_2;
            aux_buffer[3 * 8 + i] = x6_2;
            aux_buffer[1 * 8 + i] = x7_2;
        }

        for i in 0..64 {
            coeffs_buffer.push((aux_buffer[i] / quantization_table[i]) as i8);
        }
    }

    fn forward_real_dct_and_quant(
        block_buffer: &mut Vec<u8>,
        quantization_table: [i32; 64],
        coeffs_buffer: &mut Vec<i8>
    ) {
        // This code follows the actual DCT mathematical formula.
        // This algorithm is extremely slow due to the cosine calculation and floating point arithmetic
        let inverse_sqrt_two: f32 = 1.0 / SQRT_2;

        let mut quant_idx = 0;

        let mut alpha_u: f32;
        let mut alpha_v: f32;
        let mut sum: f32;
        for u in 0..8 {
            if u == 0 {
                alpha_u = inverse_sqrt_two;
            } else {
                alpha_u = 1.0;
            }
            for v in 0..8 {
                sum = 0.0;
                if v == 0 {
                    alpha_v = inverse_sqrt_two;
                } else {
                    alpha_v = 1.0;
                }

                for x in 0..8 {
                    for y in 0..8 {
                        let block_element = Self::dct_shift_range(block_buffer[x * 8 + y]);
                        sum +=
                            (block_element as f32) *
                            (((((2 * x + 1) * u) as f32) * PI) / 16.0).cos() *
                            (((((2 * y + 1) * v) as f32) * PI) / 16.0).cos();
                    }
                }

                coeffs_buffer.push(
                    ((0.25 * alpha_u * alpha_v * sum) /
                        (quantization_table[quant_idx] as f32)) as i8
                );
                quant_idx += 1;
            }
        }
    }

    fn quantization(
        coeffs_buffer: &[f32; 64],
        quantization_table: [i8; 64],
        dct_coeffs: &mut Vec<i8>
    ) {
        for i in 0..64 {
            dct_coeffs.push((coeffs_buffer[i] / (quantization_table[i] as f32)) as i8);
        }
    }

    fn get_encoded_data(&self) -> BitVec {
        let mut bits: BitVec = BitVec::new();

        // Run length encode

        // Huffman encode

        return bits;
    }

    fn runlength_encode(
        y_channel: &Vec<i8>,
        cb_channel: &Vec<i8>,
        cr_channel: &Vec<i8>,
        result_buffer: &Vec<i8>
    ) {}

    fn huffman_encode(runlength: &Vec<i8>, bits: &mut BitVec) {
        // recordar que cada 4 fin de bloques, cambia el tipo de canal y de tabla
    }

    fn get_huffman_codes(table: &mut HuffmanTable) {
        let mut code: u32 = 0;

        for i in 0..16 {
            for j in i..table.offsets[(i as usize) + 1] {
                table.codes[j as usize] = code;
                code += 1;
            }
            code <<= 1;
        }
        table.set = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_dct_and_quant() {
        let mut result: Vec<i8> = Vec::<i8>::with_capacity(64);

        // example taken from wikipedia JPEG article, DCT section
        #[rustfmt::skip]
        let mut input_block: Vec<u8> = vec![52,55,61,66,70,61,64,73,63,59,55,90,109,85,69,72,62,59,68,113,144,104,66,73,63,58,71,122,154,106,70,69,67,61,68,104,126,88,68,70,79,65,60,70,77,68,58,75,85,71,64,59,55,61,65,83,87,79,69,68,65,76,78,94];
        #[rustfmt::skip]
        let expected: Vec<i8> = vec![-26,-3,-6,2,2,-1,0,0,0,-2,-4,1,1,0,0,0,-3,1,5,-1,-1,0,0,0,-3,1,2,-1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        // there might be differences between the result and the expected output due to rounding.
        // Instead of asserting if result and expected are equal, we will see if they are "close enough"
        // this error might be even higher for the binDct implementation, since it's actually an approximation of the transform
        let delta_error_threshold = 1; // I will supose that, in average, the numbers shouldn't differ by more than 1 from the expected value

        let mut average_error = 0.0;

        JpegImage::forward_real_dct_and_quant(
            &mut input_block,
            DEFAULT_Y_QUANTIZATION_TABLE,
            &mut result
        );

        for i in 0..64 {
            let error: f64 = (result[i] - expected[i]).abs() as f64;
            average_error += error;
        }

        average_error /= 64.0;

        println!("result: {:?}", result);
        println!("expected: {:?}", expected);
        println!("average error: {}", average_error);

        assert!(average_error <= (delta_error_threshold as f64));
    }

    #[test]
    fn test_bin_dct_and_quant() {
        let mut result: Vec<i8> = Vec::<i8>::with_capacity(64);

        // example taken from wikipedia JPEG article, DCT section
        #[rustfmt::skip]
        let mut input_block: Vec<u8> = vec![52,55,61,66,70,61,64,73,63,59,55,90,109,85,69,72,62,59,68,113,144,104,66,73,63,58,71,122,154,106,70,69,67,61,68,104,126,88,68,70,79,65,60,70,77,68,58,75,85,71,64,59,55,61,65,83,87,79,69,68,65,76,78,94];
        #[rustfmt::skip]
        let expected: Vec<i8> = vec![-26,-3,-6,2,2,-1,0,0,0,-2,-4,1,1,0,0,0,-3,1,5,-1,-1,0,0,0,-3,1,2,-1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        // there might be differences between the result and the expected output due to rounding.
        // Instead of asserting if result and expected are equal, we will see if they are "close enough"
        // this error might be even higher for the binDct implementation, since it's actually an approximation of the transform
        let delta_error_threshold = 1;

        let mut average_error = 0.0;

        JpegImage::forward_bin_dct_and_quant(
            &mut input_block,
            DEFAULT_Y_QUANTIZATION_TABLE,
            &mut result
        );

        for i in 0..64 {
            let error: f64 = (result[i] - expected[i]).abs() as f64;
            average_error += error;
        }

        average_error /= 64.0;

        println!("result: {:?}", result);
        println!("expected: {:?}", expected);
        println!("average error: {}", average_error);

        assert!(average_error <= (delta_error_threshold as f64));
    }
}
