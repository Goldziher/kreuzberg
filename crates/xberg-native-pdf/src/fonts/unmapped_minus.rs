use super::FontInfo;
use skrifa::outline::OutlinePen;
use skrifa::raw::TableProvider;
use skrifa::raw::ps::cff::CffFontRef;
use skrifa::{FontRef, GlyphId};

// ~keep This extra recovery pass targets small subset fonts; larger fonts keep the normal decode cascade.
const MAX_FONT_BYTES: usize = 1024 * 1024;
const SYMBOLIC_FLAG: i32 = 1 << 2;
const NONSYMBOLIC_FLAG: i32 = 1 << 5;
const MIN_WIDTH_EM: f32 = 0.3;
const MAX_WIDTH_EM: f32 = 0.9;
const MIN_THICKNESS_EM: f32 = 0.015;
const MAX_THICKNESS_EM: f32 = 0.09;
const MIN_AXIS_EM: f32 = 0.15;
const MAX_AXIS_EM: f32 = 0.4;
const MIN_ASPECT_RATIO: f32 = 6.0;

pub(super) fn recover(info: &FontInfo, code: u32) -> Option<char> {
    if info.subtype != "Type1"
        || !matches!(code, 0..=8 | 11..=12 | 14..=31)
        || !info
            .flags
            .is_some_and(|flags| flags & (SYMBOLIC_FLAG | NONSYMBOLIC_FLAG) == SYMBOLIC_FLAG)
    {
        return None;
    }
    let gid = *info.cff_gid_map.as_ref()?.get(&(code as u8))?;
    let data = info.embedded_font_data.as_ref()?;
    if gid == 0 || data.len() > MAX_FONT_BYTES {
        return None;
    }
    let font = FontRef::new(data).ok()?;
    let units_per_em = font.head().ok()?.units_per_em();
    let cff = font.cff().ok()?;
    let outline = CffFontRef::new_cff(cff.offset_data().as_bytes(), 0, Some(i32::from(units_per_em))).ok()?;
    let glyph = GlyphId::from(gid);
    let subfont = outline.subfont(outline.subfont_index(glyph)?, &[]).ok()?;
    let mut pen = RectanglePen::default();
    outline.draw(&subfont, glyph, &[], None, &mut pen).ok()?;
    // ~keep An otherwise dropped control-code glyph has no Unicode semantics. Recover only
    // a single wide, thin bar at the math axis; preserve all explicit mappings upstream.
    pen.is_math_bar(units_per_em).then_some('−')
}

struct RectanglePen {
    points: [(f32, f32); 5],
    length: usize,
    closed: bool,
    valid: bool,
}

impl Default for RectanglePen {
    fn default() -> Self {
        Self {
            points: [(0.0, 0.0); 5],
            length: 0,
            closed: false,
            valid: true,
        }
    }
}

impl RectanglePen {
    fn push(&mut self, x: f32, y: f32) {
        if self.closed || self.length == self.points.len() || !x.is_finite() || !y.is_finite() {
            self.valid = false;
            return;
        }
        self.points[self.length] = (x, y);
        self.length += 1;
    }

    fn is_math_bar(&self, units_per_em: u16) -> bool {
        if !self.valid || !self.closed || units_per_em == 0 {
            return false;
        }
        let count = if self.length == 5 && self.points[0] == self.points[4] {
            4
        } else {
            self.length
        };
        if count != 4 || !self.is_rectangle() {
            return false;
        }
        let min_x = self.points[..4].iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = self.points[..4].iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = self.points[..4].iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = self.points[..4].iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        let em = f32::from(units_per_em);
        let width = (max_x - min_x) / em;
        let thickness = (max_y - min_y) / em;
        let axis = (min_y + max_y) / (2.0 * em);
        (MIN_WIDTH_EM..=MAX_WIDTH_EM).contains(&width)
            && (MIN_THICKNESS_EM..=MAX_THICKNESS_EM).contains(&thickness)
            && (MIN_AXIS_EM..=MAX_AXIS_EM).contains(&axis)
            && width >= thickness * MIN_ASPECT_RATIO
            && min_x >= 0.0
            && max_x <= em
    }

    fn is_rectangle(&self) -> bool {
        let points = &self.points[..4];
        for index in 0..4 {
            let current = points[index];
            let next = points[(index + 1) % 4];
            if (current.0 == next.0) == (current.1 == next.1) {
                return false;
            }
            if points[..index].contains(&current) {
                return false;
            }
        }
        true
    }
}

impl OutlinePen for RectanglePen {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.length != 0 {
            self.valid = false;
        }
        self.push(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if self.length == 0 {
            self.valid = false;
        }
        self.push(x, y);
    }

    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        self.valid = false;
    }

    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        self.valid = false;
    }

    fn close(&mut self) {
        if self.closed || self.length < 4 {
            self.valid = false;
        }
        self.closed = true;
    }
}
