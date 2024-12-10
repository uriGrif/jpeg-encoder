pub enum HuffmanTableType {
    YDC,
    CHDC,
    YAC,
    CHAC,
}

pub struct HuffmanTable<'a> {
    pub offsets: [u8; 17], // these are the starting indexes in the symbols or codes arrays of codes that are i+1 bits long
    pub symbols: &'a [u8],
    // for DC coeffs: these are the length in bits of diff (= DC - previousDC)
    // for AC coeffs: these are amount of previous zeros (run length - 4 bits) concatenated (a|b) with the length in bits of the AC coeff (4 bits)
    pub codes: &'a mut [u32],
    pub set: bool,
}

impl<'a> HuffmanTable<'a> {
    pub fn generate_codes(&mut self) {
        let mut code: u32 = 0;

        for i in 0..16 {
            for j in self.offsets[i]..self.offsets[i + 1] {
                self.codes[j as usize] = code;
                code += 1;
            }
            code <<= 1;
        }
        self.set = true;
    }

    pub fn get_code(&self, symbol: u8) -> Option<(u32, u8)> {
        for i in 0..16 {
            for j in self.offsets[i]..self.offsets[i + 1] {
                if symbol == self.symbols[j as usize] {
                    let code: u32 = self.codes[j as usize];
                    let code_length: u8 = (i as u8) + 1;
                    return Some((code, code_length));
                }
            }
        }
        return None;
    }
}

pub static mut Y_DC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 0, 1, 6, 7, 8, 9, 10, 11, 12, 12, 12, 12, 12, 12, 12, 12],
    symbols: &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b],
    codes: &mut [0; 12],
    set: false,
};

pub static mut CH_DC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 12, 12, 12, 12],
    symbols: &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b],
    codes: &mut [0; 12],
    set: false,
};

pub static mut Y_AC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 0, 2, 3, 6, 9, 11, 15, 18, 23, 28, 32, 36, 36, 36, 37, 162],
    symbols: &[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
        0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
        0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
        0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
        0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
        0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
        0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
        0xf9, 0xfa,
    ],
    codes: &mut [0; 176],
    set: false,
};

pub static mut CH_AC_HUFFMAN_TABLE: HuffmanTable = HuffmanTable {
    offsets: [0, 0, 2, 3, 5, 9, 13, 16, 20, 27, 32, 36, 40, 40, 41, 43, 162],
    symbols: &[
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
        0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
        0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
        0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
        0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
        0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
        0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
        0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
        0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
        0xf9, 0xfa,
    ],
    codes: &mut [0; 176],
    set: false,
};

pub const ZIG_ZAG_MAP: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

// these functions are workarounds for having the tables as global variables, while also being able to initialize them
// I looked into OnceCell and lazy_static, but couldn't get them to work
pub fn initialize_huffman_tables() {
    unsafe {
        if !Y_DC_HUFFMAN_TABLE.set {
            Y_DC_HUFFMAN_TABLE.generate_codes();
        }
        if !CH_DC_HUFFMAN_TABLE.set {
            CH_DC_HUFFMAN_TABLE.generate_codes();
        }
        if !Y_AC_HUFFMAN_TABLE.set {
            Y_AC_HUFFMAN_TABLE.generate_codes();
        }
        if !CH_AC_HUFFMAN_TABLE.set {
            CH_AC_HUFFMAN_TABLE.generate_codes();
        }
    }
}

pub fn get_huffman_table(table_type: HuffmanTableType) -> &'static HuffmanTable<'static> {
    unsafe {
        match table_type {
            HuffmanTableType::YDC => &Y_DC_HUFFMAN_TABLE,
            HuffmanTableType::CHDC => &CH_DC_HUFFMAN_TABLE,
            HuffmanTableType::YAC => &Y_AC_HUFFMAN_TABLE,
            HuffmanTableType::CHAC => &CH_AC_HUFFMAN_TABLE,
        }
    }
}
