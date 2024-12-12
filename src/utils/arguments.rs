use clap::Parser;
use crate::jpeg::{ dct_quant::DctAlgorithm, jpeg_image::JpegImage };

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// the input image path
    #[arg(short, long, required = true)]
    pub image: String,

    /// the output image path (optional)
    #[arg(short, long, default_value_t = String::new())]
    pub output: String,

    /// Subsampling ratio in the format `4:2:0`, `4:4:4`, or `4:2:2`
    #[arg(short, long, value_parser = parse_subsampling_ratio, default_value = "4:2:0")]
    pub subsampling_ratio: (u8, u8, u8),

    /// DCT algorithm to use: either "RealDct" or "BinDct"
    #[arg(short, long, value_enum, default_value_t = DctAlgorithm::RealDct)]
    pub dct_algorithm: DctAlgorithm,
}

// Custom parser for subsampling ratio
fn parse_subsampling_ratio(s: &str) -> Result<(u8, u8, u8), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err("Subsampling ratio must be in the format A:B:C".to_string());
    }
    let parsed: Result<Vec<u8>, _> = parts
        .iter()
        .map(|&part| part.parse::<u8>())
        .collect();
    match parsed {
        Ok(values) if values.len() == 3 => {
            let result = (values[0], values[1], values[2]);
            _ = JpegImage::get_downsampling_factor(result); // will panic if incorrect
            Ok(result)
        }
        _ =>
            Err("Subsampling ratio must consist of three integers separated by colons".to_string()),
    }
}

impl Args {
    pub fn get_args() -> Args {
        let mut args = Args::parse();

        if !args.image.ends_with(".bmp") {
            panic!("Input image must be a .bmp file\n");
        }

        if args.output.is_empty() {
            args.output = format!("{}.jpeg", args.image.strip_suffix(".bmp").unwrap());
        }

        args
    }

    pub fn print_args(&self) {
        println!("image: \"{}\"", self.image);
        println!("output: \"{}\"", self.output);
        println!("subsampling ratio: \"{:?}\"", self.subsampling_ratio);
        println!("dct algorithm: \"{:?}\"", self.dct_algorithm);
        print!("\n");
    }
}
