use usvg::{Opacity, Paint};
use crate::color_table::{ColorTable, RgbaF32};
use crate::commands::{FlatColored, LinearGradient, Point, RadialGradient, Style};
use crate::common::Unit;

pub(crate) fn set_color(color_table: &mut ColorTable, usvg_color: &usvg::Color, opacity: &Opacity) -> u64 {
    let new_color = RgbaF32(usvg_color.red as f32 / 255.0, usvg_color.green as f32 / 255.0, usvg_color.blue as f32 / 255.0, opacity.get());
    for (index, color) in color_table.iter().enumerate() {
        if *color == new_color {
            return index as u64;
        }
    }

    color_table.push(new_color);
    (color_table.len() - 1) as u64
}

pub(crate) fn usvg_paint_to_tinyvg_style(paint: &Paint, opacity: &Opacity, color_table: &mut ColorTable) -> Style {

    match paint {
        Paint::Color(color) => {
            Style::FlatColor(FlatColored {
                color_index: set_color(color_table, color, opacity),
            })
        }
        Paint::LinearGradient(gradient) => {
            let point_0 = Point::new(Unit(gradient.x1() as f64), Unit(gradient.y1() as f64));
            let point_1 = Point::new(Unit(gradient.x2() as f64), Unit(gradient.y2() as f64));

            let stop_0 = gradient.stops().first().unwrap();
            let stop_1 = gradient.stops().last().unwrap();
            Style::LinearGradient(LinearGradient {
                point_0,
                point_1,
                color_index_0: set_color(color_table, &stop_0.color(), &Opacity::new(stop_0.opacity().get() * opacity.get()).unwrap()),
                color_index_1: set_color(color_table, &stop_1.color(), &Opacity::new(stop_1.opacity().get() * opacity.get()).unwrap()),
            })
        }
        Paint::RadialGradient(gradient) => {
            let (x1, y1) = (gradient.fx(), gradient.fy());
            let (x2, y2) = (gradient.cx(), gradient.cy() + gradient.r().get());

            let point_0 = Point::new(Unit(x1 as f64), Unit(y1 as f64));
            let point_1 = Point::new(Unit(x2 as f64), Unit(y2 as f64));

            let stop_0 = gradient.stops().first().unwrap();
            let stop_1 = gradient.stops().last().unwrap();

            Style::RadialGradient(RadialGradient {
                point_0,
                point_1,
                color_index_0: set_color(color_table, &stop_0.color(), &Opacity::new(stop_0.opacity().get() * opacity.get()).unwrap()),
                color_index_1: set_color(color_table, &stop_1.color(), &Opacity::new(stop_1.opacity().get() * opacity.get()).unwrap()),
            })
        }
        Paint::Pattern(_) => {
            panic!("Pattern is not supported");
        }
    }
}

pub(crate) fn usvg_point_to_tinyvg_point(usvg_point: usvg::tiny_skia_path::Point) -> Point {
    Point::new(Unit(usvg_point.x as f64), Unit(usvg_point.y as f64))
}