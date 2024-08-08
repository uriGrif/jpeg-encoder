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

    pub fn dct_and_quant(&mut self) {
        // TODO
        // - mucho codigo repetido -> ordenar
        // - procesar canales en paralelo

        let mut y_block_idx: usize = 0;
        let mut cb_block_idx: usize = 0;
        let mut cr_block_idx: usize = 0;

        let f = |
            dct_coeffs: &mut Vec<i8>,
            quantization_table: [i8; 64],
            block_idx: &mut usize,
            block_buffer: &mut Vec<u8>
        | {
            // get pixels in block and push the results of the calculation to the dct_coeffs vec
            match self.dct_algorithm {
                DctAlgorithm::RealDct => Self::forward_real_dct(block_buffer, dct_coeffs),
                DctAlgorithm::BinDct => Self::forward_bin_dct(block_buffer, dct_coeffs),
            }
            // perform quantization on the dct_coeffs the where just calculated
            Self::quantization(&mut dct_coeffs[*block_idx..*block_idx + 64], quantization_table);
            // increment block_idx for next quantization
            *block_idx += 64;
        };

        let mut f_for_y_channel = |block_buffer: &mut Vec<u8>|
            f(&mut self.y_dct_coeffs, DEFAULT_Y_QUANTIZATION_TABLE, &mut y_block_idx, block_buffer);

        let mut f_for_cb_channel = |block_buffer: &mut Vec<u8>|
            f(
                &mut self.cb_dct_coeffs,
                DEFAULT_CH_QUANTIZATION_TABLE,
                &mut cb_block_idx,
                block_buffer
            );

        let mut f_for_cr_channel = |block_buffer: &mut Vec<u8>|
            f(
                &mut self.cr_dct_coeffs,
                DEFAULT_CH_QUANTIZATION_TABLE,
                &mut cr_block_idx,
                block_buffer
            );

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

    fn forward_bin_dct(block_buffer: &mut Vec<u8>, dct_coeffs: &mut Vec<i8>) {
        // TODO hacer algoritmo y que vaya pusheando a dct_coeffs. Hacer shift_range antes de calcular
    }

    fn forward_real_dct(block_buffer: &mut Vec<u8>, dct_coeffs: &mut Vec<i8>) {
        // this code follows the actual DCT mathematical formula.
        // This algorithm is extremely slow due to the cosine calculation and floating point arithmetic

        // TODO: hacer el shift_range antes de calcular

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
                            (block_buffer[x * 8 + y] as f32) *
                            ((((2 * x + 1) as f32) * (u as f32) * PI) / 16.0).cos() *
                            ((((2 * y + 1) as f32) * (v as f32) * PI) / 16.0).cos();
                    }
                }

                // result[u * 8 + v] = 0.25 * alpha_u * alpha_v * sum;
                // TODO: deberia hacer dct_coeffs.push(lo que ahora figura en result[...]) --> ver bien como lo pusheo por el indice
            }
        }
    }

    fn quantization(coeffs_block: &mut [i8], quantization_table: [i8; 64]) {
        for i in 0..64 {
            coeffs_block[i] /= quantization_table[i];
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
