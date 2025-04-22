use crate::common::{read_unit, read_variable_sized_unsigned_number, Unit};
use crate::header::TinyVgHeader;
use crate::TinyVgParseError;
use byteorder::ReadBytesExt;
use std::io::{Cursor, Read};

#[repr(u8)]
#[derive(Debug)]
pub enum StyleType {
    /// Colored The shape is uniformly colored with a single color.
    Flat = 0,
    /// Gradient The shape is colored with a linear gradient.
    Linear = 1,
    /// Gradient The shape is colored with a radial gradient.
    Radial = 2
}

impl StyleType {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Flat,
            1 => Self::Linear,
            2 => Self::Radial,
            _ => unreachable!("Style::from_u8 must be 0, 1, or 2.")
        }
    }
}

#[derive(Debug)]
pub struct FlatColored {
    pub color_index: u64
}
impl FlatColored {
    pub fn read_from_cursor(cursor: &mut Cursor<&[u8]>) -> Result<FlatColored, TinyVgParseError> {
        let color_index = read_variable_sized_unsigned_number(cursor)?;

        Ok(FlatColored {
            color_index,
        })
    }

}

#[derive(Debug)]
pub struct LinearGradient {
    pub point_0: Point,
    pub point_1: Point,
    pub color_index_0: u64,
    pub color_index_1: u64,
}

impl LinearGradient {
    pub fn read_from_cursor(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>) -> Result<LinearGradient, TinyVgParseError> {
        let point_0 = Point::read_point(header, cursor)?;
        let point_1 = Point::read_point(header, cursor)?;

        let color_index_0 = read_variable_sized_unsigned_number(cursor)?;
        let color_index_1 = read_variable_sized_unsigned_number(cursor)?;

        Ok(LinearGradient {
            point_0,
            point_1,
            color_index_0,
            color_index_1,
        })
    }
}

#[derive(Debug)]
pub struct RadialGradient {
    pub point_0: Point,
    pub point_1: Point,
    pub color_index_0: u64,
    pub color_index_1: u64,
}

impl RadialGradient {
    pub fn read_from_cursor(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>) -> Result<RadialGradient, TinyVgParseError> {
        let point_0 = Point::read_point(header, cursor)?;
        let point_1 = Point::read_point(header, cursor)?;

        let color_index_0 = read_variable_sized_unsigned_number(cursor)?;
        let color_index_1 = read_variable_sized_unsigned_number(cursor)?;

        Ok(RadialGradient {
            point_0,
            point_1,
            color_index_0,
            color_index_1,
        })
    }
}

#[derive(Debug)]
pub enum Style {
    FlatColor(FlatColored),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

impl Style {
    fn read_cursor_using_style_type(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>, style_type: &StyleType) ->  Result<Style, TinyVgParseError> {
        match style_type {
            StyleType::Flat   => Ok(Style::FlatColor(FlatColored::read_from_cursor(cursor)?)),
            StyleType::Linear => Ok(Style::LinearGradient(LinearGradient::read_from_cursor(header, cursor)?)),
            StyleType::Radial => Ok(Style::RadialGradient(RadialGradient::read_from_cursor(header, cursor)?))
        }
    }
}

/// The next draw command.
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum CommandType {
    /// This command determines the end of file.
    EndOfDocument = 0,

    /// This command fills an N-gon.
    FillPolygon = 1,

    /// This command fills a set of rectangles.
    FillRectangles = 2,

    /// This command fills a free-form path.
    FillPath = 3,

    /// This command draws a set of lines.
    DrawLines = 4,

    /// This command draws the outline of a polygon.
    DrawLineLoop = 5,

    /// This command draws a list of end-to-end lines.
    DrawLineStrip = 6,

    /// This command draws a free-form path.
    DrawLinePath = 7,

    /// This command draws a filled polygon with an outline.
    OutlineFillPolygon = 8,

    /// This command draws several filled rectangles with an outline.
    OutlineFillRectangles = 9,

    /// This command combines the fill and draw line path command into one.
    OutlineFillPath = 10,

    /// This command defines the contents and glyph location for text.
    TextHint = 11
}

impl CommandType {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::EndOfDocument,
            1 => Self::FillPolygon,
            2 => Self::FillRectangles,
            3 => Self::FillPath,
            4 => Self::DrawLines,
            5 => Self::DrawLineLoop,
            6 => Self::DrawLineStrip,
            7 => Self::DrawLinePath,
            8 => Self::OutlineFillPolygon,
            9 => Self::OutlineFillRectangles,
            10 => Self::OutlineFillPath,
            11 => Self::TextHint,
            _ => unreachable!("Style::from_u8 must be in the range 0 to 11.")
        }
    }
}

#[derive(Debug, Copy, Clone,)]
pub struct Point {
    pub x: Unit,
    pub y: Unit,
}

impl Point {
    fn read_point(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>) ->  Result<Point, TinyVgParseError> {
        let x = read_unit(header.scale, cursor, &header.coordinate_range)?;
        let y = read_unit(header.scale, cursor, &header.coordinate_range)?;
        let start = Point { x, y };
        Ok(start)
    }
    
    pub fn move_to(&self, point: &Point) -> Self {
        point.clone()
    }
}

#[derive(Debug)]
pub struct Rectangle {
    pub x: Unit,
    pub y: Unit,
    pub width: Unit,
    pub height: Unit,
}

impl Rectangle {
    fn read_rectangle(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>) ->  Result<Rectangle, TinyVgParseError> {
        let x = read_unit(header.scale, cursor, &header.coordinate_range)?;
        let y = read_unit(header.scale, cursor, &header.coordinate_range)?;
        let width = read_unit(header.scale, cursor, &header.coordinate_range)?;
        let height = read_unit(header.scale, cursor, &header.coordinate_range)?;
        Ok(Rectangle { x, y, width, height})
    }
}

#[derive(Debug)]
pub struct Line {
    /// Start point of the line
    pub start: Point,
    /// End point of the line.
    pub end: Point,
}

impl Line {
    fn read_line(header: &TinyVgHeader, cursor: &mut Cursor<&[u8]>) ->  Result<Line, TinyVgParseError> {
        let start = Point::read_point(header, cursor)?;
        let end = Point::read_point(header, cursor)?;
        Ok(Line{ start, end })
    }
}

#[derive(Debug)]
pub struct FillPolygonData {
    pub style: Style,
    pub points: Vec<Point>,
}

#[derive(Debug)]
pub struct FillRectanglesData {
    pub style: Style,
    pub rectangles: Vec<Rectangle>,
}

#[derive(Debug)]
pub struct FillPathData {
    pub style: Style,
    pub path: Path,
}

#[derive(Debug)]
pub struct DrawLinesData {
    pub lines: Vec<Line>,
    pub line_width: Unit,
    pub line_style: Style,
}

#[derive(Debug)]
pub struct DrawLineLoopData {
    pub line_style: Style,
    pub line_width: Unit,
    pub points: Vec<Point>
}

#[derive(Debug)]
pub struct DrawLineStripData {
    pub style: Style,
    pub line_width: Unit,
    pub points: Vec<Point>
}

#[derive(Debug)]
pub struct DrawLinePathData {
    pub style: Style,
    pub line_width: Unit,
    pub path: Path,
}

#[derive(Debug)]
pub struct OutlineFillPolygonData {
    pub fill_style: Style,
    pub line_style: Style,
    pub line_width: Unit,
    pub points: Vec<Point>,
}

#[derive(Debug)]
pub struct OutlineFillRectanglesData {
    pub fill_style: Style,
    pub line_style: Style,
    pub line_width: Unit,
    pub rectangles: Vec<Rectangle>,
}

#[derive(Debug)]
pub struct OutlineFillPathData {
    pub path: Path,
    pub fill_style: Style,
    pub line_style: Style,
    pub line_width: Unit
}

#[derive(Debug)]
pub struct TextHintData {
    /// The center of the descender line for the defined text.
    pub center: Point,
    /// The amount of degrees the text is rotated.
    pub rotation: Unit,
    /// The font size or distance from the ascender line to the
    /// descender line for the text.
    pub height: Unit,
    pub text: String,
    /// The number of glyphs within the text.
    pub glyph_length: u64,
    /// The start and end offset on the descender line from the
    /// center for each glyph
    pub glyph_offset: Vec<(Unit, Unit)>,
}

#[derive(Debug)]
pub enum DrawCommand {
    /// This command fills an N-gon.
    FillPolygon(FillPolygonData),

    /// This command fills a set of rectangles.
    FillRectangles(FillRectanglesData),

    /// This command fills a free-form path.
    FillPath(FillPathData),

    /// This command draws a set of lines.
    DrawLines(DrawLinesData),

    /// This command draws the outline of a polygon.
    DrawLineLoop(DrawLineLoopData),

    /// This command draws a list of end-to-end lines.
    DrawLineStrip(DrawLineStripData),

    /// This command draws a free-form path.
    DrawLinePath(DrawLinePathData),

    /// This command draws a filled polygon with an outline.
    OutlineFillPolygon(OutlineFillPolygonData),

    /// This command draws several filled rectangles with an outline.
    OutlineFillRectangles(OutlineFillRectanglesData),

    /// This command combines the fill and draw line path command into one.
    OutlineFillPath(OutlineFillPathData),

    /// This command only provides metadata for accessibility or text selection tools for the position and content
    /// of text. A renderer can safely ignore this command since it must not have any effect on the resulting
    /// graphic.
    TextHint(TextHintData)
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
enum PathCommandType {
    Line = 0,
    HorizontalLine = 1,
    VerticalLine = 2,
    CubicBezier = 3,
    ArcCircle = 4,
    ArcEllipse = 5,
    ClosePath = 6,
    QuadraticBezier = 7,
}

impl PathCommandType {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Line,
            1 => Self::HorizontalLine,
            2 => Self::VerticalLine,
            3 => Self::CubicBezier,
            4 => Self::ArcCircle,
            5 => Self::ArcEllipse,
            6 => Self::ClosePath,
            7 => Self::QuadraticBezier,
            _ => unreachable!("PathCommand::from_u8 must be in the range 0 to 7.")
        }
    }
}

#[derive(Debug)]
pub struct CubicBezier {
    pub control_point_0: Point,
    pub control_point_1: Point,
    pub point_1: Point,
}

#[derive(Debug)]
pub struct ArcCircle {
    pub large_arc: bool,
    pub sweep: bool,
    pub radius: Unit,
    pub target: Point,
}

#[derive(Debug)]
pub struct ArcEllipse {
    pub large_arc: bool,
    pub sweep: bool,
    pub radius_x: Unit,
    pub radius_y: Unit,
    pub rotation: Unit,
    pub target: Point,
}

#[derive(Debug)]
pub struct QuadraticBezier {
    pub control_point: Point,
    pub point_1: Point,
}

#[derive(Debug)]
pub enum PathCommand {
    Line(Point, Option<Unit>),
    HorizontalLine(Unit, Option<Unit>),
    VerticalLine(Unit, Option<Unit>),
    CubicBezier(CubicBezier, Option<Unit>),
    ArcCircle(ArcCircle, Option<Unit>),
    ArcEllipse(ArcEllipse, Option<Unit>),
    ClosePath,
    QuadraticBezier(QuadraticBezier, Option<Unit>),
}

#[derive(Debug)]
pub struct Segment {
    pub start: Point,
    pub path_commands: Vec<PathCommand>,
}

#[derive(Debug)]
pub struct Path {
    pub segments: Vec<Segment>,
}

impl Path {
    pub fn parse(cursor: &mut Cursor<&[u8]>, header: &TinyVgHeader, segment_count: usize) -> Result<Self, TinyVgParseError> {
        let mut segment_command_counts: Vec<usize> = Vec::new();
        for _ in 0..segment_count {
            let segment_length = read_variable_sized_unsigned_number(cursor)? + 1;
            segment_command_counts.push(segment_length as usize);
        }

        let mut segments: Vec<Segment> = Vec::new();

        for i in 0..segment_count {
            let start = Point::read_point(header, cursor)?;

            let mut segment = Segment {
                start,
                path_commands: vec![],
            };

            let commands_count = segment_command_counts[i];

            for _ in 0..commands_count {
                let command_tag = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
                let path_command_raw = command_tag & 0b00_00_01_11;
                let path_command = PathCommandType::from_u8(path_command_raw);
                let has_line_width = (command_tag & 0b00_01_00_00) != 0;
                let mut line_width: Option<Unit> = None;

                if has_line_width {
                    // FIXME: Figure out how this should be used in the Vello example.
                    line_width = Some(read_unit(header.scale, cursor, &header.coordinate_range)?);
                }

                match path_command {
                    PathCommandType::Line => {
                        let point = Point::read_point(header, cursor)?;
                        segment.path_commands.push(PathCommand::Line(point, line_width));
                    }
                    PathCommandType::HorizontalLine => {
                        let pos_x = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        segment.path_commands.push(PathCommand::HorizontalLine(pos_x, line_width));
                    }
                    PathCommandType::VerticalLine => {
                        let pos_y = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        segment.path_commands.push(PathCommand::VerticalLine(pos_y, line_width));
                    }
                    PathCommandType::CubicBezier => {
                        let control_0 = Point::read_point(header, cursor)?;
                        let control_1 = Point::read_point(header, cursor)?;
                        let point_1 = Point::read_point(header, cursor)?;

                        segment.path_commands.push(PathCommand::CubicBezier(
                            CubicBezier {
                                control_point_0: control_0,
                                control_point_1: control_1,
                                point_1,
                            }, 
                            line_width
                        ));
                    }
                    PathCommandType::ArcCircle => {
                        let large_arc_sweep_padding = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
                        let large_arc = (large_arc_sweep_padding & 0b00_00_00_01) != 0;
                        let sweep = (large_arc_sweep_padding & 0b00_00_00_10) != 0;
                        let radius = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        let target = Point::read_point(header, cursor)?;

                        segment.path_commands.push(PathCommand::ArcCircle(
                            ArcCircle {
                                large_arc,
                                sweep,
                                radius,
                                target
                            },
                            line_width
                        ))
                    }
                    PathCommandType::ArcEllipse => {
                        let large_arc_sweep_padding = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
                        let large_arc = (large_arc_sweep_padding & 0b00_00_00_01) != 0;
                        let sweep = (large_arc_sweep_padding & 0b00_00_00_10) != 0;

                        let radius_x = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        let radius_y = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        let rotation = read_unit(header.scale, cursor, &header.coordinate_range)?;
                        let target = Point::read_point(header, cursor)?;

                        let arc_ellipse = ArcEllipse {
                            large_arc,
                            sweep,
                            radius_x,
                            radius_y,
                            rotation,
                            target,
                        };
                        segment.path_commands.push(PathCommand::ArcEllipse(arc_ellipse, line_width));
                    }
                    PathCommandType::ClosePath => {
                        segment.path_commands.push(PathCommand::ClosePath);
                    }
                    PathCommandType::QuadraticBezier => {
                        let control = Point::read_point(header, cursor)?;
                        let point_1 = Point::read_point(header, cursor)?;

                        let quadratic_bezier = QuadraticBezier {
                            control_point: control,
                            point_1
                        };
                        segment.path_commands.push(PathCommand::QuadraticBezier(quadratic_bezier, line_width));
                    }
                }
            }

            segments.push(segment);
        }

        Ok(Self {
            segments,
        })
    }
}

pub(crate) fn parse_draw_commands(cursor: &mut Cursor<&[u8]>, header: &TinyVgHeader) -> Result<Vec<DrawCommand>, TinyVgParseError> {
    let mut draw_commands: Vec<DrawCommand> = Vec::new();

    loop {
        let encoded_command = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
        // bits 0–6 = command_index
        let command_index = encoded_command & 0b00_11_11_11;
        // bits 7-8 = prim_style_kind
        let prim_style_kind = (encoded_command & 0b11_00_00_00) >> 6;

        let command = CommandType::from_u8(command_index);

        // If this command is read, the TinyVG file has ended. This command must have prim_style_kind to be
        // set to 0, so the last byte of every TinyVG file is 0x00.
        if matches!(command, CommandType::EndOfDocument) {
            break;
        }

        let style_type = StyleType::from_u8(prim_style_kind);

        match command {
            CommandType::EndOfDocument => {
                unreachable!("We should have broken out of the loop above.")
            }
            CommandType::FillPolygon => {
                // The number of points in the polygon. This value is offset by 1.
                let point_count = read_variable_sized_unsigned_number(cursor)? + 1;
                let mut points: Vec<Point> = Vec::with_capacity(point_count as usize);

                // The style that is used to fill the polygon.
                let style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The points of the polygon.
                for _ in 0..point_count {
                    let point = Point::read_point(header, cursor)?;
                    points.push(point);
                }

                let data = FillPolygonData {
                    style,
                    points,
                };
                draw_commands.push(DrawCommand::FillPolygon(data))
            }
            CommandType::FillRectangles => {
                // The number of points in the polygon. This value is offset by 1.
                let rectangle_count = read_variable_sized_unsigned_number(cursor)? + 1;

                // The style that is used to fill all rectangles.
                let style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;
                
                // The list of rectangles to be filled.
                let mut rectangles: Vec<Rectangle> = Vec::with_capacity(rectangle_count as usize);
                for _ in 0..rectangle_count {
                    // Horizontal distance of the left side to the origin.
                    let x = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    
                    // Vertical distance of the upper side to the origin.
                    let y = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    
                    // Horizontal extent of the rectangle.
                    let width = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    
                    // Vertical extent of the rectangle origin.
                    let height = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    rectangles.push(Rectangle { x, y, width, height });
                }

                let data = FillRectanglesData {
                    rectangles,
                    style,
                };
                draw_commands.push(DrawCommand::FillRectangles(data))
            }
            CommandType::FillPath => {
                // The number of segments in the path. This value is offset by 1.
                let segment_count = read_variable_sized_unsigned_number(cursor)? + 1;
                
                // The style that is used to fill the path.
                let style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // A path with segment_count segments.
                let path = Path::parse(cursor, header, segment_count as usize)?;

                let data = FillPathData {
                    path,
                    style,
                };
                draw_commands.push(DrawCommand::FillPath(data));
            }
            CommandType::DrawLines => {
                // The number of rectangles. This value is offset by 1.
                let line_count = read_variable_sized_unsigned_number(cursor)? + 1;
                
                // The style that is used to draw the all rectangles.
                let line_style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The list of lines.
                let mut lines: Vec<Line> = Vec::with_capacity(line_count as usize);
                for _ in 0..line_count {
                    let line = Line::read_line(header, cursor)?;
                    lines.push(line);
                }
                
                let data = DrawLinesData {
                    lines,
                    line_width,
                    line_style,
                };
                draw_commands.push(DrawCommand::DrawLines(data));
            }
            CommandType::DrawLineLoop => {
                // The number of points. This value is offset by 1.
                let point_count = read_variable_sized_unsigned_number(cursor)? + 1;

                // The style that is used to draw the all rectangles.
                let line_style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The points of the polygon.
                let mut points: Vec<Point> = Vec::with_capacity(point_count as usize);
                for _ in 0..point_count {
                    let point = Point::read_point(header, cursor)?;
                    points.push(point);
                }

                let data = DrawLineLoopData {
                    line_style,
                    line_width,
                    points,
                };
                draw_commands.push(DrawCommand::DrawLineLoop(data));
            }
            CommandType::DrawLineStrip => {
                // The number of points. This value is offset by 1.
                let point_count = read_variable_sized_unsigned_number(cursor)? + 1;

                // The style that is used to draw the all rectangles.
                let style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The points of the line strip.
                let mut points: Vec<Point> = Vec::with_capacity(point_count as usize);
                for _ in 0..point_count {
                    let point = Point::read_point(header, cursor)?;
                    points.push(point);
                }

                let data = DrawLineStripData {
                    style,
                    line_width,
                    points
                };
                draw_commands.push(DrawCommand::DrawLineStrip(data));
            }
            CommandType::DrawLinePath => {
                // The number of segments in the path. This value is offset by 1.
                let segment_count = read_variable_sized_unsigned_number(cursor)? + 1;

                // The style that is used to draw the all rectangles.
                let style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // A path with segment_count segments.
                let path = Path::parse(cursor, header, segment_count as usize)?;

                let data = DrawLinePathData {
                    style,
                    line_width,
                    path,
                };
                draw_commands.push(DrawCommand::DrawLinePath(data));
            }
            CommandType::OutlineFillPolygon => {
                let point_count_sec_style_kind = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
                // The number of points in the polygon. This value is offset by 1.
                let point_count = (point_count_sec_style_kind & 0b00_11_11_11) + 1;

                // The secondary style used in this command.
                let sec_style_kind = point_count_sec_style_kind & 0b11_00_00_00;

                // The style that is used to fill the polygon.
                let fill_style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The style that is used to draw the outline of the polygon.
                let line_style = Style::read_cursor_using_style_type(header, cursor, &StyleType::from_u8(sec_style_kind))?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The set of points of this polygon.
                let mut points: Vec<Point> = Vec::with_capacity(point_count as usize);
                for _ in 0..point_count {
                    let point = Point::read_point(header, cursor)?;
                    points.push(point);
                }

                let data = OutlineFillPolygonData {
                    points,
                    line_width,
                    line_style,
                    fill_style,
                };
                draw_commands.push(DrawCommand::OutlineFillPolygon(data));
            }
            CommandType::OutlineFillRectangles => {
                let rect_count_sec_style_kind = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;
                // The number of rectangles. This value is offset by 1.
                let rect_count = (rect_count_sec_style_kind & 0b00_11_11_11) + 1;

                // The secondary style used in this command.
                let sec_style_kind = rect_count_sec_style_kind & 0b11_00_00_00;

                // The style that is used to fill the polygon.
                let fill_style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The style that is used to draw the outline of the polygon.
                let line_style = Style::read_cursor_using_style_type(header, cursor, &StyleType::from_u8(sec_style_kind))?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The list of rectangles to be drawn.
                let mut rectangles: Vec<Rectangle> = Vec::with_capacity(rect_count as usize);
                for _ in 0..rect_count {
                    let rectangle = Rectangle::read_rectangle(header, cursor)?;
                    rectangles.push(rectangle);
                }

                let data = OutlineFillRectanglesData {
                    fill_style,
                    line_style,
                    line_width,
                    rectangles,
                };
                draw_commands.push(DrawCommand::OutlineFillRectangles(data));
            }
            CommandType::OutlineFillPath => {
                let segment_count_and_sec_style_kind = cursor.read_u8().map_err(|_| TinyVgParseError::InvalidCommand)?;

                // The number of points in the polygon. This value is offset by 1
                let segment_count = (segment_count_and_sec_style_kind & 0b00_11_11_11) + 1;

                // The secondary style used in this command.
                let sec_style_kind = segment_count_and_sec_style_kind & 0b11_00_00_00;
                let sec_style_type = StyleType::from_u8(sec_style_kind);

                // The style that is used to fill the polygon.
                let fill_style = Style::read_cursor_using_style_type(header, cursor, &style_type)?;

                // The style that is used to draw the outline of the polygon.
                let line_style = Style::read_cursor_using_style_type(header, cursor, &sec_style_type)?;

                // The width of the line.
                let line_width = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The path that should be drawn
                let path = Path::parse(cursor, header, segment_count as usize)?;

                let data = OutlineFillPathData {
                    path,
                    fill_style,
                    line_style,
                    line_width,
                };
                draw_commands.push(DrawCommand::OutlineFillPath(data));
            }

            CommandType::TextHint => {
                // The center of the descender line for the defined text.
                let center = Point::read_point(header, cursor)?;

                // The amount of degrees the text is rotated.
                let rotation = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The font size or distance from the ascender line to the
                // descender line for the text.
                let height = read_unit(header.scale, cursor, &header.coordinate_range)?;

                // The number of bytes used to encode the text.
                let text_length = read_variable_sized_unsigned_number(cursor)?;

                // The UTF-8 encoded bytes corresponding to the text.
                let mut text_buffer: Vec<u8> = vec![0; text_length as usize];
                cursor.read_exact(text_buffer.as_mut_slice()).map_err(|_| TinyVgParseError::InvalidCommand)?;
                let text = String::from_utf8(text_buffer).map_err(|_| TinyVgParseError::InvalidCommand)?;

                // The number of glyphs within the text.
                let glyph_length = read_variable_sized_unsigned_number(cursor)?;

                // The start and end offset on the descender line from the
                // center for each glyph.
                let mut glyph_offset: Vec<(Unit, Unit)> = Vec::with_capacity(glyph_length as usize);
                for _ in 0..glyph_length {
                    let start_offset = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    let end_offset = read_unit(header.scale, cursor, &header.coordinate_range)?;
                    glyph_offset.push((start_offset, end_offset));
                }

                let data = TextHintData {
                    center,
                    text,
                    rotation,
                    height,
                    glyph_length,
                    glyph_offset
                };
                draw_commands.push(DrawCommand::TextHint(data));
            }
        }

    }

    Ok(draw_commands)
}