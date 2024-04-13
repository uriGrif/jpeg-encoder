mod colorspace;
mod arguments;
use arguments::Args;
use image::JpegImage;
mod image;
mod bitvec;

fn main() {
    // parse arguments

    let args: Args = Args::get_args();
    args.print_args();

    // create jpeg image object from bmp file, with color space conversion to ycbcr

    let mut jpeg_image: JpegImage = JpegImage::create_from_bmp(&args.image);

    // Chrominance Downsampling

    jpeg_image.chrominance_downsampling();

    // Discrete Cosine Transform and Quantization

    jpeg_image.dct_and_quant();

    // Run Length and Huffman Encoding

    // write to ouptut file

    // TODO
    // pasar toda la logica de cada paso a una funcion dentro de JpegImage
}
