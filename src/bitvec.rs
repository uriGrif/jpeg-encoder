pub struct BitVec {
    data: Vec<u8>,
    next_bit_pos: u8,
    size: u128,
}

impl BitVec {
    pub fn new() -> BitVec {
        BitVec {
            data: vec![],
            next_bit_pos: 0,
            size: 0,
        }
    }

    /// writes the most significant bit of the byte sent
    pub fn writeBit(&mut self, b: u8) {
        if self.next_bit_pos == 0 {
            self.data.push(b | 0u8);
        } else {
            let aux: u8 = b >> self.next_bit_pos;
            let len = self.data.len();
            let mut last_byte = self.data[len - 1];
            last_byte |= aux;
            self.data[len - 1] = last_byte;
        }
        self.next_bit_pos = (self.next_bit_pos + 1) % 8;
        self.size += 1;
    }

    pub fn writeBits(&mut self, b: u8, n: u8) {
        for i in 0..n {
            self.writeBit(b << i);
        }
    }
}
