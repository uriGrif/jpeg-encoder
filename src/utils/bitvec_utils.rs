use bitvec::{ order::Msb0, vec::BitVec };

pub fn write_bits(bitvec: &mut BitVec<u8, Msb0>, value: u32, length: u8) {
    for i in (0..length).rev() {
        let bit = (value >> i) & 1; // get the i-th bit
        bitvec.push(bit != 0);
    }
}
