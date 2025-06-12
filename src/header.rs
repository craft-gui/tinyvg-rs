use std::io::{Cursor, Read};
use byteorder::ReadBytesExt;
use crate::common::{read_size, read_variable_sized_unsigned_number};
use crate::TinyVgParseError;

#[repr(u8)]
#[derive(Debug)]
pub enum ColorEncoding {
    /// Each color is a 4-tuple (red, green, blue, alpha) of bytes with the color
    /// channels encoded in sRGB and the alpha as linear alpha.
    Rgba8888 = 0,

    /// Each color is encoded as a 3-tuple (red, green, blue) with 16 bit per color.
    /// While red and blue both use 5 bit, the green channel uses 6 bit. red uses
    /// bit range 0...4, green bits 5...10 and blue bits 11...15. This color also uses
    /// the sRGB color space.

    Rgb565 = 1,
    /// Each color is a 4-tuple (red, green ,blue, alpha) of binary32 IEEE 754
    /// floating point value with the color channels encoded in scRGB and the
    /// alpha as linear alpha. A color value of 1.0 is full intensity, while a value of
    /// 0.0 is zero intensity.
    RgbaF32 = 2,

    /// The custom color encoding is defined undefined. The information how these
    /// colors are encoded must be implemented via external means.
    Custom = 3
}

impl ColorEncoding {
    fn from_u8(value: u8) -> ColorEncoding {
        match value {
            0 => ColorEncoding::Rgba8888,
            1 => ColorEncoding::Rgb565,
            2 => ColorEncoding::RgbaF32,
            3 => ColorEncoding::Custom,
            _ => unreachable!("ColorEncoding::from_u8 must be a 2-bit value.")
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum CoordinateRange {
    /// Each Unit takes up 16 bit.
    Default = 0,

    /// Each Unit takes up 8 bit.
    Reduced = 1,

    /// Each Unit takes up 32 bit.
    Enhanced = 2,
}

impl CoordinateRange {
    fn from_u8(value: u8) -> CoordinateRange {
        match value {
            0 => CoordinateRange::Default,
            1 => CoordinateRange::Reduced,
            2 => CoordinateRange::Enhanced,
            _ => unreachable!("CoordinateRange::from_u8 must be a 2-bit value.")
        }
    }
}

#[derive(Debug)]
pub struct TinyVgHeader {
    /// https://en.wikipedia.org/wiki/File_format#Magic_number
    pub magic: [u8; 2],

    /// The TinyVG version.
    pub version: u8,

    /// Defines the number of fraction bits in a Unit value.
    pub scale: u8,

    /// Defines the type of color information that is used in the color table.
    pub color_encoding: ColorEncoding,

    /// Defines the number of total bits in a Unit value and thus the overall
    /// precision of the file.
    pub coordinate_range: CoordinateRange,

    /// Encodes the maximum width of the output file in display units.
    /// A value of 0 indicates that the image has the maximum possible
    /// width.
    pub width: u32,

    /// Encodes the maximum height of the output file in display units.
    /// A value of 0 indicates that the image has the maximum possible
    /// height.
    pub height: u32,

    /// The number of colors in the color table.
    pub color_count: u64,
}

impl TinyVgHeader {
    pub(crate) fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, TinyVgParseError> {
        let mut magic = [0u8; 2];
        cursor.read_exact(&mut magic).map_err(|_| TinyVgParseError::InvalidHeader)?;
        
        // Must be { 0x72, 0x56 }
        if magic[0] != 0x72 || magic[1] != 0x56 {
            return Err(TinyVgParseError::InvalidHeader);
        }
        
        let version = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidHeader)?;

        // The encoded scale, color encoding, and coordinate range data.
        let scc = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidHeader)?;

        // bits 0–3 = scale
        let scale: u8 = scc & 0x0F;

        // bits 4–5 = color encoding
        let color_encoding_raw = (scc & 0b00_11_00_00) >> 4;
        let color_encoding = ColorEncoding::from_u8(color_encoding_raw);

        // bits 6–7 = coordinate range
        let coordinate_range_raw = (scc & 0b11_00_00_00) >> 6;
        let coordinate_range = CoordinateRange::from_u8(coordinate_range_raw);

        let width: u32 = read_size(&coordinate_range, cursor)?;
        let height: u32 = read_size(&coordinate_range, cursor)?;

        let color_count = read_variable_sized_unsigned_number(cursor)?;

        let header = TinyVgHeader {
            magic,
            version,
            scale,
            color_encoding,
            coordinate_range,
            width,
            height,
            color_count,
        };

        Ok(header)
    }   
}