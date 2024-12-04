mod colorspace;
mod arguments;
use arguments::Args;
use jpeg_image::JpegImage;
mod jpeg_image;
use bitvec;
mod quant_tables;
mod huffman_tables;
mod bmp_image;
mod pixel_matrix;

fn main() {
    // parse arguments

    let args: Args = Args::get_args();
    args.print_args();

    // create jpeg image object from bmp file, with color space conversion to ycbcr

    let mut jpeg_image: JpegImage = JpegImage::from_bmp(&args.image);

    // Chrominance Downsampling

    jpeg_image.chrominance_downsampling();

    // Discrete Cosine Transform and Quantization

    jpeg_image.dct_and_quantization();

    // Run Length and Huffman Encoding

    // write to ouptut file
}
