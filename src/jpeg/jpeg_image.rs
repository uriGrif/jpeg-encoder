use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use crate::jpeg::dct_quant::DctAlgorithm;
use crate::pixel_matrix::pixel_matrix::PixelMatrix;
use crate::jpeg::sampling::DEFAULT_DOWNSAMPLING_RATIO;
use crate::bmp::bmp_image::BmpImage;
use crate::utils::colorspace::{ RGBValue, YCbCrValue, rgb_to_ycbcr };

pub struct JpegImage {
    pub path: String,
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
    pub entropy_coded_bits: BitVec<u8, Msb0>,
}

impl JpegImage {
    pub fn new(
        path: String,
        width: i32,
        height: i32,
        chrominance_downsampling_ratio: (u8, u8, u8),
        dct_algorithm: DctAlgorithm
    ) -> JpegImage {
        let (horizontal_downsampling, vertical_downsampling): (
            usize,
            usize,
        ) = Self::get_downsampling_factor(chrominance_downsampling_ratio);

        // account for padding, as dct works in 8x8 blocks
        let aux_block_width = 8 * (horizontal_downsampling as i32);
        let padded_width = if width % aux_block_width == 0 {
            width as usize
        } else {
            (width + aux_block_width - (width % aux_block_width)) as usize
        };

        let aux_block_height = 8 * (vertical_downsampling as i32);
        let padded_height = if height % aux_block_height == 0 {
            height as usize
        } else {
            (height + aux_block_height - (height % aux_block_height)) as usize
        };

        let (downsampled_width, downsampled_height) = Self::get_downsampled_dimensions(
            width as usize,
            height as usize,
            horizontal_downsampling,
            vertical_downsampling
        );

        // initialize channels matrixes
        let y_channel = PixelMatrix::new_with_default(
            padded_width as usize,
            padded_height as usize
        );
        let cb_channel = PixelMatrix::new_with_default(
            padded_width as usize,
            padded_height as usize
        );
        let cr_channel = PixelMatrix::new_with_default(
            padded_width as usize,
            padded_height as usize
        );

        // initialize dct coefficients matrixes
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
            entropy_coded_bits: BitVec::new(),
        };

        image
    }

    pub fn from_bmp(bmp_path: &String, jpeg_path: &String) -> JpegImage {
        let mut bmp_image: BmpImage = BmpImage::new(bmp_path);
        bmp_image.load_pixels();

        let mut image = JpegImage::new(
            jpeg_path.clone(),
            bmp_image.width,
            bmp_image.height,
            DEFAULT_DOWNSAMPLING_RATIO,
            DctAlgorithm::RealDct
        );

        for i in 0..bmp_image.height as usize {
            for j in 0..bmp_image.width as usize {
                match bmp_image.pixels.get_pixel(i, j) {
                    Some(rgb_pixel) => {
                        let ycbcr: YCbCrValue = rgb_to_ycbcr(rgb_pixel);

                        image.y_channel.set_pixel(i, j, ycbcr.0);
                        image.cb_channel.set_pixel(i, j, ycbcr.1);
                        image.cr_channel.set_pixel(i, j, ycbcr.2);
                    }
                    _ => {}
                }
            }
        }

        image
    }
}
