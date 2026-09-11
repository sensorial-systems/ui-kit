//! XY Graph widget implementation for immediate UI.

use super::response::XyGraphResponse;
use ui_kit_core::{Color, Point, Rect, Style, XyPlot, XyPoint};
use crate::immediate::Ui;

/// Immediate mode XY graph widget.
#[derive(Clone, Debug, PartialEq)]
pub struct XyGraphWidget {
    pub background_color: Color,
    pub border_color: Color,
    pub grid_color: Color,
    pub text_color: Color,
    pub corner_radius: f32,
}

impl Default for XyGraphWidget {
    fn default() -> Self {
        Self {
            background_color: [0.07, 0.08, 0.10, 0.95],
            border_color: [0.20, 0.23, 0.28, 0.6],
            grid_color: [0.22, 0.25, 0.30, 0.45],
            text_color: [0.55, 0.60, 0.68, 1.0],
            corner_radius: 6.0,
        }
    }
}

impl XyGraphWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render and interact with an `XyPlot`.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        outer_rect: Rect,
        plot: &mut XyPlot,
    ) -> XyGraphResponse {
        let mut response = XyGraphResponse::default();

        // 1. Render main container panel
        let orig_style = ui.style.clone();
        ui.style = Style {
            fill: self.background_color,
            stroke: self.border_color,
            stroke_width: 1.0,
            radius: self.corner_radius,
            ..orig_style.clone()
        };
        ui.panel(outer_rect);
        ui.style = orig_style.clone();

        // 2. Compute inner plot area
        let has_y_labels = !plot.y_axis.ticks.is_empty();
        let has_x_labels = !plot.x_axis.ticks.is_empty();
        let left_pad = if has_y_labels { 32.0 } else { 6.0 };
        let bottom_pad = if has_x_labels { 18.0 } else { 6.0 };
        let top_pad = if plot.title.is_some() { 18.0 } else { 6.0 };
        let right_pad = 8.0;

        let plot_rect = Rect {
            x: outer_rect.x + left_pad,
            y: outer_rect.y + top_pad,
            width: (outer_rect.width - left_pad - right_pad).max(10.0),
            height: (outer_rect.height - top_pad - bottom_pad).max(10.0),
        };

        // Track hovered pointer in data coordinates
        if let Some(pointer) = ui.input().pointer {
            if plot_rect.contains(pointer) {
                response.hovered_data_point = Some(plot.screen_to_data(pointer, plot_rect));
            }
        }

        // 3. Render title if specified
        if let Some(title) = &plot.title {
            let title_rect = Rect {
                x: outer_rect.x + 8.0,
                y: outer_rect.y + 3.0,
                width: outer_rect.width - 16.0,
                height: 14.0,
            };
            self.draw_text(ui, title_rect, title, 11.0, [0.8, 0.85, 0.9, 1.0]);
        }

        // 4. Render Shaded Regions (e.g. cut band)
        for region in &plot.regions {
            let p0 = plot.data_to_screen(XyPoint::new(region.x_min, 0.0), plot_rect);
            let p1 = plot.data_to_screen(XyPoint::new(region.x_max, 0.0), plot_rect);
            let rx = p0.x.min(p1.x).max(plot_rect.x);
            let rw = (p0.x.max(p1.x) - rx).min(plot_rect.x + plot_rect.width - rx);
            if rw > 0.0 {
                let region_rect = Rect {
                    x: rx,
                    y: plot_rect.y,
                    width: rw,
                    height: plot_rect.height,
                };
                self.draw_box(ui, region_rect, region.color, [0.0; 4], 0.0);

                if let Some(label) = &region.label {
                    let label_rect = Rect {
                        x: rx + 4.0,
                        y: plot_rect.y + 4.0,
                        width: (rw - 8.0).max(30.0),
                        height: 12.0,
                    };
                    self.draw_text(ui, label_rect, label, 9.5, [region.color[0], region.color[1], region.color[2], 0.85]);
                }
            }
        }

        // 5. Grid lines & Axis ticks
        // X-axis ticks
        for tick in &plot.x_axis.ticks {
            let sx = plot.data_to_screen(XyPoint::new(tick.value, 0.0), plot_rect).x;
            if sx >= plot_rect.x && sx <= plot_rect.x + plot_rect.width {
                if plot.x_axis.show_grid {
                    let line_rect = Rect {
                        x: sx.round(),
                        y: plot_rect.y,
                        width: 1.0,
                        height: plot_rect.height,
                    };
                    self.draw_box(ui, line_rect, self.grid_color, [0.0; 4], 0.0);
                }
                if !tick.label.is_empty() {
                    let lbl_rect = Rect {
                        x: sx - 16.0,
                        y: plot_rect.y + plot_rect.height + 2.0,
                        width: 32.0,
                        height: 14.0,
                    };
                    self.draw_text(ui, lbl_rect, &tick.label, 9.5, self.text_color);
                }
            }
        }

        // Y-axis ticks
        for tick in &plot.y_axis.ticks {
            let sy = plot.data_to_screen(XyPoint::new(0.0, tick.value), plot_rect).y;
            if sy >= plot_rect.y && sy <= plot_rect.y + plot_rect.height {
                if plot.y_axis.show_grid {
                    let line_rect = Rect {
                        x: plot_rect.x,
                        y: sy.round(),
                        width: plot_rect.width,
                        height: 1.0,
                    };
                    self.draw_box(ui, line_rect, self.grid_color, [0.0; 4], 0.0);
                }
                if !tick.label.is_empty() {
                    let lbl_rect = Rect {
                        x: outer_rect.x + 2.0,
                        y: sy - 7.0,
                        width: left_pad - 4.0,
                        height: 14.0,
                    };
                    self.draw_text(ui, lbl_rect, &tick.label, 9.5, self.text_color);
                }
            }
        }

        // 6. Render Series
        for series in &plot.series {
            if series.points.is_empty() {
                continue;
            }

            // Map points to screen coordinates
            let screen_points: Vec<Point> = series
                .points
                .iter()
                .map(|&p| plot.data_to_screen(p, plot_rect))
                .collect();

            // 6a. Filled Area Under Curve
            if let Some(fill_color) = series.fill_color {
                let base_y = plot
                    .data_to_screen(XyPoint::new(0.0, series.fill_baseline), plot_rect)
                    .y
                    .clamp(plot_rect.y, plot_rect.y + plot_rect.height);

                for i in 0..screen_points.len().saturating_sub(1) {
                    let p0 = screen_points[i];
                    let p1 = screen_points[i + 1];
                    let col_x = p0.x.min(p1.x);
                    let col_w = (p1.x - p0.x).abs().max(1.0);
                    let top_y = p0.y.min(p1.y).clamp(plot_rect.y, plot_rect.y + plot_rect.height);
                    let col_h = (base_y - top_y).max(0.0);

                    if col_h > 0.0 {
                        let strip_rect = Rect {
                            x: col_x,
                            y: top_y,
                            width: col_w,
                            height: col_h,
                        };
                        self.draw_box(ui, strip_rect, fill_color, [0.0; 4], 0.0);
                    }
                }
            }

            // 6b. Line Stroke
            let half_stroke = (series.stroke_width * 0.5).max(0.5);
            for i in 0..screen_points.len().saturating_sub(1) {
                let p0 = screen_points[i];
                let p1 = screen_points[i + 1];
                let seg_x = p0.x.min(p1.x);
                let seg_w = (p1.x - p0.x).abs().max(1.0);
                let seg_y = (p0.y.min(p1.y) - half_stroke).max(plot_rect.y);
                let seg_h = ((p1.y - p0.y).abs() + series.stroke_width).max(1.5);

                let seg_rect = Rect {
                    x: seg_x,
                    y: seg_y,
                    width: seg_w,
                    height: seg_h,
                };
                self.draw_box(ui, seg_rect, series.stroke_color, [0.0; 4], 0.0);
            }
        }

        // 7. Interactive Handles
        for i in 0..plot.handles.len() {
            let handle_pos = plot.handles[i].position;
            let handle_radius = plot.handles[i].radius;
            let handle_color = plot.handles[i].color;
            let handle_label = plot.handles[i].label.clone();
            let handle_id_str = plot.handles[i].id.clone();

            let h_pos = plot.data_to_screen(handle_pos, plot_rect);
            let hit_size = (handle_radius * 2.5 + 8.0).max(18.0);
            let hit_rect = Rect {
                x: h_pos.x - hit_size * 0.5,
                y: h_pos.y - hit_size * 0.5,
                width: hit_size,
                height: hit_size,
            };

            let handle_widget_id = format!("{id}.handle.{handle_id_str}");
            let h_resp = ui.interact(&handle_widget_id, hit_rect);

            if h_resp.hovered {
                response.hovered = true;
            }

            let mut active_pos = handle_pos;
            if h_resp.active {
                response.active = true;
                if let Some(pointer) = ui.input().pointer {
                    let new_data = plot.screen_to_data(pointer, plot_rect);
                    let clamped = plot.handles[i].clamp_position(new_data);
                    if plot.handles[i].position != clamped {
                        plot.handles[i].position = clamped;
                        active_pos = clamped;
                        response.changed = true;
                        response.dragged_handle = Some((handle_id_str, clamped));
                    }
                }
            }

            // Draw handle halo when hovered or active
            let active_h_pos = plot.data_to_screen(active_pos, plot_rect);
            if h_resp.hovered || h_resp.active {
                let halo_radius = handle_radius + 3.0;
                let halo_rect = Rect {
                    x: active_h_pos.x - halo_radius,
                    y: active_h_pos.y - halo_radius,
                    width: halo_radius * 2.0,
                    height: halo_radius * 2.0,
                };
                let mut halo_color = handle_color;
                halo_color[3] = if h_resp.active { 0.5 } else { 0.25 };
                self.draw_box(ui, halo_rect, halo_color, [0.0; 4], halo_radius);
            }

            // Draw solid handle node
            let node_radius = handle_radius;
            let node_rect = Rect {
                x: active_h_pos.x - node_radius,
                y: active_h_pos.y - node_radius,
                width: node_radius * 2.0,
                height: node_radius * 2.0,
            };
            self.draw_box(ui, node_rect, handle_color, [1.0, 1.0, 1.0, 0.9], node_radius);

            // Draw center white dot
            let center_radius = 2.0;
            let center_rect = Rect {
                x: active_h_pos.x - center_radius,
                y: active_h_pos.y - center_radius,
                width: center_radius * 2.0,
                height: center_radius * 2.0,
            };
            self.draw_box(ui, center_rect, [1.0, 1.0, 1.0, 1.0], [0.0; 4], center_radius);

            // Floating value label / badge
            if let Some(lbl) = &handle_label {
                let badge_w = (lbl.len() as f32 * 6.5 + 10.0).max(40.0);
                let badge_rect = Rect {
                    x: (active_h_pos.x - badge_w * 0.5).clamp(outer_rect.x + 2.0, outer_rect.x + outer_rect.width - badge_w - 2.0),
                    y: active_h_pos.y - handle_radius - 17.0,
                    width: badge_w,
                    height: 14.0,
                };
                self.draw_box(ui, badge_rect, [0.05, 0.06, 0.08, 0.85], [0.3, 0.35, 0.42, 0.6], 3.0);
                self.draw_text(ui, badge_rect, lbl, 9.0, [0.95, 0.95, 0.95, 1.0]);
            }
        }

        response
    }

    fn draw_box(
        &self,
        ui: &mut Ui,
        rect: Rect,
        fill: Color,
        stroke: Color,
        radius: f32,
    ) {
        let orig = ui.style.clone();
        ui.style = Style {
            fill,
            stroke,
            stroke_width: if stroke[3] > 0.0 { 1.0 } else { 0.0 },
            radius,
            ..orig.clone()
        };
        ui.panel(rect);
        ui.style = orig;
    }

    fn draw_text(
        &self,
        ui: &mut Ui,
        rect: Rect,
        text: &str,
        size: f32,
        color: Color,
    ) {
        let orig = ui.style.clone();
        ui.style = Style {
            foreground: color,
            font_size: size,
            font_weight: 400,
            text_padding: 0.0,
            fill: [0.0; 4],
            stroke: [0.0; 4],
            stroke_width: 0.0,
            ..orig.clone()
        };
        ui.label(rect, text);
        ui.style = orig;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immediate::Input;
    use ui_kit_core::{XyAxis, XyHandle, XyRange, XySeries};

    #[test]
    fn xy_graph_emits_draw_commands_and_handles_interact() {
        let mut ui = Ui::default();
        ui.begin(Input::default());

        let mut plot = XyPlot::new(
            XyAxis::new(XyRange::new(0.0, 10.0)).with_uniform_ticks(5),
            XyAxis::new(XyRange::new(0.0, 1.0)).with_uniform_ticks(3),
        )
        .with_series(XySeries::from_function("line", 0.0, 10.0, 11, |x| x * 0.1))
        .with_handle(XyHandle::new("cutoff", XyPoint::new(5.0, 0.5)));

        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };

        let response = ui.xy_graph("test_graph", rect, &mut plot);
        assert!(!response.hovered);
        assert!(!ui.commands.is_empty());
    }
}
