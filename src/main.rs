mod utils;
use crate::utils::arguments::Args;
mod jpeg;
use jpeg::jpeg_image::JpegImage;
mod bmp;
mod pixel_matrix;

fn main() {
    // parse arguments

    let args: Args = Args::get_args();
    args.print_args();

    // create jpeg image object from bmp file, with color space conversion to ycbcr

    println!("Loading bmp");
    let mut jpeg_image: JpegImage = JpegImage::from_bmp(&args.image, &args.output);

    // Chrominance Downsampling

    println!("Chrominance downsampling");
    jpeg_image.chrominance_downsampling();

    // Discrete Cosine Transform and Quantization

    println!("DCT + quant");
    jpeg_image.dct_and_quantization();

    // Run Length and Huffman Encoding

    println!("Entropy coding");
    jpeg_image.generate_entropy_encoded_data();

    // write to ouptut file
    println!("Creating file");
    jpeg_image.generate_file().unwrap();
}
