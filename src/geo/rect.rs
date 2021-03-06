use crate::Point;

/// Rect struct to make it more convenient to
/// construct sprite instances
/// Assumes a2d coordinates (i.e. origin at upper-left
/// corner)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    upper_left: [f32; 2],
    lower_right: [f32; 2],
}

impl Rect {
    /// Create a new Rect
    /// returns None if the rectangle would be degenerate
    /// or close to generate
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Option<Rect> {
        if appx_eq(x1, x2) || appx_eq(y1, y2) {
            None
        } else {
            let upper_left = [min(x1, x2), min(y1, y2)];
            let lower_right = [max(x1, x2), max(y1, y2)];
            Some(Self {
                upper_left,
                lower_right,
            })
        }
    }

    pub const fn upper_left(&self) -> [f32; 2] {
        self.upper_left
    }

    pub const fn lower_right(&self) -> [f32; 2] {
        self.lower_right
    }
}

impl From<[f32; 4]> for Rect {
    fn from(arr: [f32; 4]) -> Rect {
        match Rect::new(arr[0], arr[1], arr[2], arr[3]) {
            Some(r) => r,
            None => panic!("Tried to construct degenerate a2d Rect"),
        }
    }
}

impl From<[[f32; 2]; 2]> for Rect {
    fn from(arr: [[f32; 2]; 2]) -> Rect {
        [arr[0][0], arr[0][1], arr[1][0], arr[1][1]].into()
    }
}

impl From<[Point; 2]> for Rect {
    fn from(points: [Point; 2]) -> Self {
        [points[0].to_array(), points[1].to_array()].into()
    }
}

// TODO: audit this
fn appx_eq(a: f32, b: f32) -> bool {
    // basically from
    // https://stackoverflow.com/questions/4915462/
    // how-should-i-do-floating-point-comparison
    a == b || {
        const EPSILON: f32 = f32::EPSILON * 128.0;
        const RELTH: f32 = EPSILON;
        let diff = (a - b).abs();
        let norm = min(a.abs() + b.abs(), f32::MAX);
        diff < max(RELTH, EPSILON * norm)
    }
}

fn min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}
