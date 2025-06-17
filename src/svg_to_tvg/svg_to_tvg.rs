use crate::color_table::{ColorTable, RgbaF32};
use crate::commands::{CommandType, CubicBezier, DrawCommand, DrawLinePathData, FillPathData, OutlineFillPathData, Path, PathCommand, PathCommandType, Point, QuadraticBezier, Segment, Style, StyleType};
use crate::common::{write_point, write_size, write_unit, write_variable_sized_unsigned_number, Unit};
use crate::header::{ColorEncoding, CoordinateRange, TinyVgHeader};
use crate::svg_to_tvg::usvg_conversion_utils::{usvg_paint_to_tinyvg_style, usvg_point_to_tinyvg_point};
use crate::TinyVgParseError;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Write};
use usvg::tiny_skia_path::PathSegment;
use usvg::{Node, Opacity, Transform};

pub fn svg_to_tvg(svg_bytes: &[u8]) -> Vec<u8> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_bytes, &opt).expect("Failed to parse the SVG");

    let size = tree.size();

    let width = size.width() as u32;
    let height = size.height() as u32;

    let coordinate_limit = width.max(height);
    let mut scale_bits = 0;
    while scale_bits < 15 && (coordinate_limit << (scale_bits + 1)) <= i16::MAX as u32
    {
        scale_bits += 1;
    }

    let mut color_table: ColorTable = Vec::new();
    let mut draw_commands: Vec<DrawCommand> = Vec::new();

    let mut stack: Vec<(&Node, Transform, Opacity)> = tree.root().children().iter().rev().map(|c| (c, tree.root().transform(), tree.root().opacity())).collect();
    while let Some((node, transform, opacity)) = stack.pop() {

        match node {
            Node::Group(group) => {
                let new_transform = transform.post_concat(group.transform());
                let new_opacity = Opacity::new(opacity.get() * group.opacity().get()).unwrap();
                for child in group.children().iter().rev() {
                    stack.push((child, new_transform, new_opacity));
                }
            }
            Node::Path(path) => {
                let new_path = path.data().clone().transform(transform).unwrap();
                let mut segments: Vec<Segment> = Vec::new();
                let mut current = Segment {
                    start: Point::new(Unit(0.0), Unit(0.0)),
                    path_commands: Vec::new(),
                };

                for seg in new_path.segments() {

                    match seg {
                        PathSegment::MoveTo(p) => {
                            if !current.path_commands.is_empty() {
                                segments.push(current);
                            }

                            current = Segment {
                                start: usvg_point_to_tinyvg_point(p),
                                path_commands: Vec::new(),
                            };
                        }

                        PathSegment::LineTo(p) => {
                            current.path_commands.push(PathCommand::Line(usvg_point_to_tinyvg_point(p), None));
                        }

                        PathSegment::QuadTo(cp, p1) => {
                            let quad = QuadraticBezier {
                                control_point: usvg_point_to_tinyvg_point(cp),
                                point_1: usvg_point_to_tinyvg_point(p1),
                            };
                            current.path_commands.push(PathCommand::QuadraticBezier(quad, None));
                        }

                        PathSegment::CubicTo(c0, c1, p1) => {
                            let cubic = CubicBezier {
                                control_point_0: usvg_point_to_tinyvg_point(c0),
                                control_point_1: usvg_point_to_tinyvg_point(c1),
                                point_1: usvg_point_to_tinyvg_point(p1),
                            };
                            current.path_commands.push(PathCommand::CubicBezier(cubic, None));
                        }

                        PathSegment::Close => {
                            current.path_commands.push(PathCommand::ClosePath);
                        }
                    }
                }

                if !current.path_commands.is_empty() {
                    segments.push(current.clone());
                }

                let fill = path.fill();
                let stroke = path.stroke();

                let path = Path {
                    segments,
                };

                let cmd = match (fill, stroke) {
                    (Some(fill), Some(stroke)) => {

                        let fill_opacity = Opacity::new(&fill.opacity().get() * opacity.get()).unwrap();
                        let fill_style = usvg_paint_to_tinyvg_style(&fill.paint(), &fill_opacity, &mut color_table);

                        let stroke_opacity = Opacity::new(&stroke.opacity().get() * opacity.get()).unwrap();
                        let stroke_style = usvg_paint_to_tinyvg_style(&stroke.paint(), &stroke_opacity, &mut color_table);

                        let data = OutlineFillPathData {
                            path,
                            fill_style,
                            line_style: stroke_style,
                            line_width: Unit(stroke.width().get() as f64),
                        };
                        DrawCommand::OutlineFillPath(data)
                    }
                    (Some(fill), None) => {
                        let fill_opacity = Opacity::new(&fill.opacity().get() * opacity.get()).unwrap();
                        let fill_style = usvg_paint_to_tinyvg_style(&fill.paint(), &fill_opacity, &mut color_table);

                        DrawCommand::FillPath(FillPathData {
                            style: fill_style,
                            path
                        })
                    }
                    (None, Some(stroke)) => {
                        let stroke_opacity = Opacity::new(&stroke.opacity().get() * opacity.get()).unwrap();
                        let stroke_style = usvg_paint_to_tinyvg_style(&stroke.paint(), &stroke_opacity, &mut color_table);

                        DrawCommand::DrawLinePath(DrawLinePathData {
                            style: stroke_style,
                            path,
                            line_width: Unit(stroke.width().get() as f64),
                        })
                    }
                    (None, None) => continue,
                };

                draw_commands.push(cmd);
            }

            Node::Image(_img) => {}
            Node::Text(_text) => {
                // TODO: We should probably support this.
            }
        }
    }


    let mut writer = Cursor::new(Vec::new());
    let header = TinyVgHeader {
        magic: [0x72, 0x56],
        version: 1,
        scale: scale_bits,
        color_encoding: ColorEncoding::RgbaF32,
        coordinate_range: CoordinateRange::Default,
        width,
        height,
        color_count: color_table.len() as u64,
    };

    write_header(&header, &mut writer).unwrap();
    write_color_table(&mut writer, &header, &color_table).unwrap();
    write_draw_commands(&mut writer, &header, &draw_commands).unwrap();
    write_end(&mut writer).unwrap();

    writer.into_inner()
}

pub fn write_header(header: &TinyVgHeader, cursor: &mut Cursor<Vec<u8>>) -> Result<(), TinyVgParseError> {
    cursor.write_all(&header.magic).map_err(|_| TinyVgParseError::InvalidHeader)?;
    cursor.write_u8(header.version).map_err(|_| TinyVgParseError::InvalidHeader)?;

    // Pack scale (bits 0–3), color encoding (bits 4–5), coordinate range (bits 6–7)
    let scc = (header.scale & 0x0F) | ((header.color_encoding as u8) << 4) | ((header.coordinate_range as u8) << 6);
    cursor.write_u8(scc).map_err(|_| TinyVgParseError::InvalidHeader)?;

    write_size(&header.coordinate_range, cursor, header.width)?;
    write_size(&header.coordinate_range, cursor, header.height)?;
    write_variable_sized_unsigned_number(cursor, header.color_count)?;

    Ok(())
}

pub fn write_color_table(
    cursor: &mut Cursor<Vec<u8>>,
    header: &TinyVgHeader,
    colors: &[RgbaF32],
) -> Result<(), TinyVgParseError> {
    if header.color_encoding != ColorEncoding::RgbaF32 {
        return Err(TinyVgParseError::InvalidColorTable);
    }

    for &RgbaF32(r, g, b, a) in colors {
        cursor.write_f32::<LittleEndian>(r).map_err(|_| TinyVgParseError::InvalidColorTable)?;
        cursor.write_f32::<LittleEndian>(g).map_err(|_| TinyVgParseError::InvalidColorTable)?;
        cursor.write_f32::<LittleEndian>(b).map_err(|_| TinyVgParseError::InvalidColorTable)?;
        cursor.write_f32::<LittleEndian>(a).map_err(|_| TinyVgParseError::InvalidColorTable)?;
    }

    Ok(())
}

pub fn write_draw_commands(cursor: &mut Cursor<Vec<u8>>, header: &TinyVgHeader, commands: &Vec<DrawCommand>) -> Result<(), TinyVgParseError> {
    for command in commands {
        match command {
            DrawCommand::FillPolygon(_) => {}
            DrawCommand::FillRectangles(_) => {}
            DrawCommand::FillPath(data) => {
                write_command_and_primary_style(cursor, CommandType::FillPath, StyleType::from_style(&data.style))?;
                write_variable_sized_unsigned_number(cursor, data.path.segments.len() as u64 - 1)?;
                write_style(cursor, header, &data.style)?;
                write_path(&data.path, cursor, header)?;
            }
            DrawCommand::DrawLines(_) => {}
            DrawCommand::DrawLineLoop(_) => {}
            DrawCommand::DrawLineStrip(_) => {}
            DrawCommand::DrawLinePath(data) => {
                write_command_and_primary_style(cursor, CommandType::DrawLinePath, StyleType::from_style(&data.style))?;
                write_variable_sized_unsigned_number(cursor, data.path.segments.len() as u64 - 1)?;
                write_style(cursor, header, &data.style)?;
                write_unit(header.scale, cursor, &header.coordinate_range, data.line_width)?;
                write_path(&data.path, cursor, header)?;
            }
            DrawCommand::OutlineFillPolygon(_) => {}
            DrawCommand::OutlineFillRectangles(_) => {}
            DrawCommand::OutlineFillPath(data) => {
                write_command_and_primary_style(cursor, CommandType::OutlineFillPath, StyleType::from_style(&data.fill_style))?;
                let segment_count = data.path.segments.len() - 1;

                let line_style_type = StyleType::from_style(&data.line_style);
                let segment_and_style = ((line_style_type as u8) << 6) | (segment_count as u8);
                cursor
                    .write_all(&[segment_and_style])
                    .map_err(|_| TinyVgParseError::InvalidCommand)?;

                write_style(cursor, header, &data.fill_style)?;
                write_style(cursor, header, &data.line_style)?;

                write_unit(header.scale, cursor, &header.coordinate_range, data.line_width)?;

                write_path(&data.path, cursor, header)?;
            }
            DrawCommand::TextHint(_) => {}
        }
    }

    Ok(())
}

pub fn write_end(
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<(), TinyVgParseError> {
    cursor
        .write_u8(0)
        .map_err(|_| TinyVgParseError::InvalidCommand)?;

    Ok(())
}

pub fn write_style(cursor: &mut Cursor<Vec<u8>>, header: &TinyVgHeader, style: &Style) -> Result<(), TinyVgParseError> {
    match style {
        Style::FlatColor(flat_colored) => {
            write_variable_sized_unsigned_number(cursor, flat_colored.color_index)?;
        }
        Style::LinearGradient(linear_gradient) => {
            write_unit(header.scale, cursor, &header.coordinate_range, linear_gradient.point_0.x)?;
            write_unit(header.scale, cursor, &header.coordinate_range, linear_gradient.point_0.y)?;
            write_unit(header.scale, cursor, &header.coordinate_range, linear_gradient.point_1.x)?;
            write_unit(header.scale, cursor, &header.coordinate_range, linear_gradient.point_1.y)?;

            write_variable_sized_unsigned_number(cursor, linear_gradient.color_index_0)?;
            write_variable_sized_unsigned_number(cursor, linear_gradient.color_index_1)?;
        }
        Style::RadialGradient(radial_gradient) => {
            write_unit(header.scale, cursor, &header.coordinate_range, radial_gradient.point_0.x)?;
            write_unit(header.scale, cursor, &header.coordinate_range, radial_gradient.point_0.y)?;
            write_unit(header.scale, cursor, &header.coordinate_range, radial_gradient.point_1.x)?;
            write_unit(header.scale, cursor, &header.coordinate_range, radial_gradient.point_1.y)?;

            write_variable_sized_unsigned_number(cursor, radial_gradient.color_index_0)?;
            write_variable_sized_unsigned_number(cursor, radial_gradient.color_index_1)?;
        }
    }

    Ok(())
}

pub fn write_path(path: &Path, cursor: &mut Cursor<Vec<u8>>, header: &TinyVgHeader) -> Result<(), TinyVgParseError> {
    for segment in &path.segments {
        let cmd_count = segment.path_commands.len();
        if cmd_count == 0 {
            return Err(TinyVgParseError::InvalidCommand);
        }
        write_variable_sized_unsigned_number(cursor, (cmd_count - 1) as u64)?;
    }

    for segment in &path.segments {
        let Point { x, y } = segment.start;
        write_unit(header.scale, cursor, &header.coordinate_range, x)?;
        write_unit(header.scale, cursor, &header.coordinate_range, y)?;

        for cmd in &segment.path_commands {
            let (base_tag, has_lw, lw) = match cmd {
                PathCommand::Line(_, lw)              => (PathCommandType::Line as u8, lw.is_some(), lw),
                PathCommand::HorizontalLine(_, lw)    => (PathCommandType::HorizontalLine as u8, lw.is_some(), lw),
                PathCommand::VerticalLine(_, lw)      => (PathCommandType::VerticalLine as u8, lw.is_some(), lw),
                PathCommand::CubicBezier(_, lw)       => (PathCommandType::CubicBezier as u8, lw.is_some(), lw),
                PathCommand::ArcCircle(_, lw)         => (PathCommandType::ArcCircle as u8, lw.is_some(), lw),
                PathCommand::ArcEllipse(_, lw)        => (PathCommandType::ArcEllipse as u8, lw.is_some(), lw),
                PathCommand::QuadraticBezier(_, lw)   => (PathCommandType::QuadraticBezier as u8, lw.is_some(), lw),
                PathCommand::ClosePath                             => (PathCommandType::ClosePath as u8, false, &None),
            };

            let mut tag = base_tag & 0b0000_0111;
            if has_lw {
                tag |= 0b0001_0000;
            }
            cursor.write_all(&[tag]).map_err(|_| TinyVgParseError::InvalidCommand)?;

            if let Some(unit) = *lw {
                write_unit(header.scale, cursor, &header.coordinate_range, unit)?;
            }

            match cmd {
                PathCommand::Line(p, _) => {
                    write_point(p, header, cursor)?
                }
                PathCommand::HorizontalLine(u, _) => {
                    write_unit(header.scale, cursor, &header.coordinate_range, *u)?;
                }
                PathCommand::VerticalLine(u, _) => {
                    write_unit(header.scale, cursor, &header.coordinate_range, *u)?;
                }
                PathCommand::CubicBezier(c, _) => {
                    write_point(&c.control_point_0, header, cursor)?;
                    write_point(&c.control_point_1, header, cursor)?;
                    write_point(&c.point_1, header, cursor)?
                }
                PathCommand::ArcCircle(a, _) => {
                    let mut flags = 0;
                    if a.large_arc { flags |= 0b01; }
                    if a.sweep     { flags |= 0b10; }
                    cursor.write_all(&[flags]).map_err(|_| TinyVgParseError::InvalidCommand)?;
                    write_unit(header.scale, cursor, &header.coordinate_range, a.radius)?;
                    write_point(&a.target, header, cursor)?
                }
                PathCommand::ArcEllipse(a, _) => {
                    let mut flags = 0;
                    if a.large_arc { flags |= 0b01; }
                    if a.sweep     { flags |= 0b10; }
                    cursor.write_all(&[flags]).map_err(|_| TinyVgParseError::InvalidCommand)?;
                    write_unit(header.scale, cursor, &header.coordinate_range, a.radius_x)?;
                    write_unit(header.scale, cursor, &header.coordinate_range, a.radius_y)?;
                    write_unit(header.scale, cursor, &header.coordinate_range, a.rotation)?;
                    write_point(&a.target, header, cursor)?
                }
                PathCommand::QuadraticBezier(q, _) => {
                    write_point(&q.control_point, header, cursor)?;
                    write_point(&q.point_1, header, cursor)?
                }
                PathCommand::ClosePath => {}
            }
        }
    }

    Ok(())
}


pub fn write_command_and_primary_style(
    cursor: &mut Cursor<Vec<u8>>,
    command: CommandType,
    style_type: StyleType,
) -> Result<(), TinyVgParseError> {
    let command_u8 = command as u8;
    let style_u8 = style_type as u8;

    if command_u8 > 0b0011_1111 {
        return Err(TinyVgParseError::InvalidCommand);
    }


    let combined = (style_u8 << 6) | (command_u8 & 0b0011_1111);

    cursor
        .write_all(&[combined])
        .map_err(|_| TinyVgParseError::InvalidCommand)?;

    Ok(())
}