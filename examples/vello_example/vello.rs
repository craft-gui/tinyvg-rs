use peniko::color::AlphaColor;
use peniko::kurbo::SvgArc;
use peniko::{Brush, Fill, Gradient};
use std::sync::Arc;
use tinyvg::color_table::ColorTable;
use tinyvg::commands::{DrawCommand, Path, PathCommand, Point, Segment, Style};
use tinyvg::common::Unit;
use tinyvg::TinyVg;
use vello::kurbo::{Affine, BezPath, Line, Stroke};
use vello::peniko::color::palette;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{kurbo, AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;

fn main() {
    let tiger= TinyVg::from_bytes(include_bytes!("../tiger.tvg")).unwrap();
    let app_icon= TinyVg::from_bytes(include_bytes!("../app-icon.tvg")).unwrap();
    let chart = TinyVg::from_bytes(include_bytes!("../chart.tvg")).unwrap();

    let mut app = TinyVgExample {
        context: RenderContext::new(),
        renderers: vec![],
        state: RenderState::Suspended(None),
        scene: Scene::new(),
        tiger,
        tiger_rotation: 0.0,
        app_icon,
        chart
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut app)
        .expect("Couldn't run event loop");
}


#[derive(Debug)]
enum RenderState<'s> {
    Active {
        surface: Box<RenderSurface<'s>>,
        window: Arc<Window>,
    },
    Suspended(Option<Arc<Window>>),
}

struct TinyVgExample<'s> {
    context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    state: RenderState<'s>,
    scene: Scene,
    tiger: TinyVg,
    tiger_rotation: f64,
    app_icon: TinyVg,
    chart: TinyVg
}

fn to_vello_point(point: Point) -> kurbo::Point {
    kurbo::Point::new(point.x.0, point.y.0)
}

fn to_vello_color(color: tinyvg::color_table::RgbaF32) -> Color {
    Color::from(AlphaColor::new([color.0, color.1, color.2, color.3]))
}

impl ApplicationHandler for TinyVgExample<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let RenderState::Suspended(cached_window) = &mut self.state else {
            return;
        };

        let window = cached_window
            .take()
            .unwrap_or_else(|| create_winit_window(event_loop));

        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        );
        let surface = pollster::block_on(surface_future).expect("Error creating surface");

        self.renderers
            .resize_with(self.context.devices.len(), || None);
        self.renderers[surface.dev_id]
            .get_or_insert_with(|| create_vello_renderer(&self.context, &surface));

        self.state = RenderState::Active {
            surface: Box::new(surface),
            window,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (surface, window) = match &mut self.state {
            RenderState::Active { surface, window } if window.id() == window_id => (surface, window),
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                self.context
                    .resize_surface(surface, size.width, size.height);
            }

            WindowEvent::RedrawRequested => {
                let width = surface.config.width;
                let height = surface.config.height;
                self.scene.reset();

                self.tiger_rotation = (self.tiger_rotation + 0.2) % 361.0;

                let center = kurbo::Point { x: self.tiger.header.width as f64 / 2.0, y: self.tiger.header.height as f64 / 2.0 };
                let affine = Affine::IDENTITY.then_scale(1.0).then_rotate_about(self.tiger_rotation.to_radians(), center).then_translate(kurbo::Vec2::new(0.0, 0.0));
                draw_tiny_vg(&mut self.scene, &self.tiger, affine);

                let affine = Affine::IDENTITY.then_scale(1.0).then_translate(kurbo::Vec2::new(self.tiger.header.width as f64, 0.0));
                draw_tiny_vg(&mut self.scene, &self.app_icon, affine);

                let affine = Affine::IDENTITY.then_scale(0.5).then_translate(kurbo::Vec2::new(self.tiger.header.width as f64 + self.app_icon.header.width as f64, 0.0));
                draw_tiny_vg(&mut self.scene, &self.chart, affine);

                let device_handle = &self.context.devices[surface.dev_id];

                let renderer = self.renderers[surface.dev_id].as_mut().unwrap();
                let surface_texture = surface
                    .surface
                    .get_current_texture()
                    .expect("failed to get surface texture");

                renderer
                    .render_to_surface(
                        &device_handle.device,
                        &device_handle.queue,
                        &self.scene,
                        &surface_texture,
                        &vello::RenderParams {
                            base_color: palette::css::WHITE,
                            width,
                            height,
                            antialiasing_method: AaConfig::Msaa16,
                        },
                    )
                    .expect("failed to render to surface");

                let encoder =
                    device_handle
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Surface Blit"),
                        });


                device_handle.queue.submit([encoder.finish()]);
                surface_texture.present();

                device_handle.device.poll(wgpu::Maintain::Poll);
                window.request_redraw();
            }
            _ => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let RenderState::Active { window, .. } = &self.state {
            self.state = RenderState::Suspended(Some(window.clone()));
        }
    }
}

fn create_winit_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attr = Window::default_attributes()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .with_title("TinyVG Example");
    Arc::new(event_loop.create_window(attr).unwrap())
}

fn create_vello_renderer(render_cx: &RenderContext, surface: &RenderSurface<'_>) -> Renderer {
    Renderer::new(
        &render_cx.devices[surface.dev_id].device,
        RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: None,
        },
    )
        .expect("Couldn't create renderer")
}

fn draw_path(scene: &mut Scene, path: &Path, fill_style: &Style, line_width: Option<&Unit>, color_table: &ColorTable, affine: &Affine) {

    let brush = get_brush(fill_style, color_table);
    let mut bezier_path = BezPath::new();

    for segment in &path.segments {
        let mut current = segment.start;
        bezier_path.move_to(to_vello_point(current));

        for path_command in &segment.path_commands {
            match path_command {
                PathCommand::Line(point, _line_width) => {
                    bezier_path.line_to(to_vello_point(*point));
                    current = current.move_to(&point);
                }
                PathCommand::HorizontalLine(horizontal, _line_width) => {
                    let horizontal_end_point = Point {x : *horizontal, y: current.y };
                    bezier_path.line_to(to_vello_point(horizontal_end_point));
                    current = current.move_to(&horizontal_end_point);
                }
                PathCommand::VerticalLine(vertical, _line_width) => {
                    let vertical_end_point = Point {x : current.x, y: *vertical };
                    bezier_path.line_to(to_vello_point(vertical_end_point));
                    current = current.move_to(&vertical_end_point);
                }
                PathCommand::CubicBezier(cubic_bezier, _line_width) => {
                    let end = cubic_bezier.point_1;
                    bezier_path.curve_to(
                        (cubic_bezier.control_point_0.x.0, cubic_bezier.control_point_0.y.0),
                        (cubic_bezier.control_point_1.x.0, cubic_bezier.control_point_1.y.0),
                        (end.x.0, end.y.0)
                    );
                    current = current.move_to(&end);
                }
                PathCommand::ArcCircle(arc_circle, _line_width) => {
                   let arc_start = to_vello_point(current);
                   let arc_end = to_vello_point(arc_circle.target);

                   let arc = SvgArc {
                       from: arc_start,
                       to: arc_end,
                       radii: kurbo::Vec2::new(arc_circle.radius.0, arc_circle.radius.0),
                       x_rotation: 0.0,
                       large_arc: arc_circle.large_arc,
                       sweep: arc_circle.sweep,
                   };

                   let arc = kurbo::Arc::from_svg_arc(&arc);
                   if let Some(arc) = arc {
                       for el in arc.append_iter(0.1) {
                           bezier_path.push(el);
                       }
                   }

                   current = current.move_to(&arc_circle.target);
                }
                PathCommand::ArcEllipse(arc_ellipse, _line_width) => {
                    let arc_start = to_vello_point(current);
                    let arc_end = to_vello_point(arc_ellipse.target);

                    let arc = SvgArc {
                        from: arc_start,
                        to: arc_end,
                        radii: kurbo::Vec2::new(arc_ellipse.radius_x.0, arc_ellipse.radius_y.0),
                        x_rotation: 0.0,
                        large_arc: arc_ellipse.large_arc,
                        sweep: arc_ellipse.sweep,
                    };

                    let arc = kurbo::Arc::from_svg_arc(&arc);
                    if let Some(arc) = arc {
                        for el in arc.append_iter(0.1) {
                            bezier_path.push(el);
                        }
                    }
                    current = current.move_to(&arc_ellipse.target);
                }
                PathCommand::ClosePath => {
                    bezier_path.close_path();
                }
                PathCommand::QuadraticBezier(quadratic_bezier, _line_width) => {
                    let end = quadratic_bezier.point_1;
                    bezier_path.quad_to(
                        (to_vello_point(quadratic_bezier.control_point).x, to_vello_point(quadratic_bezier.control_point).y),
                        (to_vello_point(end).x, to_vello_point(end).y),
                    );

                    current = current.move_to(&end);
                }
            }
        }
    }

    if let Some(line_width) = line_width {
        scene.stroke(
            &Stroke::new(line_width.0),
            *affine,
            &brush,
            None,
            &bezier_path,
        );
    } else {
        scene.fill(
            Fill::EvenOdd,
            *affine,
            &brush,
            None,
            &bezier_path,
        );
    }
}

fn get_brush(fill_style: &Style, color_table: &ColorTable) -> Brush {
    let brush: Brush;

    match fill_style {
        Style::FlatColor(flat_colored) => {
            let color = color_table[flat_colored.color_index as usize];
            brush = Brush::Solid(to_vello_color(color));
        }
        Style::LinearGradient(linear_gradient) => {
            let color_0 = color_table[linear_gradient.color_index_0 as usize];
            let color_1 = color_table[linear_gradient.color_index_1 as usize];

            let start = to_vello_point(linear_gradient.point_0);
            let end = to_vello_point(linear_gradient.point_1);

            let linear = Gradient::new_linear(
                start,
                end
            ).with_stops([to_vello_color(color_0), to_vello_color(color_1)]);
            brush = Brush::Gradient(linear)
        }
        Style::RadialGradient(radial_gradient) => {
            let color_0 = color_table[radial_gradient.color_index_0 as usize];
            let color_1 = color_table[radial_gradient.color_index_1 as usize];

            let center = to_vello_point(radial_gradient.point_0);
            let edge = to_vello_point(radial_gradient.point_1);
            let radius = center.distance(edge);

            let radial = Gradient::new_radial(
                center,
                radius as f32
            ).with_stops([to_vello_color(color_0), to_vello_color(color_1)]);

            brush = Brush::Gradient(radial)
        }
    }
    brush
}

fn draw_tiny_vg(scene: &mut Scene, tiny_vg: &TinyVg, affine: Affine) {

    for command in &tiny_vg.draw_commands {
        match command {
            DrawCommand::FillPolygon(data) => {
                let start = data.points[0];
                let mut segment = Segment {
                    start,
                    path_commands: vec![],
                };
                for point in &data.points {
                    segment.path_commands.push(PathCommand::Line(*point, None));
                }
                segment.path_commands.push(PathCommand::ClosePath);
                let path = Path {
                    segments: vec![segment],
                };
                draw_path(scene, &path, &data.style, None, &tiny_vg.color_table, &affine);
            }
            DrawCommand::FillRectangles(data) => {
                let brush = get_brush(&data.style, &tiny_vg.color_table);
                for rectangle in &data.rectangles {
                    let rectangle = kurbo::Rect::new(rectangle.x.0, rectangle.y.0, rectangle.height.0, rectangle.height.0);
                    scene.fill(Fill::EvenOdd, affine, &brush, None, &rectangle);
                }
            }
            DrawCommand::FillPath(data) => {
                draw_path(scene, &data.path, &data.style, None, &tiny_vg.color_table, &affine);
            }
            DrawCommand::DrawLines(data) => {
                let brush = get_brush(&data.line_style, &tiny_vg.color_table);

                for line in &data.lines {
                    let line = Line::new(to_vello_point(line.start), to_vello_point(line.end));
                    scene.stroke(&Stroke::new(data.line_width.0), affine, &brush, None, &line);
                }
            }
            DrawCommand::DrawLineLoop(data) => {
                let brush = get_brush(&data.line_style, &tiny_vg.color_table);

                let mut start = data.points[0];
                for point in &data.points {
                    let line = Line::new(to_vello_point(start.clone()), to_vello_point(*point));
                    scene.stroke(&Stroke::new(data.line_width.0), affine, &brush, None, &line);
                    start = point.clone();
                }
            }
            DrawCommand::DrawLineStrip(data) => {
                let brush = get_brush(&data.style, &tiny_vg.color_table);

                let mut start = data.points[0];
                for point in &data.points {
                    let line = Line::new(to_vello_point(start.clone()), to_vello_point(*point));
                    scene.stroke(&Stroke::new(data.line_width.0), affine, &brush, None, &line);
                    start = point.clone();
                }
            }
            DrawCommand::DrawLinePath(data) => {
                draw_path(scene, &data.path, &data.style, Some(&data.line_width), &tiny_vg.color_table, &affine);
            }
            DrawCommand::OutlineFillPolygon(data) => {
                let start = data.points[0];
                let mut segment = Segment {
                    start,
                    path_commands: vec![],
                };
                for point in &data.points {
                    segment.path_commands.push(PathCommand::Line(*point, None));
                }
                segment.path_commands.push(PathCommand::ClosePath);
                let path = Path {
                    segments: vec![segment],
                };
                draw_path(scene, &path, &data.fill_style, None, &tiny_vg.color_table, &affine);
                draw_path(scene, &path, &data.line_style, Some(&data.line_width), &tiny_vg.color_table, &affine);
            }
            DrawCommand::OutlineFillRectangles(data) => {
                let fill_brush = get_brush(&data.fill_style, &tiny_vg.color_table);
                let line_brush = get_brush(&data.line_style, &tiny_vg.color_table);
                for rectangle in &data.rectangles {
                    let rectangle = kurbo::Rect::new(rectangle.x.0, rectangle.y.0, rectangle.height.0, rectangle.height.0);
                    scene.fill(Fill::EvenOdd, affine, &fill_brush, None, &rectangle);
                    scene.stroke(&Stroke::new(data.line_width.0), affine, &line_brush, None, &rectangle);
                }
            }
            DrawCommand::OutlineFillPath(data) => {
                draw_path(scene, &data.path, &data.fill_style, None, &tiny_vg.color_table, &affine);
                draw_path(scene, &data.path, &data.line_style, Some(&data.line_width), &tiny_vg.color_table, &affine);
            },
            // This command only provides metadata for accessibility or text selection tools for the position and content
            // of text. A renderer can safely ignore this command since it must not have any effect on the resulting
            // graphic
            DrawCommand::TextHint(_data) => {}
        }
    }
}
