use crate::commands::Point;
use crate::header::TinyVgHeader;
use crate::{CoordinateRange, TinyVgParseError};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

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

pub(crate) fn write_unit(
    scale: u8,
    cursor: &mut Cursor<Vec<u8>>,
    coordinate_range: &CoordinateRange,
    value: Unit,
) -> Result<(), TinyVgParseError> {
    let scaled = (value.0 * (1 << scale) as f64).round() as i64;

    match coordinate_range {
        CoordinateRange::Default => {
            if scaled < i16::MIN as i64 || scaled > i16::MAX as i64 {
                return Err(TinyVgParseError::InvalidCommand);
            }
            cursor.write_i16::<LittleEndian>(scaled as i16).map_err(|_| TinyVgParseError::InvalidCommand)
        }
        CoordinateRange::Reduced => {
            if scaled < i8::MIN as i64 || scaled > i8::MAX as i64 {
                return Err(TinyVgParseError::InvalidCommand);
            }
            cursor.write_i8(scaled as i8).map_err(|_| TinyVgParseError::InvalidCommand)
        }
        CoordinateRange::Enhanced => {
            if scaled < i32::MIN as i64 || scaled > i32::MAX as i64 {
                return Err(TinyVgParseError::InvalidCommand);
            }
            cursor.write_i32::<LittleEndian>(scaled as i32).map_err(|_| TinyVgParseError::InvalidCommand)
        }
    }
}

pub(crate) fn write_variable_sized_unsigned_number(
    cursor: &mut Cursor<Vec<u8>>,
    mut value: u64,
) -> Result<(), TinyVgParseError> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        cursor.write_u8(byte).map_err(|_| TinyVgParseError::InvalidHeader)?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}


pub(crate) fn write_size(
    range: &CoordinateRange,
    cursor: &mut Cursor<Vec<u8>>,
    value: u32,
) -> Result<(), TinyVgParseError> {
    match range {
        CoordinateRange::Reduced => cursor.write_u8(value as u8).map_err(|_| TinyVgParseError::InvalidHeader),
        CoordinateRange::Default => cursor.write_u16::<LittleEndian>(value as u16).map_err(|_| TinyVgParseError::InvalidHeader),
        CoordinateRange::Enhanced => cursor.write_u32::<LittleEndian>(value).map_err(|_| TinyVgParseError::InvalidHeader),
    }
}

pub(crate) fn write_point(point: &Point, header: &TinyVgHeader, cursor: &mut Cursor<Vec<u8>>) -> Result<(), TinyVgParseError> {
    write_unit(header.scale, cursor, &header.coordinate_range, point.x)?;
    write_unit(header.scale, cursor, &header.coordinate_range, point.y)?;
    Ok(())
}