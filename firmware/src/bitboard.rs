pub fn only_one_bit_set_to_one(bitboard: u32) -> bool {
    bitboard != 0 && (bitboard & (bitboard - 1)) == 0
}

pub fn only_different(data1: u32, data2: u32) -> u32 {
    return data1 ^ data2;
}

pub fn set_bit(data: u32, bit_position: usize) -> u32 {
    data | (1 << bit_position)
}

pub fn get(data: u32, pos: usize) -> bool {
    // Erzeuge eine Maske, um nur das Bit an der pos-ten Position zu isolieren
    let mask = 1 << pos;

    // Führe eine Bitweise AND-Operation zwischen num und mask durch
    // Wenn das Ergebnis ungleich 0 ist, bedeutet das, dass das Bit gesetzt ist
    (data & mask) != 0
}
