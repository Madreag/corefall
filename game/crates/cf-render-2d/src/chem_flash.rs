use bevy::prelude::Resource;

pub const CHEM_FLASH_MAX_DURATION_MS: u32 = 600;
pub const CHEM_FLASH_PEAK_RADIUS_PX: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChemFlash {
    pub origin: [f32; 2],
    pub color_rgba: [u8; 4],
    pub age_ms: u32,
    pub lifetime_ms: u32,
    pub peak_radius_px: f32,
}

impl ChemFlash {
    pub fn is_expired(&self) -> bool {
        self.age_ms >= self.lifetime_ms.min(CHEM_FLASH_MAX_DURATION_MS)
    }

    pub fn advance(&mut self, dt_ms: u32) {
        self.age_ms = self.age_ms.saturating_add(dt_ms);
    }

    pub fn intensity(&self) -> f32 {
        let life = self.lifetime_ms.min(CHEM_FLASH_MAX_DURATION_MS).max(1) as f32;
        let t = (self.age_ms as f32 / life).clamp(0.0, 1.0);
        (1.0 - t).powi(2)
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ChemFlashState {
    pub flashes: Vec<ChemFlash>,
}

impl ChemFlashState {
    pub fn spawn(&mut self, origin: [f32; 2], color_rgba: [u8; 4], energy_release_j: f32) {
        let lifetime_ms = ((energy_release_j.abs() / 5000.0).clamp(50.0, CHEM_FLASH_MAX_DURATION_MS as f32)) as u32;
        let peak_radius_px = (energy_release_j.abs() / 50000.0)
            .sqrt()
            .clamp(4.0, CHEM_FLASH_PEAK_RADIUS_PX);
        self.flashes.push(ChemFlash {
            origin,
            color_rgba,
            age_ms: 0,
            lifetime_ms,
            peak_radius_px,
        });
    }

    pub fn tick(&mut self, dt_ms: u32) {
        for f in self.flashes.iter_mut() {
            f.advance(dt_ms);
        }
        self.flashes.retain(|f| !f.is_expired());
    }
}

pub fn parse_flash_color_hex(hex: &str) -> Option<[u8; 4]> {
    let s = hex.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some([r, g, b, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_terminates_within_max_duration() {
        let mut s = ChemFlashState::default();
        s.spawn([0.0, 0.0], [255, 200, 0, 255], 1_850_000.0);
        s.tick(CHEM_FLASH_MAX_DURATION_MS + 50);
        assert!(s.flashes.is_empty());
    }

    #[test]
    fn intensity_decays_to_zero() {
        let mut s = ChemFlashState::default();
        s.spawn([10.0, 20.0], [0, 200, 255, 255], 5000.0);
        let i0 = s.flashes[0].intensity();
        s.tick(100);
        let i1 = if s.flashes.is_empty() { 0.0 } else { s.flashes[0].intensity() };
        assert!(i1 < i0);
    }

    #[test]
    fn parse_hex_round_trip() {
        assert_eq!(parse_flash_color_hex("FFCC00"), Some([0xFF, 0xCC, 0x00, 0xFF]));
        assert_eq!(parse_flash_color_hex("#00CCFF"), Some([0x00, 0xCC, 0xFF, 0xFF]));
        assert_eq!(parse_flash_color_hex("zzzzzz"), None);
        assert_eq!(parse_flash_color_hex("FFF"), None);
    }

    #[test]
    fn high_energy_flash_lives_longer_than_low() {
        let mut s = ChemFlashState::default();
        s.spawn([0.0, 0.0], [255, 255, 255, 255], 100_000.0);
        s.spawn([0.0, 0.0], [255, 255, 255, 255], 5_000_000.0);
        let lo = s.flashes[0].lifetime_ms;
        let hi = s.flashes[1].lifetime_ms;
        assert!(hi > lo);
    }
}
