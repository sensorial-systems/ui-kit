use ui_kit_core::{Material, Rect};
use ui_kit_immediate::vector::{COLORED_TRIANGLES, DOTS, TRIANGLES};
use ui_kit_immediate::DrawCommand;

/// Scale a command's geometry and styles by `scale`: its rect (then moved
/// by `offset`), radii, type, padding, strokes, shadow blur and mesh
/// vertices. The clip is left alone.
fn scale_command(cmd: &mut DrawCommand, scale: f32, offset: (f32, f32)) {
    cmd.rect = Rect {
        x: cmd.rect.x * scale + offset.0,
        y: cmd.rect.y * scale + offset.1,
        width: cmd.rect.width * scale,
        height: cmd.rect.height * scale,
    };
    cmd.style.radius *= scale;
    if let Some(radii) = &mut cmd.style.corner_radii {
        for radius in radii {
            *radius *= scale;
        }
    }
    cmd.style.font_size *= scale;
    cmd.style.text_padding *= scale;
    cmd.style.stroke_width *= scale;
    if cmd.kind == "shadow" {
        cmd.value *= scale;
    }
    if let Material::Glass { blur } = &mut cmd.style.material {
        *blur *= scale;
    }
    if let Material::Custom { name, parameters } = &mut cmd.style.material {
        if name == COLORED_TRIANGLES {
            for vertex in parameters.chunks_exact_mut(6) {
                vertex[0] *= scale;
                vertex[1] *= scale;
            }
        } else if name == TRIANGLES || name == DOTS {
            for v in parameters {
                *v *= scale;
            }
        }
    }
}

/// Take commands drawn on a zoomed or panned canvas
/// ([`Ui::with_transform`](ui_kit_immediate::Ui::with_transform)) to the
/// screen, so every command is in screen units.
pub fn resolve_transforms(commands: &mut [DrawCommand]) {
    for cmd in commands {
        if let Some(transform) = cmd.transform.take() {
            scale_command(cmd, transform.scale, (transform.offset_x, transform.offset_y));
        }
    }
}

/// Scale immediate mode draw commands from logical points to physical screen
/// pixels, taking commands drawn on a canvas to the screen first.
pub fn scale_commands(commands: &mut [DrawCommand], scale: f32) {
    resolve_transforms(commands);
    if (scale - 1.0).abs() < f32::EPSILON {
        return;
    }
    for cmd in commands {
        scale_command(cmd, scale, (0.0, 0.0));
        if let Some(clip) = &mut cmd.clip {
            clip.x *= scale;
            clip.y *= scale;
            clip.width *= scale;
            clip.height *= scale;
        }
    }
}
