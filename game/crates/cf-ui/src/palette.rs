use bevy::prelude::*;

pub(crate) fn palette_focus_ring_clear() -> Color {
    Color::srgba(0.0, 0.0, 0.0, 0.0)
}

pub(crate) fn palette_focus_ring(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(1.0, 0.85, 0.0)
    }
}

pub(crate) fn palette_text(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(0.96, 0.96, 0.92)
    }
}

pub(crate) fn palette_strip_bg(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgba(0.0, 0.0, 0.0, 1.0)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.45)
    }
}

pub(crate) fn palette_banner_bg(high_contrast: bool, severity: &str) -> Color {
    if high_contrast {
        Color::srgba(0.0, 0.0, 0.0, 1.0)
    } else {
        match severity {
            "critical" => Color::srgba(0.7, 0.05, 0.05, 0.85),
            "warning" => Color::srgba(0.7, 0.5, 0.0, 0.85),
            _ => Color::srgba(0.0, 0.0, 0.0, 0.6),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_helpers_swap_for_high_contrast() {
        let normal = palette_text(false);
        let hc = palette_text(true);
        assert_ne!(normal, hc);
        let normal_bg = palette_strip_bg(false);
        let hc_bg = palette_strip_bg(true);
        assert_ne!(normal_bg, hc_bg);
        let hc_critical = palette_banner_bg(true, "critical");
        let normal_critical = palette_banner_bg(false, "critical");
        assert_ne!(hc_critical, normal_critical);
    }
}
