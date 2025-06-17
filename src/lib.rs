pub mod header;
pub mod common;
pub mod color_table;
pub mod commands;
#[cfg(feature = "svg-to-tvg")]
pub mod svg_to_tvg;

use crate::color_table::{parse_color_table, ColorTable};
use crate::commands::{parse_draw_commands, DrawCommand};
use crate::header::{CoordinateRange, TinyVgHeader};
use std::io::{Cursor};

#[derive(Debug, PartialEq)]
pub enum TinyVgParseError {
    None,
    InvalidHeader,
    InvalidColorTable,
    InvalidCommand,
}

#[derive(Debug)]
pub struct TinyVg {
    pub header: TinyVgHeader,
    pub color_table: ColorTable,
    pub draw_commands: Vec<DrawCommand>
}

impl TinyVg {

    pub fn from_bytes(data: &[u8]) -> Result<TinyVg, TinyVgParseError> {
        let mut cursor = Cursor::new(data);

        let header = TinyVgHeader::parse(&mut cursor)?;
        let color_table = parse_color_table(&mut cursor, &header)?;
        let draw_commands: Vec<DrawCommand> = parse_draw_commands(&mut cursor, &header)?;

        Ok(TinyVg {
            header,
            color_table,
            draw_commands,
        })
    }
}