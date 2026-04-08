/// A rectangle in any coordinate space, defined by its bottom-left corner,
/// width, and height. After transform_rect, it is always in page space.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f64,      // left edge (minimum X)
    pub y: f64,      // bottom edge (minimum Y, PDF Y axis goes up/points upwards)
    pub width: f64,  // horizontal size
    pub height: f64, // vertical size
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn expand(&self, bleed: f64) -> Self {
        Rect {
            x: self.x - bleed,
            y: self.y - bleed,
            width: self.width + 2.0 * bleed,
            height: self.height + 2.0 * bleed,
        }
    }

    pub fn to_pdf_array(&self) -> [f64; 4] {
        [self.x, self.y, self.right(), self.top()]
    }

    /// Build from the PDF [x0, y0, x1, y1] box format (corner representation / two corner points)
    pub fn from_corners(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Rect {
            x: x0.min(x1),
            y: y0.min(y1),
            width: (x1 - x0).abs(),
            height: (y1 - y0).abs(),
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    pub fn top(&self) -> f64 {
        self.y + self.height
    }
}
