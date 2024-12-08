use bitvec::vec::BitVec;

pub fn write_bits(bitvec: &mut BitVec, value: u32, length: u8) {
    for i in (0..length).rev() {
        let bit = (value >> i) & 1; // get the i-th bit
        bitvec.push(bit != 0);
    }
}
