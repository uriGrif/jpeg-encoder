use std::io::Read;
use std::{ fs::File, io::Seek };
use std::os::windows::fs::FileExt;
use byteorder::{ ByteOrder, LittleEndian };
use crate::colorspace::{ rgb_to_ycbcr, RGBValue, YCbCrValue };

const BMP_IMAGE_DATA_START_OFFSET: u64 = 10;
const BMP_PIXEL_WIDTH_OFFSET: u64 = 18;
const BMP_PIXEL_HEIGHT_OFFSET: u64 = 22;

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

        let y_dct_coeffs: Vec<i8> = vec![0i8; dct_len];
        let cb_dct_coeffs: Vec<i8> = vec![0i8; dct_len / 2]; // smaller, because these channels will later be downsampled
        let cr_dct_coeffs: Vec<i8> = vec![0i8; dct_len / 2];

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

        //TODO

        self.cb_channel = new_cb;
        self.cr_channel = new_cr;
    }
}
