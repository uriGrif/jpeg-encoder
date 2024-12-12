use clap::ValueEnum;
use crate::JpegImage;
use crate::pixel_matrix::block_iterator::PixelMatrixBlockIterator;
use crate::jpeg::quant_tables::*;
use std::thread;
use std::f32::consts::{ PI, SQRT_2 };

#[derive(Debug, Clone, ValueEnum)]
pub enum DctAlgorithm {
    RealDct,
    BinDct,
}

impl JpegImage {
    pub fn dct_and_quantization(&mut self) {
        let dct_algorithm = match self.dct_algorithm {
            DctAlgorithm::RealDct => Self::forward_real_dct_and_quant,
            DctAlgorithm::BinDct => Self::forward_bin_dct_and_quant,
        };

        let f = |
            block_buffer: &mut Vec<u8>,
            quantization_table: [u8; 64],
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
        quantization_table: [u8; 64],
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

            let x7_1 = x0 - x7;
            let x0_1 = x0 - (x7_1 >> 1);
            let mut x6_1 = x1 - x6;
            let x1_1 = x1 - (x6_1 >> 1);
            let mut x5_1 = x2 - x5;
            let x2_1 = x2 - (x5_1 >> 1);
            let x4_1 = x3 - x4;
            let x3_1 = x3 - (x4_1 >> 1);

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = x7_1 - x6_1;
            let x7_2 = x7_1 + x6_1;

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

            let x7_1 = x0 - x7;
            let x0_1 = x0 - (x7_1 >> 1);
            let mut x6_1 = x1 - x6;
            let x1_1 = x1 - (x6_1 >> 1);
            let mut x5_1 = x2 - x5;
            let x2_1 = x2 - (x5_1 >> 1);
            let x4_1 = x3 - x4;
            let x3_1 = x3 - (x4_1 >> 1);

            x6_1 = ((x5_1 * 3) >> 3) + x6_1;
            x5_1 = ((x6_1 * 5) >> 3) - x5_1;

            let mut x0_2 = x0_1 + x3_1;
            let mut x3_2 = x0_1 - x3_1;
            let mut x1_2 = x1_1 + x2_1;
            let mut x2_2 = x1_1 - x2_1;
            let mut x4_2 = x4_1 + x5_1;
            let mut x5_2 = x4_1 - x5_1;
            let mut x6_2 = x7_1 - x6_1;
            let x7_2 = x7_1 + x6_1;

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
            coeffs_block_iterator.set_next_pixel(
                (aux_buffer[i] / (quantization_table[i] as i32)) as i16
            );
        }
    }

    fn forward_real_dct_and_quant(
        block_buffer: &mut Vec<u8>,
        quantization_table: [u8; 64],
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel_matrix::pixel_matrix::PixelMatrix;

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
