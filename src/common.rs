use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::{CoordinateRange, TinyVgParseError};

#[derive(Debug, Copy, Clone)]
pub struct Unit(pub f64);

/// Unit may be 8, 16, or 32 bits, so we will advance the cursor conditionally.
pub(crate) fn read_size(coordinate_range: &CoordinateRange, cursor: &mut Cursor<&[u8]>) -> Result<u32, TinyVgParseError> {
    let res = match coordinate_range {
        CoordinateRange::Reduced  => cursor.read_u8().map_err(|_| TinyVgParseError::InvalidHeader)? as u32,
        CoordinateRange::Default  => cursor.read_u16::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidHeader)? as u32,
        CoordinateRange::Enhanced => cursor.read_u32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidHeader)?
    };
    Ok(res)
}

/// Page 4, VarUInt.
/// This type is used to encode 32-bit unsigned integers while keeping the number of bytes low. It is encoded
/// as a variable-sized integer that uses 7 bit per byte for integer bits and the 7th bit to encode that there
/// are more bits available.
pub(crate) fn read_variable_sized_unsigned_number(cursor: &mut Cursor<&[u8]>) -> Result<u64, TinyVgParseError> {
    let mut count = 0u64;
    let mut result = 0u64;
    loop {
        let byte = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidHeader)?;
        let val: u64 = (byte as u64 & 0x7F) << (7 * count);
        result |= val;
        if (byte & 0x80) == 0 {
            break;
        }
        count += 1;
    }

    Ok(result)
}


pub(crate) fn read_unit(scale: u8, cursor: &mut Cursor<&[u8]>, coordinate_range: &CoordinateRange) -> Result<Unit, TinyVgParseError> {
    let raw: i64;

    match coordinate_range {
        CoordinateRange::Default => raw = cursor.read_i16::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidCommand)? as i64,
        CoordinateRange::Reduced => raw = cursor.read_i8().map_err(|_| TinyVgParseError::InvalidCommand)? as i64,
        CoordinateRange::Enhanced => raw = cursor.read_i32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidCommand)? as i64,
    }
    
    let units_in_css_px: f64 = raw as f64 / (1 << scale) as f64;

    Ok(Unit(units_in_css_px))
}