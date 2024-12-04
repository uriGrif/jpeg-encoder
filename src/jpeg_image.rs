use std::cell::RefCell;
use std::f32::consts::{ PI, SQRT_2 };
use std::thread;
use std::fs::File;
use bitvec::order::Msb0;
use bitvec::ptr::write_bits;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use crate::bmp_image::BmpImage;
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };
use crate::pixel_matrix::{ PixelMatrix, PixelMatrixBlockIterator };
use crate::quant_tables::*;
use crate::huffman_tables::*;

const DEFAULT_DOWNSAMPLING_RATIO: (u8, u8, u8) = (4, 2, 0);

pub enum DctAlgorithm {
    RealDct,
    BinDct,
}

struct RunLength {
    symbol: u8,
    amplitude: i16, // coefficient
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
    y_dct_coeffs: PixelMatrix<i16>,
    cb_dct_coeffs: PixelMatrix<i16>,
    cr_dct_coeffs: PixelMatrix<i16>,
}

impl JpegImage {
    // TODO: separar los procesos en sus propios archivos (dsps se pueden poner los procesos inversos en c/u)

    pub fn new(
        path: Option<String>,
        file: Option<File>,
        width: i32,
        height: i32,
        chrominance_downsampling_ratio: (u8, u8, u8),
        dct_algorithm: DctAlgorithm
    ) -> JpegImage {
        // initialize channels matrixes
        let y_channel = PixelMatrix::new(width as usize, height as usize);
        let cb_channel = PixelMatrix::new(width as usize, height as usize);
        let cr_channel = PixelMatrix::new(width as usize, height as usize);

        // initialize dct coefficients matrixes
        let (horizontal_downsampling, vertical_downsampling): (
            usize,
            usize,
        ) = Self::get_downsampling_factor(DEFAULT_DOWNSAMPLING_RATIO);

        let padded_width = (width + (width % 8)) as usize; // account for padding, as dct works in 8x8 blocks
        let padded_height = (height + (height % 8)) as usize;

        let downsampled_width = padded_width / horizontal_downsampling;
        let downsampled_height = padded_height / vertical_downsampling;

        let y_dct_coeffs: PixelMatrix<i16> = PixelMatrix::<i16>::new_with_default(
            padded_width,
            padded_height
        );
        let mut cb_dct_coeffs: PixelMatrix<i16> = PixelMatrix::<i16>::new_with_default(
            downsampled_width,
            downsampled_height
        );
        let cr_dct_coeffs: PixelMatrix<i16> = PixelMatrix::<i16>::new_with_default(
            downsampled_width,
            downsampled_height
        );

        let image: JpegImage = JpegImage {
            file,
            path,
            width: width,
            height: height,
            chrominance_downsampling_ratio,
            dct_algorithm,
            y_channel,
            cb_channel,
            cr_channel,
            y_dct_coeffs,
            cb_dct_coeffs,
            cr_dct_coeffs,
        };

        image
    }

    pub fn from_bmp(bmp_path: &String) -> JpegImage {
        let mut bmp_image: BmpImage = BmpImage::new(bmp_path);
        bmp_image.load_pixels();

        let mut image = JpegImage::new(
            None,
            None,
            bmp_image.width,
            bmp_image.height,
            DEFAULT_DOWNSAMPLING_RATIO,
            DctAlgorithm::BinDct
        );

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

    fn get_downsampling_factor(downsampling_ratio: (u8, u8, u8)) -> (usize, usize) {
        // returns the horizontal and vertical factors by which the chrominance channels must be downsampled
        match downsampling_ratio {
            (4, 4, 4) => {
                return (1, 1);
            }
            (4, 2, 0) => {
                return (2, 2);
            }
            (4, 2, 2) => {
                return (2, 1);
            }
            _ => {
                panic!("Invalid chrominance downsampling ratio!");
            }
        }
    }

    pub fn chrominance_downsampling(&mut self) {
        let (horizontal_downsampling, vertical_downsampling): (
            usize,
            usize,
        ) = Self::get_downsampling_factor(self.chrominance_downsampling_ratio);

        if horizontal_downsampling == 1 && vertical_downsampling == 1 {
            return;
        }

        let new_channel_width = (self.width as usize).div_ceil(horizontal_downsampling);
        let new_channel_height = (self.height as usize).div_ceil(vertical_downsampling);

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
                self.cb_channel
                    .get_block_iterator(horizontal_downsampling, vertical_downsampling, false)
                    .for_each_block(&mut add_average_cb);
            });

            let cr_handle = s.spawn(|| {
                self.cr_channel
                    .get_block_iterator(horizontal_downsampling, vertical_downsampling, false)
                    .for_each_block(&mut add_average_cr);
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
            block_buffer: &mut Vec<u8>,
            quantization_table: [i32; 64],
            dct_coeffs_iterator: &mut PixelMatrixBlockIterator<i16>
        | {
            dct_algorithm(block_buffer, quantization_table, dct_coeffs_iterator)
        };

        thread::scope(|s| {
            let y_handle = s.spawn(|| {
                let mut channel_iterator = self.y_channel.get_block_iterator(8, 8, true);
                let mut coeffs_block_iterator = self.y_dct_coeffs.get_block_iterator(8, 8, true);
                channel_iterator.for_each_block(
                    &mut (|block_buffer: &mut Vec<u8>|
                        f(block_buffer, DEFAULT_Y_QUANTIZATION_TABLE, &mut coeffs_block_iterator))
                );
            });

            let cb_handle = s.spawn(|| {
                let mut channel_iterator = self.cb_channel.get_block_iterator(8, 8, true);
                let mut coeffs_block_iterator = self.cb_dct_coeffs.get_block_iterator(8, 8, true);
                channel_iterator.for_each_block(
                    &mut (|block_buffer: &mut Vec<u8>|
                        f(block_buffer, DEFAULT_CH_QUANTIZATION_TABLE, &mut coeffs_block_iterator))
                );
            });

            let cr_handle = s.spawn(|| {
                let mut channel_iterator = self.cr_channel.get_block_iterator(8, 8, true);
                let mut coeffs_block_iterator = self.cr_dct_coeffs.get_block_iterator(8, 8, true);
                channel_iterator.for_each_block(
                    &mut (|block_buffer: &mut Vec<u8>|
                        f(block_buffer, DEFAULT_CH_QUANTIZATION_TABLE, &mut coeffs_block_iterator))
                );
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
        coeffs_block_iterator: &mut PixelMatrixBlockIterator<i16>
    ) {
        // Version "all-lifting binDCT-C" of this paper:
        // https://thanglong.ece.jhu.edu/Tran/Pub/intDCT.pdf
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

            let mut x7_1 = x0 - x7;
            let mut x0_1 = x0 - (x7_1 >> 1);
            let mut x6_1 = x1 - x6;
            let mut x1_1 = x1 - (x6_1 >> 1);
            let mut x5_1 = x2 - x5;
            let mut x2_1 = x2 - (x5_1 >> 1);
            let mut x4_1 = x3 - x4;
            let mut x3_1 = x3 - (x4_1 >> 1);

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = x7_1 - x6_1;
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

            let mut x7_1 = x0 - x7;
            let mut x0_1 = x0 - (x7_1 >> 1);
            let mut x6_1 = x1 - x6;
            let mut x1_1 = x1 - (x6_1 >> 1);
            let mut x5_1 = x2 - x5;
            let mut x2_1 = x2 - (x5_1 >> 1);
            let mut x4_1 = x3 - x4;
            let mut x3_1 = x3 - (x4_1 >> 1);

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = x7_1 - x6_1;
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
            coeffs_block_iterator.set_next_pixel((aux_buffer[i] / quantization_table[i]) as i16);
        }
    }

    fn forward_real_dct_and_quant(
        block_buffer: &mut Vec<u8>,
        quantization_table: [i32; 64],
        coeffs_block_iterator: &mut PixelMatrixBlockIterator<i16>
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

                coeffs_block_iterator.set_next_pixel(
                    ((0.25 * alpha_u * alpha_v * sum) /
                        (quantization_table[quant_idx] as f32)) as i16
                );
                quant_idx += 1;
            }
        }
    }

    fn get_entropy_encoded_data(&mut self) -> BitVec {
        initialize_huffman_tables();

        let bits = RefCell::new(BitVec::new());

        let (horizontal_downsampling, vertical_downsampling): (
            usize,
            usize,
        ) = Self::get_downsampling_factor(self.chrominance_downsampling_ratio);

        let mut y_dct_block_iterator = self.y_dct_coeffs.get_block_iterator(
            8 * horizontal_downsampling,
            8 * vertical_downsampling,
            false
        );

        let mut cb_dct_block_iterator = self.cb_dct_coeffs.get_block_iterator(8, 8, false);

        let mut cr_dct_block_iterator = self.cr_dct_coeffs.get_block_iterator(8, 8, false);

        let mut run_length_result_buffer = Vec::<RunLength>::with_capacity(64);

        let mut prev_dc_coeff = 0i16;

        let process_single_block = RefCell::new(
            |
                block_buffer: &mut Vec<i16>,
                dc_huffman_table: &HuffmanTable,
                ac_huffman_table: &HuffmanTable
            | {
                run_length_result_buffer.clear();
                JpegImage::runlength_encode(
                    &mut prev_dc_coeff,
                    &block_buffer,
                    &mut run_length_result_buffer
                );
                JpegImage::huffman_encode(
                    &run_length_result_buffer,
                    &mut bits.borrow_mut(),
                    dc_huffman_table,
                    ac_huffman_table
                );
            }
        );

        // this is used to process the luminance blocks, which may be more than the chrominance blocks, due to the downsampling
        let mut process_multiple_blocks = |block_buffer: &mut Vec<i16>| {
            // I couldn't find a good way to avoid the memory duplication of using to_owned()
            // without using unsafe rust or some really weird Arc<Mutex<>>
            // while at the same time trying to keep the block_iterator interface
            let mut new_pixel_matrix = PixelMatrix::new_from_pixels(
                8 * horizontal_downsampling,
                8 * vertical_downsampling,
                block_buffer.to_owned()
            );

            new_pixel_matrix
                .get_block_iterator(8, 8, false)
                .for_each_block(
                    &mut (|block_buffer: &mut Vec<i16>|
                        process_single_block.borrow_mut()(
                            block_buffer,
                            get_huffman_table(HuffmanTableType::YDC),
                            get_huffman_table(HuffmanTableType::YAC)
                        ))
                );
        };

        for _ in 0..cb_dct_block_iterator.get_blocks_amount() {
            y_dct_block_iterator.for_each_block(&mut process_multiple_blocks);
            cb_dct_block_iterator.for_each_block(
                &mut (|block_buffer: &mut Vec<i16>|
                    process_single_block.borrow_mut()(
                        block_buffer,
                        get_huffman_table(HuffmanTableType::CHDC),
                        get_huffman_table(HuffmanTableType::CHAC)
                    ))
            );
            cr_dct_block_iterator.for_each_block(
                &mut (|block_buffer: &mut Vec<i16>|
                    process_single_block.borrow_mut()(
                        block_buffer,
                        get_huffman_table(HuffmanTableType::CHDC),
                        get_huffman_table(HuffmanTableType::CHAC)
                    ))
            );
        }

        return bits.into_inner();
    }

    fn bit_length(mut value: i16) -> u8 {
        let mut length: u8 = 0;
        while value > 0 {
            value >>= 1;
            length += 1;
        }
        return length;
    }

    fn get_run_length_symbol(zeros_count: u8, bit_length: u8) -> u8 {
        ((zeros_count & 0xf0) << 4) | (bit_length & 0x0f)
    }

    fn coeff_to_amplitude(value: i16, bit_length: u8) -> i16 {
        if value < 0 { value + (1 << bit_length) - 1 } else { value }
    }

    fn runlength_encode(
        prev_dc_coeff: &mut i16,
        dct_coeffs: &Vec<i16>,
        result_buffer: &mut Vec<RunLength>
    ) {
        let dc_bit_length = Self::bit_length(dct_coeffs[0].abs());
        if dc_bit_length > 11 {
            panic!("DC coefficient bit length greater than 11!");
        }
        let dc_coeff = dct_coeffs[0] - *prev_dc_coeff;
        // handle DC coefficient
        result_buffer.push(RunLength {
            symbol: Self::get_run_length_symbol(0, dc_bit_length),
            amplitude: Self::coeff_to_amplitude(dc_coeff, dc_bit_length),
        });
        *prev_dc_coeff = dct_coeffs[0];

        // handle AC coefficients
        let mut zeros_count = 0u8;
        let mut i: usize = 1;
        while i < 64 {
            while i < 64 && dct_coeffs[ZIG_ZAG_MAP[i]] == 0 {
                zeros_count += 1;
                i += 1;
            }

            if i == 64 {
                result_buffer.push(RunLength {
                    symbol: 0x00, // End Of Block
                    amplitude: 0,
                });
                break;
            }

            while zeros_count >= 16 {
                result_buffer.push(RunLength {
                    symbol: 0xf0,
                    amplitude: 0,
                });
                i -= 16;
            }

            let ac_coeff = dct_coeffs[ZIG_ZAG_MAP[i]];
            let ac_bit_length = Self::get_run_length_symbol(
                zeros_count,
                Self::bit_length(ac_coeff.abs())
            );
            if ac_bit_length > 10 {
                panic!("AC coefficient bit length greater than 10!");
            }
            result_buffer.push(RunLength {
                symbol: ac_bit_length,
                amplitude: Self::coeff_to_amplitude(ac_coeff, dc_bit_length),
            });
            zeros_count = 0;
            i += 1;
        }
    }

    fn huffman_encode(
        runlength: &Vec<RunLength>,
        bitvec: &mut BitVec,
        dc_huffman_table: &HuffmanTable,
        ac_huffman_table: &HuffmanTable
    ) {
        // recordar que cada 4 fin de bloques, cambia el tipo de canal y de tabla
        for (i, r) in runlength.iter().enumerate() {
            if i == 0 {
                // dc coeff
                let (code, code_length) = dc_huffman_table
                    .get_code(r.symbol)
                    .expect("DC Huffman Code Not Found!");
                Self::write_bits(bitvec, code, code_length);
                Self::write_bits(bitvec, r.amplitude as u32, r.symbol & 0x0f);
                continue;
            }

            // ac coeffs
            let (code, code_length) = ac_huffman_table
                .get_code(r.symbol)
                .expect("AC Huffman Code Not Found!");
            Self::write_bits(bitvec, code, code_length);
            if r.symbol != 0xf0 && r.symbol != 0x00 {
                Self::write_bits(bitvec, r.amplitude as u32, r.symbol & 0x0f);
            }
        }
    }

    fn write_bits(bitvec: &mut BitVec, value: u32, length: u8) {
        for i in (0..length).rev() {
            let bit = (value >> i) & 1; // get the i-th bit
            bitvec.push(bit != 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_dct_and_quant() {
        let mut result = PixelMatrix::<i16>::new_with_default(8, 8);

        // example taken from wikipedia JPEG article, DCT section
        #[rustfmt::skip]
        let mut input_block: Vec<u8> = vec![52,55,61,66,70,61,64,73,63,59,55,90,109,85,69,72,62,59,68,113,144,104,66,73,63,58,71,122,154,106,70,69,67,61,68,104,126,88,68,70,79,65,60,70,77,68,58,75,85,71,64,59,55,61,65,83,87,79,69,68,65,76,78,94];
        #[rustfmt::skip]
        let expected: Vec<i16> = vec![-26,-3,-6,2,2,-1,0,0,0,-2,-4,1,1,0,0,0,-3,1,5,-1,-1,0,0,0,-3,1,2,-1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        // there might be differences between the result and the expected output due to rounding.
        // Instead of asserting if result and expected are equal, we will see if they are "close enough"
        // this error might be even higher for the binDct implementation, since it's actually an approximation of the transform
        let delta_error_threshold = 1; // I will supose that, in average, the numbers shouldn't differ by more than 1 from the expected value

        let mut average_error = 0.0;

        let mut block_iterator = result.get_block_iterator(8, 8, true);

        JpegImage::forward_real_dct_and_quant(
            &mut input_block,
            DEFAULT_Y_QUANTIZATION_TABLE,
            &mut block_iterator
        );

        block_iterator.reset();

        for i in 0..64 {
            let error: f64 = (block_iterator.get_next_pixel().unwrap() - expected[i]).abs() as f64;
            average_error += error;
        }

        average_error /= 64.0;

        println!("result: {:?}", result.pixels);
        println!("expected: {:?}", expected);
        println!("average error: {}", average_error);

        assert!(average_error <= (delta_error_threshold as f64));
    }

    #[test]
    fn test_bin_dct_and_quant() {
        let mut result = PixelMatrix::<i16>::new_with_default(8, 8);

        // example taken from wikipedia JPEG article, DCT section
        #[rustfmt::skip]
        let mut input_block: Vec<u8> = vec![52,55,61,66,70,61,64,73,63,59,55,90,109,85,69,72,62,59,68,113,144,104,66,73,63,58,71,122,154,106,70,69,67,61,68,104,126,88,68,70,79,65,60,70,77,68,58,75,85,71,64,59,55,61,65,83,87,79,69,68,65,76,78,94];
        #[rustfmt::skip]
        let expected: Vec<i16> = vec![-26,-3,-6,2,2,-1,0,0,0,-2,-4,1,1,0,0,0,-3,1,5,-1,-1,0,0,0,-3,1,2,-1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        // there might be differences between the result and the expected output due to rounding.
        // Instead of asserting if result and expected are equal, we will see if they are "close enough"
        // this error might be even higher for the binDct implementation, since it's actually an approximation of the transform
        let delta_error_threshold = 1;

        let mut average_error = 0.0;

        let mut block_iterator = result.get_block_iterator(8, 8, true);

        JpegImage::forward_bin_dct_and_quant(
            &mut input_block,
            DEFAULT_Y_QUANTIZATION_TABLE,
            &mut block_iterator
        );

        block_iterator.reset();

        for i in 0..64 {
            let error: f64 = (block_iterator.get_next_pixel().unwrap() - expected[i]).abs() as f64;
            average_error += error;
        }

        average_error /= 64.0;

        println!("result: {:?}", result.pixels);
        println!("expected: {:?}", expected);
        println!("average error: {}", average_error);

        assert!(average_error <= (delta_error_threshold as f64));
    }
}
