use crate::rect::Rect;

/// A 2D affine transformation matrix storing the 6 PDF values [a, b, c, d, e, f].
///
/// The transformation from local space to page space is:
///     x' = a*x + c*y + e
///     y' = b*x + d*y + f
///
/// The identity matrix (no transformation) is: a=1, b=0, c=0, d=1, e=0, f=0
#[derive(Clone, Copy, Debug)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Matrix {
    /// The identity matrix – applying it changes nothing
    /// a is 1, b is 0, c is 0, d is 1, e is 0, f is 0
    /// This means:
    /// x' = 1*x + 0*y + 0 = x
    /// y' = 0*x + 1*y + 0 = y
    /// So points remain unchanged.
    /// the reason a is 1 and d is 1 is because they are scaling factors for x and y respectively.
    /// If they were 0, all x or y values would collapse to 0.
    /// since the identity matrix does not scale, rotate, shear, or translate,
    /// it must have scaling factors of 1 for both axes.
    pub fn identity() -> Self {
        Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Construct from a 6-element slice in PDF order [a, b, c, d, e, f]
    pub fn from_values(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Matrix { a, b, c, d, e, f }
    }

    /// Concatenate (multiply) this matrix with another matrix
    /// In PDF, `cm` operator applies the new matrix after the current one
    /// so this means: new_ctm = old_ctm * cm_matrix
    /// This is how nested transformations work in PDF graphics state and how they compound.
    pub fn concat(&self, other: &Matrix) -> Matrix {
        // Matrix multiplication for 2D affine transformations
        Matrix {
            // Refers back to our transformation equations above
            // x' = a*x + c*y + e
            // y' = b*x + d*y + f
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }
    /// Transform a single point from local space to page space
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        // Apply the matrix transformation to a point (x, y)
        (
            // Again, returns the transformed coordinates based on the matrix equations
            // x' = a*x + c*y + e
            // y' = b*x + d*y + f
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
    /// Transform a rectangle into page space, returning an axis-aligned bounding box.
    /// We transform all four corners and take the min/max because rotation and negative scaling can reorder the corners.
    /// NOTE: we get an axis-aligned bounding box, because rotation/shear means the 4 corners don't stay axis-aligned
    pub fn transform_rect(&self, rect: &Rect) -> Rect {
        // Transform all four corners of the rectangle and return the bounding box
        let corners = [
            self.transform_point(rect.x, rect.y),
            self.transform_point(rect.x + rect.width, rect.y),
            self.transform_point(rect.x, rect.y + rect.height),
            self.transform_point(rect.x + rect.width, rect.y + rect.height),
        ];

        let min_x = corners
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::INFINITY, f64::min);
        let max_x = corners
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = corners
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_y = corners
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}
