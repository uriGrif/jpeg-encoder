use crate::jpeg::dct_quant::DctAlgorithm;
use crate::pixel_matrix::pixel_matrix::PixelMatrix;
use std::fs::File;
use crate::jpeg::sampling::DEFAULT_DOWNSAMPLING_RATIO;
use crate::bmp::bmp_image::BmpImage;
use crate::utils::colorspace::{ RGBValue, YCbCrValue, rgb_to_ycbcr };

pub struct JpegImage {
    pub path: Option<String>,
    pub file: Option<File>,
    pub width: i32,
    pub height: i32,
    pub chrominance_downsampling_ratio: (u8, u8, u8),
    pub dct_algorithm: DctAlgorithm,
    pub y_channel: PixelMatrix<u8>,
    pub cb_channel: PixelMatrix<u8>,
    pub cr_channel: PixelMatrix<u8>,
    pub y_dct_coeffs: PixelMatrix<i16>,
    pub cb_dct_coeffs: PixelMatrix<i16>,
    pub cr_dct_coeffs: PixelMatrix<i16>,
}

impl JpegImage {
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

        let padded_width = (width + 8 - (width % 8)) as usize; // account for padding, as dct works in 8x8 blocks
        let padded_height = (height + 8 - (height % 8)) as usize;

        let downsampled_width = padded_width / horizontal_downsampling;
        let downsampled_height = padded_height / vertical_downsampling;

        let y_dct_coeffs: PixelMatrix<i16> = PixelMatrix::<i16>::new_with_default(
            padded_width,
            padded_height
        );
        let cb_dct_coeffs: PixelMatrix<i16> = PixelMatrix::<i16>::new_with_default(
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
}
