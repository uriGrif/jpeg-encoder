use crate::JpegImage;
use crate::jpeg::huffman_tables::*;
use std::cell::RefCell;
use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use crate::pixel_matrix::pixel_matrix::PixelMatrix;
use crate::utils::bitvec_utils::write_bits;

struct RunLength {
    symbol: u8,
    amplitude: i16, // coefficient
}

impl JpegImage {
    pub fn generate_entropy_encoded_data(&mut self) {
        initialize_huffman_tables();

        // let bits = RefCell::new(BitVec::new());

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

        let mut aux_buffer = Vec::<i16>::with_capacity(
            8 * horizontal_downsampling * 8 * vertical_downsampling
        );

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
                    &mut self.entropy_coded_bits,
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

        for i in 0..cb_dct_block_iterator.get_blocks_amount() {
            if i != 0 {
                y_dct_block_iterator.increment_block_idx();
                cb_dct_block_iterator.increment_block_idx();
                cr_dct_block_iterator.increment_block_idx();
            }
            y_dct_block_iterator.block_operation(&mut aux_buffer, &mut process_multiple_blocks);
            cb_dct_block_iterator.block_operation(
                &mut aux_buffer,
                &mut (|block_buffer: &mut Vec<i16>|
                    process_single_block.borrow_mut()(
                        block_buffer,
                        get_huffman_table(HuffmanTableType::CHDC),
                        get_huffman_table(HuffmanTableType::CHAC)
                    ))
            );
            cr_dct_block_iterator.block_operation(
                &mut aux_buffer,
                &mut (|block_buffer: &mut Vec<i16>|
                    process_single_block.borrow_mut()(
                        block_buffer,
                        get_huffman_table(HuffmanTableType::CHDC),
                        get_huffman_table(HuffmanTableType::CHAC)
                    ))
            );
        }
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
                zeros_count -= 16;
            }

            let ac_coeff = dct_coeffs[ZIG_ZAG_MAP[i]];
            let ac_bit_length = Self::bit_length(ac_coeff.abs());
            if ac_bit_length > 10 {
                panic!("AC coefficient bit length greater than 10!");
            }
            result_buffer.push(RunLength {
                symbol: Self::get_run_length_symbol(zeros_count, ac_bit_length),
                amplitude: Self::coeff_to_amplitude(ac_coeff, dc_bit_length),
            });
            zeros_count = 0;
            i += 1;
        }
    }

    fn huffman_encode(
        runlength: &Vec<RunLength>,
        bitvec: &mut BitVec<u8, Msb0>,
        dc_huffman_table: &HuffmanTable,
        ac_huffman_table: &HuffmanTable
    ) {
        for (i, r) in runlength.iter().enumerate() {
            if i == 0 {
                // dc coeff
                let (code, code_length) = dc_huffman_table
                    .get_code(r.symbol)
                    .expect("DC Huffman Code Not Found!");
                write_bits(bitvec, code, code_length);
                write_bits(bitvec, r.amplitude as u32, r.symbol & 0x0f);
                continue;
            }

            // ac coeffs
            let (code, code_length) = ac_huffman_table
                .get_code(r.symbol)
                .expect("AC Huffman Code Not Found!");
            write_bits(bitvec, code, code_length);
            if r.symbol != 0xf0 && r.symbol != 0x00 {
                write_bits(bitvec, r.amplitude as u32, r.symbol & 0x0f);
            }
        }
    }
}
