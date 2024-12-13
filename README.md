# JPEG Encoder

![](https://www.eetimes.com/wp-content/uploads/media-1101219-fig1.jpg)

## About the project

This is a simple command line tool that converts BMP image files to JPEG image files.

I made this project because I once learned how the JPEG algorithm worked and thought it was really cool and clever all the things it did to drastically reduce the size of an image, while keeping a good quality.

Moreover, it ended up being a great project to learn to code in the Rust programming language, and learning a whole lot of other interesting stuff, such as Huffman Coding, file headers and much more.

![](https://img.shields.io/badge/rust-orange.svg?style=flat&logo=rust)

## Theory behind the code

If you want to learn how the code and the whole compression process works, you can check out [this document](jpeg_theory.md). It contains everything I learned in this project, with all the best sources I used.

## Getting started

1.  Make sure you have the Rust compiler (You can get it [here](https://www.rust-lang.org))

2.  Clone this repository:

```console
git clone https://github.com/uriGrif/jpeg-encoder.git
```

3. Go inside the project folder and try it out

```console
cd jpeg-encoder
cargo run -- --image <BMP_INPUT_FILE> [OPTIONS]
```

```
OPTIONS:
  -i, --image <IMAGE>
          the input image path
  -o, --output <OUTPUT>
          the output image path (optional) [default: ]
  -s, --subsampling-ratio <SUBSAMPLING_RATIO>
          Subsampling ratio in the format `4:2:0`, `4:4:4`, or `4:2:2` [default: 4:2:0]
  -d, --dct-algorithm <DCT_ALGORITHM>
          DCT algorithm to use: either "RealDct" or "BinDct" [default: real-dct] [possible values: real-dct, bin-dct]
  -h, --help
          Print help
  -V, --version
          Print version
```

4. You can also build the binary and use it anywhere

```console
cargo build --release
```
