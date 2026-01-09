use chess::BitBoard;
use serde::Deserialize;

pub fn serialize<S>(bitboard: &BitBoard, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(bitboard.0)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<BitBoard, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <u64 as Deserialize>::deserialize(deserializer)?;
    Ok(BitBoard::new(value))
}
