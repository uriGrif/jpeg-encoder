use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// the input image path
    #[arg(short, long, required = true)]
    pub image: String,

    /// the output image path (optional)
    #[arg(short, long, default_value_t = String::from(""))]
    pub output: String,
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
        println!("outputs: \"{}\"", self.output);
    }
}
