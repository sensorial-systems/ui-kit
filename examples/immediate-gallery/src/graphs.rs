use super::*;
#[derive(Default)]
pub struct GraphCamera {
    pub zoom: f32,
    pub pan: Point,
    drag: Option<(Point, Point)>,
}
impl GraphCamera {
    fn controls(&mut self, p: &mut Painter, id: &str, rect: Rect) {
        if self.zoom == 0.0 {
            self.zoom = 1.0;
        }
        for (i, label) in ["−", "+", "Fit"].iter().enumerate() {
            if p.button(
                &format!("{id}-zoom-{i}"),
                r(
                    rect.x + rect.width - 142.0 + i as f32 * 44.0,
                    rect.y + 14.0,
                    40.0,
                    30.0,
                ),
                label,
                1,
                false,
            ) {
                match i {
                    0 => self.zoom = (self.zoom / 1.2).max(0.35),
                    1 => self.zoom = (self.zoom * 1.2).min(3.0),
                    _ => {
                        self.zoom = 1.0;
                        self.pan = Point::default();
                    }
                }
            }
        }
    }
    fn update_drag(&mut self, input: &Input) {
        if input.down {
            if let (Some((start, pan)), Some(pt)) = (self.drag, input.pointer) {
                self.pan = Point::new(pan.x + pt.x - start.x, pan.y + pt.y - start.y);
            }
        } else {
            self.drag = None;
        }
    }
    fn drag(&mut self, p: &mut Painter, id: &str, area: Rect) {
        let response = p.interact(&format!("{id}-pan"), area);
        if response.active {
            if let Some(pt) = p.input.pointer {
                let (start, pan) = *self.drag.get_or_insert((pt, self.pan));
                self.pan = Point::new(pan.x + pt.x - start.x, pan.y + pt.y - start.y);
            }
        } else {
            self.drag = None;
        }
    }
}
impl Painter<'_> {
    fn arrow(&mut self, from: Point, to: Point, color: Color) {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = dx.hypot(dy);
        if len < 0.001 {
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        self.triangles(
            &[
                to,
                Point::new(to.x - ux * 8.0 - uy * 4.0, to.y - uy * 8.0 + ux * 4.0),
                Point::new(to.x - ux * 8.0 + uy * 4.0, to.y - uy * 8.0 - ux * 4.0),
            ],
            color,
        );
    }
    fn animated_line(&mut self, a: Point, b: Point, color: Color, time: f32) {
        self.line(a.x, a.y, b.x, b.y, color, 1.5);
        let len = (b.x - a.x).hypot(b.y - a.y);
        if len < 1.0 {
            return;
        }
        let t = (time * 28.0).rem_euclid(18.0);
        let count = (len / 18.0).ceil() as usize;
        for i in 0..count {
            let f = ((i as f32 * 18.0 + t) / len).min(1.0);
            self.box_(
                r(
                    a.x + (b.x - a.x) * f - 1.5,
                    a.y + (b.y - a.y) * f - 1.5,
                    3.0,
                    3.0,
                ),
                color,
                [0.0; 4],
                1.5,
            );
        }
    }
}
impl Gallery {
    pub(super) fn network_graph(&mut self, p: &mut Painter, rect: Rect, time: f32) {
        use ui_kit_core::graph::{circle_connection, network_layout, NetworkNode};
        p.card(rect);
        p.label(
            rect.x + 20.0,
            rect.y + 18.0,
            rect.width - 200.0,
            30.0,
            "Network Graph (Mesh Topology)",
            18.0,
            600,
            p.palette.fg,
        );
        p.label(
            rect.x + 20.0,
            rect.y + 52.0,
            rect.width - 40.0,
            24.0,
            "Hub-and-spoke automatic circular projection with direct straight mesh links.",
            13.0,
            400,
            p.palette.muted,
        );
        self.network_camera.update_drag(p.input);
        self.network_camera.controls(p, "network", rect);
        let ids = ["gateway", "auth", "payment", "db", "cache"];
        let labels = ["Gateway", "Auth Svc", "Billing", "Shared DB", "Redis"];
        let colors = [BLUE, GREEN, RED, YELLOW, BLUE];
        let edges = [
            (0, 1, BLUE, true, "Active"),
            (0, 2, RED, false, ""),
            (1, 3, GREEN, false, ""),
            (2, 3, YELLOW, false, ""),
            (3, 4, BLUE, true, "Sync"),
            (0, 4, p.palette.muted, false, ""),
        ];
        let nodes: Vec<_> = ids
            .iter()
            .map(|id| NetworkNode {
                id: (*id).into(),
                position: None,
            })
            .collect();
        let connections: Vec<_> = edges
            .iter()
            .map(|(a, b, _, _, _)| (ids[*a].into(), ids[*b].into()))
            .collect();
        let layout = network_layout(&nodes, &connections, Point::new(450.0, 210.0), 135.0);
        let area = r(
            rect.x + 20.0,
            rect.y + 88.0,
            rect.width - 40.0,
            rect.height - 108.0,
        );
        let scale = (area.width / 900.0).min(area.height / 420.0) * self.network_camera.zoom;
        let transform = |pt: Point| {
            Point::new(
                area.x + area.width * 0.5 + (pt.x - 450.0) * scale + self.network_camera.pan.x,
                area.y + area.height * 0.5 + (pt.y - 210.0) * scale + self.network_camera.pan.y,
            )
        };
        let points: Vec<_> = ids
            .iter()
            .map(|id| transform(layout.positions[*id]))
            .collect();
        let previous = p.ui.clip;
        p.ui.clip = Some(previous.unwrap_or(p.viewport).intersect(p.screen(area)));
        for (a, b, color, animated, label) in edges {
            let from = circle_connection(points[a], points[b], 35.0 * scale);
            let to = circle_connection(points[b], points[a], 35.0 * scale);
            if animated {
                p.animated_line(from, to, color, time);
                p.arrow(to, from, color);
            } else {
                p.line(from.x, from.y, to.x, to.y, color, 1.5);
            }
            p.arrow(from, to, color);
            if !label.is_empty() {
                let rr = r(
                    (from.x + to.x) * 0.5 - 24.0,
                    (from.y + to.y) * 0.5 - 10.0,
                    48.0,
                    20.0,
                );
                p.box_(rr, p.palette.card, [0.0; 4], 4.0);
                p.label(rr.x, rr.y, rr.width, rr.height, label, 11.0, 500, color);
            }
        }
        for (i, pt) in points.iter().enumerate() {
            let node = r(
                pt.x - 35.0 * scale,
                pt.y - 35.0 * scale,
                70.0 * scale,
                70.0 * scale,
            );
            if p.button(
                &format!("network-node-{}", ids[i]),
                node,
                labels[i],
                if self.network_selected == ids[i] {
                    5
                } else {
                    2
                },
                false,
            ) {
                self.network_selected = ids[i].into();
            }
            let style = &mut p.ui.commands.last_mut().unwrap().style;
            style.radius = node.width * 0.5;
            style.stroke = colors[i];
            style.font_size = 12.0 * scale;
            style.text_padding = 3.0 * scale;
        }
        self.network_camera.drag(p, "network", area);
        p.ui.clip = previous;
    }
    pub(super) fn flow_graph(&mut self, p: &mut Painter, rect: Rect, time: f32) {
        p.card(rect);
        p.label(
            rect.x + 20.0,
            rect.y + 18.0,
            rect.width - 200.0,
            30.0,
            "Flow Graph (CI/CD Pipeline)",
            18.0,
            600,
            p.palette.fg,
        );
        self.flow_camera.update_drag(p.input);
        self.flow_camera.controls(p, "flow", rect);
        let area = r(
            rect.x + 20.0,
            rect.y + 68.0,
            rect.width - 40.0,
            rect.height - 88.0,
        );
        let scale = (area.width / 900.0).min(area.height / 320.0) * self.flow_camera.zoom;
        let nodes = [
            ("checkout", "1. Checkout", "Source Code", 90.0, 160.0, GREEN),
            (
                "lint",
                "2a. Lint & Format",
                "Cargo clippy",
                320.0,
                80.0,
                GREEN,
            ),
            ("test", "2b. Unit Tests", "Cargo test", 320.0, 240.0, GREEN),
            (
                "build",
                "3. Build Artifact",
                "Cargo build --release",
                550.0,
                160.0,
                BLUE,
            ),
            (
                "deploy",
                "4. Deploy",
                "Deploy to AWS",
                780.0,
                160.0,
                p.palette.muted,
            ),
        ];
        let rects: Vec<_> = nodes
            .iter()
            .map(|(_, _, _, x, y, _)| {
                r(
                    area.x + area.width * 0.5 + (*x - 450.0) * scale + self.flow_camera.pan.x
                        - 85.0 * scale,
                    area.y + area.height * 0.5 + (*y - 160.0) * scale + self.flow_camera.pan.y
                        - 32.0 * scale,
                    170.0 * scale,
                    64.0 * scale,
                )
            })
            .collect();
        let previous = p.ui.clip;
        p.ui.clip = Some(previous.unwrap_or(p.viewport).intersect(p.screen(area)));
        for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)] {
            let a_rect = rects[a];
            let b_rect = rects[b];
            let from = Point::new(a_rect.x + a_rect.width, a_rect.y + a_rect.height * 0.5);
            let to = Point::new(b_rect.x, b_rect.y + b_rect.height * 0.5);
            let mid = (from.x + to.x) * 0.5;
            let color = if a == 0 { GREEN } else { BLUE };
            for (start, end) in [
                (from, Point::new(mid, from.y)),
                (Point::new(mid, from.y), Point::new(mid, to.y)),
                (Point::new(mid, to.y), to),
            ] {
                if a == 0 {
                    p.animated_line(start, end, color, time);
                } else {
                    p.line(start.x, start.y, end.x, end.y, color, 1.5);
                }
            }
            p.arrow(Point::new(mid, to.y), to, color);
            if b == 3 {
                p.label(
                    mid - 20.0,
                    to.y - 22.0,
                    48.0,
                    20.0,
                    "Passed",
                    10.0,
                    500,
                    p.palette.muted,
                );
            }
        }
        for (i, (id, title, description, _, _, color)) in nodes.iter().enumerate() {
            let rr = rects[i];
            let response = p.interact(&format!("flow-node-{id}"), rr);
            if response.clicked {
                self.flow_selected = (*id).into();
            }
            p.box_(
                rr,
                p.palette.card,
                if self.flow_selected == *id {
                    BLUE
                } else {
                    *color
                },
                8.0 * scale,
            );
            p.label(
                rr.x + 10.0 * scale,
                rr.y + 8.0 * scale,
                rr.width - 24.0 * scale,
                24.0 * scale,
                title,
                13.0 * scale,
                600,
                p.palette.fg,
            );
            p.label(
                rr.x + 10.0 * scale,
                rr.y + 34.0 * scale,
                rr.width - 20.0 * scale,
                20.0 * scale,
                description,
                11.0 * scale,
                400,
                p.palette.muted,
            );
            p.box_(
                r(
                    rr.x + rr.width - 12.0 * scale,
                    rr.y + 10.0 * scale,
                    5.0 * scale,
                    5.0 * scale,
                ),
                *color,
                [0.0; 4],
                2.5 * scale,
            );
        }
        self.flow_camera.drag(p, "flow", area);
        p.ui.clip = previous;
    }
}
