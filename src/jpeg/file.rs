use std::{ fs::File, io::{ self, Write } };
use byteorder::{ BigEndian, WriteBytesExt };
use super::{
    huffman_tables::{ get_huffman_table, HuffmanTable, HuffmanTableType, ZIG_ZAG_MAP },
    jpeg_image::JpegImage,
    quant_tables::{ DEFAULT_CH_QUANTIZATION_TABLE, DEFAULT_Y_QUANTIZATION_TABLE },
};

impl JpegImage {
    fn write_soi(file: &mut File) -> io::Result<usize> {
        file.write(&[0xff, 0xd8])
    }

    fn write_app0(file: &mut File) -> io::Result<usize> {
        file.write(&[0xff, 0xe0])?;
        file.write_u16::<BigEndian>(16)?; // length
        file.write(b"JFIF\0")?;
        file.write(&[1, 1])?; // version
        file.write(&[1])?; // units
        file.write_u16::<BigEndian>(72)?; // density
        file.write_u16::<BigEndian>(72)?; // density
        file.write(&[0, 0]) // thumbnail
    }

    fn write_quantization_tables(
        file: &mut File,
        destination: u8, // 0 for luminance, 1 for chrominance
        table: &[u8; 64]
    ) -> io::Result<usize> {
        file.write(&[0xff, 0xdb])?;
        file.write_u16::<BigEndian>(67)?; // length
        file.write(&[destination])?;
        for i in 0..64 {
            file.write_u8(table[ZIG_ZAG_MAP[i]])?;
        }
        Ok(69)
    }

    fn write_start_of_frame(&self, file: &mut File) -> io::Result<usize> {
        file.write(&[0xff, 0xc0])?;
        file.write_u16::<BigEndian>(17)?; // length
        file.write_u8(8)?; // precision
        file.write_u16::<BigEndian>(self.height as u16)?;
        file.write_u16::<BigEndian>(self.width as u16)?;
        file.write_u8(3)?; // components
        for i in 1..4 {
            file.write_u8(i)?;
            let (h, v) = Self::get_downsampling_factor(self.chrominance_downsampling_ratio);
            let sampling_factor: u8 = if i == 1 { ((h as u8) << 4) | (v as u8) } else { 0x11 };
            file.write_u8(sampling_factor)?;
            file.write_u8(if i == 1 { 0 } else { 1 })?; // quant table
        }
        Ok(19)
    }

    fn write_huffman_table(
        file: &mut File,
        coeff_type: u8, // 0 for DC - 1 for AC
        table_id: u8, // 0 for Y - 1 for Ch
        table: &HuffmanTable
    ) -> io::Result<usize> {
        file.write(&[0xff, 0xc4])?;
        file.write_u16::<BigEndian>(19 + (table.offsets[16] as u16))?;
        file.write_u8((coeff_type << 4) | table_id)?;
        for i in 0..16 {
            file.write_u8(table.offsets[i + 1] - table.offsets[i])?;
        }
        for i in 0..16 {
            for j in table.offsets[i] as usize..table.offsets[i + 1] as usize {
                file.write_u8(table.symbols[j])?;
            }
        }

        Ok(1)
    }

    fn write_start_of_scan(file: &mut File) -> io::Result<usize> {
        file.write(&[0xff, 0xda])?;
        file.write_u16::<BigEndian>(12)?; // length
        file.write_u8(3)?; // components
        for i in 1..4 {
            file.write_u8(i)?;
            file.write_u8(if i == 1 { 0 } else { 0x11 })?; // dc, ac table
        }
        file.write_u8(0)?;
        file.write_u8(63)?;
        file.write_u8(0)?;

        Ok(14)
    }

    fn write_image_data(&self, file: &mut File) -> io::Result<usize> {
        self.entropy_coded_bits
            .as_raw_slice()
            .into_iter()
            .for_each(|byte| {
                file.write_u8(*byte);
                if *byte == 0xff {
                    file.write_u8(0); // escape possible marker
                }
            });
        Ok(1)
    }

    pub fn generate_file(&self) -> std::io::Result<()> {
        let mut file = File::create(&self.path)?;

        // START OF IMAGE
        Self::write_soi(&mut file)?;

        // APP0
        Self::write_app0(&mut file)?;

        // QUANTIZATION TABLES
        Self::write_quantization_tables(&mut file, 0, &DEFAULT_Y_QUANTIZATION_TABLE)?;
        Self::write_quantization_tables(&mut file, 1, &DEFAULT_CH_QUANTIZATION_TABLE)?;

        // START OF FRAME
        self.write_start_of_frame(&mut file)?;

        // DEFINE HUFFMAN TABLES
        Self::write_huffman_table(&mut file, 0, 0, get_huffman_table(HuffmanTableType::YDC))?;
        Self::write_huffman_table(&mut file, 0, 1, get_huffman_table(HuffmanTableType::CHDC))?;
        Self::write_huffman_table(&mut file, 1, 0, get_huffman_table(HuffmanTableType::YAC))?;
        Self::write_huffman_table(&mut file, 1, 1, get_huffman_table(HuffmanTableType::CHAC))?;

        // START OF SCAN
        Self::write_start_of_scan(&mut file)?;

        // IMAGE DATA
        self.write_image_data(&mut file)?;

        // END OF IMAGE
        file.write(&[0xff, 0xd9])?;

        return Ok(());
    }
}
