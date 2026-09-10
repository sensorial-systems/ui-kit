use ui_kit_core::{Material, Rect};
use ui_kit_immediate::DrawCommand;

/// Scale immediate mode draw commands from logical points to physical screen pixels.
pub fn scale_commands(commands: &mut [DrawCommand], scale: f32) {
    if (scale - 1.0).abs() < f32::EPSILON {
        return;
    }
    for cmd in commands {
        cmd.rect = Rect {
            x: cmd.rect.x * scale,
            y: cmd.rect.y * scale,
            width: cmd.rect.width * scale,
            height: cmd.rect.height * scale,
        };
        if let Some(clip) = &mut cmd.clip {
            clip.x *= scale;
            clip.y *= scale;
            clip.width *= scale;
            clip.height *= scale;
        }
        cmd.style.radius *= scale;
        cmd.style.font_size *= scale;
        cmd.style.text_padding *= scale;
        cmd.style.stroke_width *= scale;
        if cmd.kind == "shadow" {
            cmd.value *= scale;
        }
        if let Material::Custom { name, parameters } = &mut cmd.style.material {
            if name == "gallery.colored-triangles" {
                for vertex in parameters.chunks_exact_mut(6) {
                    vertex[0] *= scale;
                    vertex[1] *= scale;
                }
            } else if name == "gallery.triangles" {
                for v in parameters {
                    *v *= scale;
                }
            }
        }
    }
}
