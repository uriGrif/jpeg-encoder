use std::f32::consts::{ PI, SQRT_2 };
use std::io::Read;
use std::thread;
use std::{ fs::File, io::Seek };
use std::os::windows::fs::FileExt;
use byteorder::{ ByteOrder, LittleEndian };
use clap::builder::styling::RgbColor;
use crate::bmp_image::{ self, BmpImage };
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };
use crate::bit_vec::BitVec;
use crate::pixel_matrix::PixelMatrix;
use crate::quant_tables::*;
use crate::huffman_tables::*;

const DEFAULT_DOWNSAMPLING_RATIO: (u8, u8, u8) = (4, 2, 0);

pub struct JpegImage {
    path: Option<String>,
    file: Option<File>,
    width: i32,
    height: i32,
    chrominance_downsampling_ratio: (u8, u8, u8),
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

    pub fn chrominance_downsampling(&mut self) {
        let block_width: usize;
        let block_height: usize;
        let new_channel_width: usize;
        let new_channel_height: usize;

        match self.chrominance_downsampling_ratio {
            (4, 4, 4) => {
                // no subsampling to be done
                return;
            }
            (4, 2, 0) => {
                block_width = 2;
                block_height = 2;
                new_channel_width = (self.width / 2 + (self.width % 2)) as usize;
                new_channel_height = (self.height / 2 + (self.height % 2)) as usize;
            }
            (4, 2, 2) => {
                block_width = 2;
                block_height = 1;
                new_channel_width = (self.width / 2 + (self.width % 2)) as usize;
                new_channel_height = self.height as usize;
            }
            _ => {
                panic!("Invalid chrominance downsampling ratio!");
            }
        }

        let mut new_cb = PixelMatrix::<u8>::new(new_channel_width, new_channel_height);
        let mut new_cr = PixelMatrix::<u8>::new(new_channel_width, new_channel_height);

        let mut add_average_cb = |block_buffer: &mut Vec<u8>| {
            new_cb.push_next(
                (block_buffer
                    .iter()
                    .map(|x| *x as usize)
                    .sum::<usize>() / block_buffer.len()) as u8
            );
        };

        let mut add_average_cr = |block_buffer: &mut Vec<u8>| {
            new_cr.push_next(
                (block_buffer
                    .iter()
                    .map(|x| *x as usize)
                    .sum::<usize>() / block_buffer.len()) as u8
            );
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
                Self::block_to_buffer(&mut buffer, row, column, &self.y_channel.pixels);
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

                Self::block_to_buffer(&mut buffer, row, column, &self.cb_channel.pixels);
                // perform DCT
                Self::discrete_cosine_transform(buffer, &mut dct_buffer);
                // perform Quantization
                Self::quantization(&mut dct_buffer, false);
                // add buffer to dct_coeffs Vec
                for i in 0..64 {
                    self.cb_dct_coeffs.push(dct_buffer[i] as i8);
                }

                // Red Chrominance Channel

                Self::block_to_buffer(&mut buffer, row, column, &self.cr_channel.pixels);
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
        // this code follows the mathematical formula. Cool for understanding the process, but I guess there must be a better performant way of doing this
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
