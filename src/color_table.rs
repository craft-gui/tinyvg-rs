use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::{TinyVgParseError};
use crate::header::{ColorEncoding, TinyVgHeader};
#[derive(Debug, Copy, Clone)]
pub struct RgbaF32(pub f32, pub f32, pub f32, pub f32);

pub type ColorTable = Vec<RgbaF32>;

pub(crate) fn parse_color_table(cursor: &mut Cursor<&[u8]>, header: &TinyVgHeader) -> Result<ColorTable, TinyVgParseError> {
    let mut color_table_rgba_f32 = Vec::with_capacity(header.color_count as usize);

    for _ in 0..header.color_count {
        match header.color_encoding {
            ColorEncoding::Rgba8888 => {
                let r = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidColorTable)? as f32;
                let g = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidColorTable)? as f32;
                let b = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidColorTable)? as f32;
                let a = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidColorTable)? as f32;
                color_table_rgba_f32.push(RgbaF32(r / 255.0, g / 255.0, b / 255.0, a / 255.0));
            }
            ColorEncoding::Rgb565 => {
                let color = cursor.read_u16::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidColorTable)?;
                const FIVE_BIT_MASK: u16 = 31;
                const SIX_BIT_MASK: u16 = 63;
                // Red color channel between 0 and 100% intensity, mapped to integer values 0 to 31.
                let red: u8 = (color & FIVE_BIT_MASK) as u8;
                // Green color channel between 0 and 100% intensity, mapped to integer values 0 to 63.
                let green: u8 = ((color >> 5) & SIX_BIT_MASK) as u8;
                // Blue color channel between 0 and 100% intensity, mapped to integer values 0 to 31.
                let blue: u8 = ((color >> 11) & FIVE_BIT_MASK) as u8;
                color_table_rgba_f32.push(RgbaF32(red as f32 / 31.0, green as f32 / 63.0, blue as f32 / 31.0, 1.0));
            }
            ColorEncoding::RgbaF32 => {
                let r = cursor.read_f32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidColorTable)?;
                let g = cursor.read_f32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidColorTable)?;
                let b = cursor.read_f32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidColorTable)?;
                let a = cursor.read_f32::<LittleEndian>().map_err(|_| TinyVgParseError::InvalidColorTable)?;

                color_table_rgba_f32.push(RgbaF32(r, g, b, a));
            }
            ColorEncoding::Custom => unreachable!("Custom color encoding not supported.")

        }
    }

    Ok(color_table_rgba_f32)
}   