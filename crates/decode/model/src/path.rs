//! A path a caller draws, as Figma stores a vector.
//!
//! A vector node carries a *vector network* (`vectorData.vectorNetworkBlob`),
//! present on all 11,745 vectors in the fixtures, and usually geometry derived
//! from it (`fillGeometry`, `strokeGeometry`: command blobs, see
//! [`crate::vector`]). The network's layout, decoded exactly on every one of
//! those vectors:
//!
//! - a header of three `u32`: vertex, segment and region counts;
//! - each vertex: `styleID u32, x f32, y f32`;
//! - each segment: `styleID u32, start u32, start tangent (f32, f32), end u32,
//!   end tangent (f32, f32)`. A tangent is the cubic's control point relative
//!   to its end; a straight segment has zero tangents;
//! - each region: `styleID << 1 | winding u32` (1 is `NONZERO`, 0 `ODD`, on all
//!   3,041 regions whose vector has a fill rule to compare), a loop count, and
//!   each loop as a segment count and the segments' indices.
//!
//! Everything is little-endian. A path comes from SVG path data, the form icon
//! sets publish (Lucide, Heroicons…), and is reduced to moves, lines and cubics.

use std::fmt;

/// A point in the path's own coordinates.
pub type Point = (f32, f32);

/// One piece of a subpath, ending at its last point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Line(Point),
    Cubic(Point, Point, Point),
}

impl Segment {
    fn end(&self) -> Point {
        match *self {
            Segment::Line(p) | Segment::Cubic(_, _, p) => p,
        }
    }
}

/// A run of segments from one move, closed back to its start or open.
#[derive(Debug, Clone, PartialEq)]
pub struct Subpath {
    pub start: Point,
    pub segments: Vec<Segment>,
    pub closed: bool,
}

/// A drawn path: its subpaths, in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    pub subpaths: Vec<Subpath>,
}

/// Why path data could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError(pub String);

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PathError {}

impl Path {
    /// Read SVG path data: `M L H V C S Q T A Z`, absolute and relative.
    pub fn parse(d: &str) -> Result<Self, PathError> {
        Parser::new(d).run()
    }

    /// The tight bounds of the path: `(min_x, min_y, max_x, max_y)`, taking
    /// each cubic's extremes, not its control points.
    #[must_use]
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        let mut add = |(x, y): Point| {
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            });
        };
        for subpath in &self.subpaths {
            add(subpath.start);
            let mut from = subpath.start;
            for segment in &subpath.segments {
                if let Segment::Cubic(c1, c2, to) = *segment {
                    for t in cubic_extremes(from, c1, c2, to) {
                        add(cubic_at(from, c1, c2, to, t));
                    }
                }
                add(segment.end());
                from = segment.end();
            }
        }
        bounds
    }

    /// Every point mapped through `f`.
    #[must_use]
    pub fn map(&self, f: impl Fn(Point) -> Point) -> Self {
        Self {
            subpaths: self
                .subpaths
                .iter()
                .map(|s| Subpath {
                    start: f(s.start),
                    closed: s.closed,
                    segments: s
                        .segments
                        .iter()
                        .map(|segment| match *segment {
                            Segment::Line(p) => Segment::Line(f(p)),
                            Segment::Cubic(a, b, p) => Segment::Cubic(f(a), f(b), f(p)),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// The path as a geometry command blob, as `fillGeometry` stores one.
    #[must_use]
    pub fn commands_blob(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut put = |op: u8, points: &[Point]| {
            out.push(op);
            for (x, y) in points {
                out.extend_from_slice(&x.to_le_bytes());
                out.extend_from_slice(&y.to_le_bytes());
            }
        };
        for subpath in &self.subpaths {
            put(1, &[subpath.start]);
            for segment in &subpath.segments {
                match *segment {
                    Segment::Line(p) => put(2, &[p]),
                    Segment::Cubic(a, b, p) => put(4, &[a, b, p]),
                }
            }
            if subpath.closed {
                put(0, &[]);
            }
        }
        out
    }

    /// The path as a vector network blob. Each closed subpath is a loop of one
    /// region, filled by `winding` (`true` for `NONZERO`); open subpaths are
    /// segments only, which is how a stroke-only line is stored.
    #[must_use]
    pub fn network_blob(&self, nonzero: bool) -> Vec<u8> {
        let mut vertices: Vec<Point> = Vec::new();
        // (start vertex, start tangent, end vertex, end tangent)
        let mut segments: Vec<(u32, Point, u32, Point)> = Vec::new();
        let mut loops: Vec<Vec<u32>> = Vec::new();
        for subpath in &self.subpaths {
            let first = vertices.len() as u32;
            vertices.push(subpath.start);
            let mut from_index = first;
            let mut from = subpath.start;
            let mut indices = Vec::new();
            let count = subpath.segments.len();
            // A subpath that ends where it began is closed, `Z` or not: a
            // circle written as two arcs back to its start, as icon sets do.
            let closed = subpath.closed || subpath.segments.last().is_some_and(|s| close_to(s.end(), subpath.start));
            for (i, segment) in subpath.segments.iter().enumerate() {
                let to = segment.end();
                // A closed subpath's last segment ends where it began.
                let returns = closed && i + 1 == count && close_to(to, subpath.start);
                let to_index = if returns {
                    first
                } else {
                    vertices.push(to);
                    vertices.len() as u32 - 1
                };
                let (t0, t1) = match *segment {
                    Segment::Line(_) => ((0.0, 0.0), (0.0, 0.0)),
                    Segment::Cubic(c1, c2, _) => ((c1.0 - from.0, c1.1 - from.1), (c2.0 - to.0, c2.1 - to.1)),
                };
                indices.push(segments.len() as u32);
                segments.push((from_index, t0, to_index, t1));
                from_index = to_index;
                from = to;
            }
            if closed {
                // A close that the data did not draw as a segment is a line back.
                if from_index != first {
                    indices.push(segments.len() as u32);
                    segments.push((from_index, (0.0, 0.0), first, (0.0, 0.0)));
                }
                loops.push(indices);
            }
        }

        let mut out = Vec::new();
        let u = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
        let f = |out: &mut Vec<u8>, v: f32| out.extend_from_slice(&v.to_le_bytes());
        u(&mut out, vertices.len() as u32);
        u(&mut out, segments.len() as u32);
        u(&mut out, u32::from(!loops.is_empty()));
        for (x, y) in &vertices {
            u(&mut out, 0);
            f(&mut out, *x);
            f(&mut out, *y);
        }
        for (start, t0, end, t1) in &segments {
            u(&mut out, 0);
            u(&mut out, *start);
            f(&mut out, t0.0);
            f(&mut out, t0.1);
            u(&mut out, *end);
            f(&mut out, t1.0);
            f(&mut out, t1.1);
        }
        if !loops.is_empty() {
            u(&mut out, u32::from(nonzero));
            u(&mut out, loops.len() as u32);
            for indices in &loops {
                u(&mut out, indices.len() as u32);
                for index in indices {
                    u(&mut out, *index);
                }
            }
        }
        out
    }
}

/// A vector network, as Figma stores it (see the module notes).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Network {
    pub vertices: Vec<Point>,
    /// `(start vertex, start tangent, end vertex, end tangent)`.
    pub segments: Vec<(u32, Point, u32, Point)>,
    /// Each region's fill rule (`true` for `NONZERO`) and loops of segments.
    pub regions: Vec<(bool, Vec<Vec<u32>>)>,
}

impl Network {
    /// Read a network blob. `None` when it does not decode exactly, to the
    /// last byte, or names a vertex or segment it does not have.
    #[must_use]
    pub fn decode(blob: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let mut read = || -> Option<u32> {
            let word = u32::from_le_bytes(blob.get(at..at + 4)?.try_into().ok()?);
            at += 4;
            Some(word)
        };
        let vertex_count = read()?;
        let segment_count = read()?;
        let region_count = read()?;
        // Counts beyond what the blob can hold are not a network.
        if u64::from(vertex_count) * 12 + u64::from(segment_count) * 28 > blob.len() as u64 {
            return None;
        }
        let mut network = Network::default();
        let as_f = f32::from_bits;
        for _ in 0..vertex_count {
            let _style = read()?;
            network.vertices.push((as_f(read()?), as_f(read()?)));
        }
        for _ in 0..segment_count {
            let _style = read()?;
            let start = read()?;
            let t0 = (as_f(read()?), as_f(read()?));
            let end = read()?;
            let t1 = (as_f(read()?), as_f(read()?));
            if start >= vertex_count || end >= vertex_count {
                return None;
            }
            network.segments.push((start, t0, end, t1));
        }
        for _ in 0..region_count {
            let packed = read()?;
            let loop_count = read()?;
            let mut loops = Vec::new();
            for _ in 0..loop_count {
                let count = read()?;
                if u64::from(count) * 4 > blob.len() as u64 {
                    return None;
                }
                let indices = (0..count).map(|_| read()).collect::<Option<Vec<u32>>>()?;
                if indices.iter().any(|i| *i >= segment_count) {
                    return None;
                }
                loops.push(indices);
            }
            network.regions.push((packed & 1 == 1, loops));
        }
        (at == blob.len()).then_some(network)
    }

    /// SVG path data for one segment's curve, from its start.
    fn curve(&self, (start, t0, end, t1): (u32, Point, u32, Point), reversed: bool, scale: Point) -> (Point, String) {
        let (mut a, mut b) = (self.vertices[start as usize], self.vertices[end as usize]);
        let (mut ta, mut tb) = (t0, t1);
        if reversed {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut ta, &mut tb);
        }
        let p = |(x, y): Point| (x * scale.0, y * scale.1);
        let (a2, b2) = (p(a), p(b));
        let d = if ta == (0.0, 0.0) && tb == (0.0, 0.0) {
            format!("L {} {}", b2.0, b2.1)
        } else {
            let (c1, c2) = (p((a.0 + ta.0, a.1 + ta.1)), p((b.0 + tb.0, b.1 + tb.1)));
            format!("C {} {} {} {} {} {}", c1.0, c1.1, c2.0, c2.1, b2.0, b2.1)
        };
        (a2, d)
    }

    /// SVG path data filling the network's regions, scaled by `scale`, and
    /// whether it fills by `NONZERO`. `None` without regions.
    #[must_use]
    pub fn fill_path(&self, scale: Point) -> Option<(String, bool)> {
        let mut d = String::new();
        for (_, loops) in &self.regions {
            for indices in loops {
                // Walk the loop, turning each segment to continue from the last.
                let mut at: Option<u32> = None;
                for (i, &index) in indices.iter().enumerate() {
                    let segment = self.segments[index as usize];
                    let reversed = match at {
                        Some(vertex) => segment.0 != vertex && segment.2 == vertex,
                        None => indices.get(1).is_some_and(|&next| {
                            let n = self.segments[next as usize];
                            segment.0 == n.0 || segment.0 == n.2
                        }),
                    };
                    let (from, curve) = self.curve(segment, reversed, scale);
                    if i == 0 {
                        d.push_str(&format!("M {} {} ", from.0, from.1));
                    }
                    d.push_str(&curve);
                    d.push(' ');
                    at = Some(if reversed { segment.0 } else { segment.2 });
                }
                d.push_str("Z ");
            }
        }
        let nonzero = self.regions.first().is_none_or(|(nonzero, _)| *nonzero);
        (!d.is_empty()).then(|| (d.trim_end().to_string(), nonzero))
    }

    /// SVG path data stroking every segment, chained where one ends where the
    /// next begins, scaled by `scale`.
    #[must_use]
    pub fn stroke_path(&self, scale: Point) -> String {
        let mut d = String::new();
        let mut at: Option<u32> = None;
        for &segment in &self.segments {
            let (from, curve) = self.curve(segment, false, scale);
            if at != Some(segment.0) {
                d.push_str(&format!("M {} {} ", from.0, from.1));
            }
            d.push_str(&curve);
            d.push(' ');
            at = Some(segment.2);
        }
        d.trim_end().to_string()
    }
}

fn close_to(a: Point, b: Point) -> bool {
    (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
}

fn cubic_at(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    let (a, b, c, d) = (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
    (a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1)
}

/// The parameters in (0, 1) where a cubic turns, on either axis.
fn cubic_extremes(p0: Point, p1: Point, p2: Point, p3: Point) -> Vec<f32> {
    let mut out = Vec::new();
    for (a0, a1, a2, a3) in [(p0.0, p1.0, p2.0, p3.0), (p0.1, p1.1, p2.1, p3.1)] {
        // The derivative's coefficients: a t² + b t + c.
        let a = 3.0 * (-a0 + 3.0 * a1 - 3.0 * a2 + a3);
        let b = 6.0 * (a0 - 2.0 * a1 + a2);
        let c = 3.0 * (a1 - a0);
        if a.abs() < 1e-9 {
            if b.abs() > 1e-9 {
                out.push(-c / b);
            }
            continue;
        }
        let discriminant = b * b - 4.0 * a * c;
        if discriminant >= 0.0 {
            let root = discriminant.sqrt();
            out.push((-b + root) / (2.0 * a));
            out.push((-b - root) / (2.0 * a));
        }
    }
    out.retain(|t| *t > 0.0 && *t < 1.0);
    out
}

/// SVG path data, read command by command.
struct Parser<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    text: &'a str,
    path: Path,
    current: Point,
    start: Point,
    /// The previous segment's second control point, for `S` and `T`.
    last_cubic: Option<Point>,
    last_quad: Option<Point>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            chars: text.char_indices().peekable(),
            text,
            path: Path::default(),
            current: (0.0, 0.0),
            start: (0.0, 0.0),
            last_cubic: None,
            last_quad: None,
        }
    }

    fn skip_separators(&mut self) {
        while self.chars.peek().is_some_and(|(_, c)| c.is_whitespace() || *c == ',') {
            self.chars.next();
        }
    }

    fn has_number(&mut self) -> bool {
        self.skip_separators();
        self.chars.peek().is_some_and(|(_, c)| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))
    }

    fn number(&mut self) -> Result<f32, PathError> {
        self.skip_separators();
        let Some(&(begin, _)) = self.chars.peek() else {
            return Err(PathError("the path data ends where a number was expected".into()));
        };
        let mut end = begin;
        let (mut seen_dot, mut seen_exponent, mut first) = (false, false, true);
        while let Some(&(i, c)) = self.chars.peek() {
            let sign_ok = first || self.text[..i].ends_with(['e', 'E']);
            let ok = c.is_ascii_digit()
                || (matches!(c, '-' | '+') && sign_ok)
                || (c == '.' && !seen_dot && !seen_exponent)
                || (matches!(c, 'e' | 'E') && !seen_exponent && !first);
            if !ok {
                break;
            }
            seen_dot |= c == '.';
            seen_exponent |= matches!(c, 'e' | 'E');
            first = false;
            end = i + c.len_utf8();
            self.chars.next();
        }
        self.text[begin..end]
            .parse()
            .map_err(|_| PathError(format!("`{}` at {begin} is not a number", &self.text[begin..end.max(begin + 1).min(self.text.len())])))
    }

    /// An arc flag, which SVG allows to be written without a separator.
    fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_separators();
        match self.chars.next() {
            Some((_, '0')) => Ok(false),
            Some((_, '1')) => Ok(true),
            other => Err(PathError(format!("an arc flag must be 0 or 1, got {other:?}"))),
        }
    }

    fn point(&mut self, relative: bool) -> Result<Point, PathError> {
        let (x, y) = (self.number()?, self.number()?);
        Ok(if relative { (self.current.0 + x, self.current.1 + y) } else { (x, y) })
    }

    fn subpath(&mut self) -> Result<&mut Subpath, PathError> {
        self.path
            .subpaths
            .last_mut()
            .filter(|s| !s.closed)
            .ok_or_else(|| PathError("a drawing command before any move (M)".into()))
    }

    fn push(&mut self, segment: Segment) -> Result<(), PathError> {
        if self.path.subpaths.is_empty() {
            return Err(PathError("a drawing command before any move (M)".into()));
        }
        if self.path.subpaths.last().is_some_and(|s| s.closed) {
            // Drawing after a close starts a new subpath where the last began.
            let start = self.start;
            self.path.subpaths.push(Subpath { start, segments: Vec::new(), closed: false });
        }
        self.subpath()?.segments.push(segment);
        self.current = segment.end();
        Ok(())
    }

    fn run(mut self) -> Result<Path, PathError> {
        let mut command: Option<char> = None;
        loop {
            self.skip_separators();
            let Some(&(i, c)) = self.chars.peek() else { break };
            let letter = if c.is_ascii_alphabetic() {
                self.chars.next();
                c
            } else {
                // A number repeats the last command; after a move it is a line.
                match command {
                    Some('M') => 'L',
                    Some('m') => 'l',
                    Some(previous) if !previous.eq_ignore_ascii_case(&'Z') => previous,
                    _ => return Err(PathError(format!("`{c}` at {i}: path data must start with a command"))),
                }
            };
            command = Some(letter);
            let relative = letter.is_ascii_lowercase();
            let (was_cubic, was_quad) = (self.last_cubic.take(), self.last_quad.take());
            match letter.to_ascii_uppercase() {
                'M' => {
                    let p = self.point(relative)?;
                    self.path.subpaths.push(Subpath { start: p, segments: Vec::new(), closed: false });
                    self.current = p;
                    self.start = p;
                }
                'L' => {
                    let p = self.point(relative)?;
                    self.push(Segment::Line(p))?;
                }
                'H' => {
                    let x = self.number()?;
                    let x = if relative { self.current.0 + x } else { x };
                    self.push(Segment::Line((x, self.current.1)))?;
                }
                'V' => {
                    let y = self.number()?;
                    let y = if relative { self.current.1 + y } else { y };
                    self.push(Segment::Line((self.current.0, y)))?;
                }
                'C' => {
                    let (c1, c2, p) = (self.point(relative)?, self.point(relative)?, self.point(relative)?);
                    self.push(Segment::Cubic(c1, c2, p))?;
                    self.last_cubic = Some(c2);
                }
                'S' => {
                    let c1 = reflect(was_cubic, self.current);
                    let (c2, p) = (self.point(relative)?, self.point(relative)?);
                    self.push(Segment::Cubic(c1, c2, p))?;
                    self.last_cubic = Some(c2);
                }
                'Q' => {
                    let (q, p) = (self.point(relative)?, self.point(relative)?);
                    let from = self.current;
                    self.push(quadratic(from, q, p))?;
                    self.last_quad = Some(q);
                }
                'T' => {
                    let q = reflect(was_quad, self.current);
                    let p = self.point(relative)?;
                    let from = self.current;
                    self.push(quadratic(from, q, p))?;
                    self.last_quad = Some(q);
                }
                'A' => {
                    let (rx, ry, rotation) = (self.number()?, self.number()?, self.number()?);
                    let (large, sweep) = (self.flag()?, self.flag()?);
                    let p = self.point(relative)?;
                    for segment in arc(self.current, (rx, ry), rotation, large, sweep, p) {
                        self.push(segment)?;
                    }
                }
                'Z' => {
                    if let Some(subpath) = self.path.subpaths.last_mut().filter(|s| !s.closed) {
                        subpath.closed = true;
                    }
                    self.current = self.start;
                }
                other => return Err(PathError(format!("`{other}` at {i} is not an SVG path command"))),
            }
            // Numbers after a close are a new command's, which SVG does not allow.
            if letter.eq_ignore_ascii_case(&'Z') && self.has_number() {
                return Err(PathError("a number after Z needs its own command".into()));
            }
        }
        self.path.subpaths.retain(|s| !s.segments.is_empty());
        if self.path.subpaths.is_empty() {
            return Err(PathError("the path data draws nothing".into()));
        }
        Ok(self.path)
    }
}

/// A control point mirrored through the current point, or the current point
/// itself when the previous segment was of another kind.
fn reflect(control: Option<Point>, current: Point) -> Point {
    control.map_or(current, |(x, y)| (2.0 * current.0 - x, 2.0 * current.1 - y))
}

/// A quadratic Bézier as the cubic that draws it exactly.
fn quadratic(from: Point, q: Point, to: Point) -> Segment {
    let c1 = (from.0 + 2.0 / 3.0 * (q.0 - from.0), from.1 + 2.0 / 3.0 * (q.1 - from.1));
    let c2 = (to.0 + 2.0 / 3.0 * (q.0 - to.0), to.1 + 2.0 / 3.0 * (q.1 - to.1));
    Segment::Cubic(c1, c2, to)
}

/// An elliptical arc as cubics, a quarter turn or less each (SVG 1.1, F.6).
fn arc(from: Point, radii: Point, rotation: f32, large: bool, sweep: bool, to: Point) -> Vec<Segment> {
    use std::f32::consts::PI;
    let (mut rx, mut ry) = (radii.0.abs(), radii.1.abs());
    if close_to(from, to) {
        return Vec::new();
    }
    if rx < 1e-6 || ry < 1e-6 {
        return vec![Segment::Line(to)];
    }
    let phi = rotation.to_radians();
    let (sin, cos) = phi.sin_cos();
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coefficient = (numerator / denominator).sqrt();
    if large == sweep {
        coefficient = -coefficient;
    }
    let (cx1, cy1) = (coefficient * rx * y1 / ry, -coefficient * ry * x1 / rx);
    let center = (cos * cx1 - sin * cy1 + (from.0 + to.0) / 2.0, sin * cx1 + cos * cy1 + (from.1 + to.1) / 2.0);
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let sign = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        let dot = (ux * vx + uy * vy) / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt());
        sign * dot.clamp(-1.0, 1.0).acos()
    };
    let theta = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }
    let pieces = (delta.abs() / (PI / 2.0)).ceil().max(1.0) as usize;
    let step = delta / pieces as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let on = |t: f32| {
        let (s, c) = t.sin_cos();
        (center.0 + rx * c * cos - ry * s * sin, center.1 + rx * c * sin + ry * s * cos)
    };
    let tangent = |t: f32| {
        let (s, c) = t.sin_cos();
        (-rx * s * cos - ry * c * sin, -rx * s * sin + ry * c * cos)
    };
    let mut out = Vec::with_capacity(pieces);
    for i in 0..pieces {
        let (a, b) = (theta + step * i as f32, theta + step * (i + 1) as f32);
        let (p0, p3) = (on(a), if i + 1 == pieces { to } else { on(b) });
        let (d0, d3) = (tangent(a), tangent(b));
        out.push(Segment::Cubic((p0.0 + k * d0.0, p0.1 + k * d0.1), (p3.0 - k * d3.0, p3.1 - k * d3.1), p3));
    }
    out
}

#[cfg(test)]
mod tests;
