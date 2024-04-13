use std::f32::consts::{ PI, SQRT_2 };
use std::io::Read;
use std::{ fs::File, io::Seek };
use std::os::windows::fs::FileExt;
use byteorder::{ ByteOrder, LittleEndian };
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };
use crate::bitvec::BitVec;

const BMP_IMAGE_DATA_START_OFFSET: u64 = 10;
const BMP_PIXEL_WIDTH_OFFSET: u64 = 18;
const BMP_PIXEL_HEIGHT_OFFSET: u64 = 22;

// TODO
// Organizar esto. Poner en otro archivo?

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

const ZIG_ZAG_MAP: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

struct HuffmanTable<'a> {
    offsets: [u8; 16],
    symbols: &'a [u8],
    codes: &'a mut [u32],
    set: bool,
}

static mut Y_DC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 1, 6, 7, 8, 9, 10, 11, 12, 12, 12, 12, 12, 12, 12, 12],
    symbols: &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b],
    codes: &mut [0; 12],
    set: false,
};

static mut CH_DC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 12, 12, 12, 12],
    symbols: &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b],
    codes: &mut [0; 12],
    set: false,
};

static mut Y_AC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 2, 3, 6, 9, 11, 15, 18, 23, 28, 32, 36, 36, 36, 37, 162],
    symbols: &[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
        0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
        0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
        0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
        0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
        0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
        0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
        0xf9, 0xfa,
    ],
    codes: &mut [0; 176],
    set: false,
};

static mut CH_AC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 2, 3, 5, 9, 13, 16, 20, 27, 32, 36, 40, 40, 41, 43, 162],
    symbols: &[
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
        0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
        0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
        0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
        0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
        0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
        0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
        0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
        0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
        0xf9, 0xfa,
    ],
    codes: &mut [0; 176],
    set: false,
};

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

        // TODO
        // check if file header corresponds to BMP

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
