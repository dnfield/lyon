use math::*;
use path::{ Verb };

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgEvent {
    MoveTo(Point),
    RelativeMoveTo(Vec2),
    LineTo(Point),
    RelativeLineTo(Vec2),
    QuadraticTo(Point, Point),
    RelativeQuadraticTo(Vec2, Vec2),
    CubicTo(Point, Point, Point),
    RelativeCubicTo(Vec2, Vec2, Vec2),
    ArcTo(Vec2, Vec2, Vec2),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    RelativeHorizontalLineTo(f32),
    RelativeVerticalLineTo(f32),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PrimitiveEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AdaptedEvent {
    Begin(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    End(bool), // close
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    LineTo(Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Segment {
    Line(Point, Point),
    QuadraticBezier(Point, Point, Point),
    CubicBezier(Point, Point, Point, Point),
}

pub trait PathIterator<EventType> : Iterator<Item=EventType> {
  fn current_position(&self) -> Point;
  fn first_position(&self) -> Point;
}

impl SvgEvent {
    pub fn to_primitive(self, current: Point) -> PrimitiveEvent {
        return match self {
            SvgEvent::MoveTo(to) => { PrimitiveEvent::MoveTo(to) }
            SvgEvent::LineTo(to) => { PrimitiveEvent::LineTo(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(ctrl, to) }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) }
            SvgEvent::Close => { PrimitiveEvent::Close }
            SvgEvent::RelativeMoveTo(to) => { PrimitiveEvent::MoveTo(current + to) }
            SvgEvent::RelativeLineTo(to) => { PrimitiveEvent::LineTo(current + to) }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(current + ctrl, current + to) }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(current + ctrl1, current + ctrl2, to) }
            SvgEvent::HorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(x, current.y)) }
            SvgEvent::VerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, y)) }
            SvgEvent::RelativeHorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(current.x + x, current.y)) }
            SvgEvent::RelativeVerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, current.y + y)) }
            // TODO arcs and smooth events
            _ => { unimplemented!() }
        };
    }
}

impl PrimitiveEvent {
    pub fn to_svg(self) -> SvgEvent {
        return match self {
            PrimitiveEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            PrimitiveEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            PrimitiveEvent::QuadraticTo(ctrl, to) => { SvgEvent::QuadraticTo(ctrl, to) }
            PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) => { SvgEvent::CubicTo(ctrl1, ctrl2, to) }
            PrimitiveEvent::Close => { SvgEvent::Close }
        };
    }
}

#[derive(Clone, Debug)]
pub struct PathIter<'l> {
    vertices: ::std::slice::Iter<'l, Point>,
    verbs: ::std::slice::Iter<'l, Verb>,
    current: Point,
    first: Point,
}

impl<'l> PathIter<'l> {
    pub fn new(vertices: &'l[Point], verbs: &'l[Verb]) -> Self {
        PathIter {
            vertices: vertices.iter(),
            verbs: verbs.iter(),
            current: Point::new(0.0, 0.0),
            first: Point::new(0.0, 0.0),
        }
    }
}

impl<'l> Iterator for PathIter<'l> {
    type Item = PrimitiveEvent;
    fn next(&mut self) -> Option<PrimitiveEvent> {
        return match self.verbs.next() {
            Some(&Verb::MoveTo) => {
                let to = *self.vertices.next().unwrap();
                self.current = to;
                self.first = to;
                Some(PrimitiveEvent::MoveTo(to))
            }
            Some(&Verb::LineTo) => {
                let to = *self.vertices.next().unwrap();
                self.current = to;
                Some(PrimitiveEvent::LineTo(to))
            }
            Some(&Verb::QuadraticTo) => {
                let ctrl = *self.vertices.next().unwrap();
                let to = *self.vertices.next().unwrap();
                self.current = to;
                Some(PrimitiveEvent::QuadraticTo(ctrl, to))
            }
            Some(&Verb::CubicTo) => {
                let ctrl1 = *self.vertices.next().unwrap();
                let ctrl2 = *self.vertices.next().unwrap();
                let to = *self.vertices.next().unwrap();
                self.current = to;
                Some(PrimitiveEvent::CubicTo(ctrl1, ctrl2, to))
            }
            Some(&Verb::Close) => {
                self.current = self.first;
                Some(PrimitiveEvent::Close)
            }
            None => { None }
        };
    }
}

impl<'l> PathIterator<PrimitiveEvent> for PathIter<'l> {
    fn current_position(&self) -> Point { self.current }
    fn first_position(&self) -> Point { self.first }
}

// Consumes an iterator of path events and yields segments.
pub struct SegmentIterator<PathIt> {
    it: PathIt,
    current_position: Point,
    first_position: Point,
    in_sub_path: bool,
}

impl<'l, PathIt:'l+Iterator<Item=PrimitiveEvent>> SegmentIterator<PathIt> {
    pub fn new(it: PathIt) -> Self {
        SegmentIterator {
            it: it,
            current_position: point(0.0, 0.0),
            first_position: point(0.0, 0.0),
            in_sub_path: false,
        }
    }

    fn close(&mut self) -> Option<Segment> {
        let first = self.first_position;
        self.first_position = self.current_position;
        self.in_sub_path = false;
        if first != self.current_position {
            Some(Segment::Line(first, self.current_position))
        } else {
            self.next()
        }
    }
}

impl<'l, PathIt:'l+Iterator<Item=PrimitiveEvent>> Iterator
for SegmentIterator<PathIt> {
    type Item = Segment;
    fn next(&mut self) -> Option<Segment> {
        return match self.it.next() {
            Some(PrimitiveEvent::MoveTo(to)) => {
                let first = self.first_position;
                self.first_position = to;
                if self.in_sub_path && first != self.current_position {
                    Some(Segment::Line(first, self.current_position))
                } else {
                    self.in_sub_path = true;
                    self.next()
                }
            }
            Some(PrimitiveEvent::LineTo(to)) => {
                self.in_sub_path = true;
                let from = self.current_position;
                self.current_position = to;
                Some(Segment::Line(from, to))
            }
            Some(PrimitiveEvent::QuadraticTo(ctrl, to)) => {
                self.in_sub_path = true;
                let from = self.current_position;
                self.current_position = to;
                Some(Segment::QuadraticBezier(from, ctrl, to))
            }
            Some(PrimitiveEvent::CubicTo(ctrl1, ctrl2, to)) => {
                self.in_sub_path = true;
                let from = self.current_position;
                self.current_position = to;
                Some(Segment::CubicBezier(from, ctrl1, ctrl2, to))
            }
            Some(PrimitiveEvent::Close) => { self.close() }
            None => { None }
        };
    }
}

pub struct SvgToPrimitiveIter<SvgIter> {
    it: SvgIter,
}

impl<SvgIter> SvgToPrimitiveIter<SvgIter> {
  pub fn new(it: SvgIter) -> Self { SvgToPrimitiveIter { it: it } }
}

impl<SvgIter> PathIterator<PrimitiveEvent> for SvgToPrimitiveIter<SvgIter>
where SvgIter : PathIterator<SvgEvent> {
  fn current_position(&self) -> Point { self.it.current_position() }
  fn first_position(&self) -> Point { self.it.first_position() }
}

impl<SvgIter> Iterator for SvgToPrimitiveIter<SvgIter>
where SvgIter: PathIterator<SvgEvent> {
    type Item = PrimitiveEvent;
    fn next(&mut self) -> Option<PrimitiveEvent> {
        return match self.it.next() {
            Some(svg_evt) => { Some(svg_evt.to_primitive(self.current_position())) }
            None => { None }
        }
    }
}

pub struct PrimitiveToSvgIter<PrimitiveIter> {
    it: PrimitiveIter,
}

impl<PrimitiveIter> PrimitiveToSvgIter<PrimitiveIter> {
  pub fn new(it: PrimitiveIter) -> Self { PrimitiveToSvgIter { it: it } }
}

impl<PrimitiveIter> PathIterator<SvgEvent> for PrimitiveToSvgIter<PrimitiveIter>
where PrimitiveIter : PathIterator<PrimitiveEvent> {
  fn current_position(&self) -> Point { self.it.current_position() }
  fn first_position(&self) -> Point { self.it.first_position() }
}

impl<PrimitiveIter> Iterator for PrimitiveToSvgIter<PrimitiveIter>
where PrimitiveIter: Iterator<Item=PrimitiveEvent> {
    type Item = SvgEvent;
    fn next(&mut self) -> Option<SvgEvent> {
        return match self.it.next() {
            Some(primitive_evt) => { Some(primitive_evt.to_svg()) }
            None => { None }
        }
    }
}

use bezier::{ QuadraticFlattenIter, QuadraticBezierSegment };

enum TmpFlattenIter {
  Quadratic(QuadraticFlattenIter),
  None,
}

pub struct FlattenIter<Iter> {
  it: Iter,
  current_curve: TmpFlattenIter,
  tolerance: f32,
}

impl<Iter> FlattenIter<Iter> {
    pub fn new(tolerance: f32, it: Iter) -> Self {
        FlattenIter {
            it: it,
            current_curve: TmpFlattenIter::None,
            tolerance: tolerance,
        }
    }
}

impl<Iter> PathIterator<FlattenedEvent> for FlattenIter<Iter>
where Iter : PathIterator<PrimitiveEvent> {
  fn current_position(&self) -> Point { self.it.current_position() }
  fn first_position(&self) -> Point { self.it.first_position() }
}

impl<Iter> Iterator for FlattenIter<Iter>
where Iter: PathIterator<PrimitiveEvent> {
    type Item = FlattenedEvent;
    fn next(&mut self) -> Option<FlattenedEvent> {
        match self.current_curve {
            TmpFlattenIter::Quadratic(ref mut it) => {
                if let Some(point) = it.next() {
                  return Some(FlattenedEvent::LineTo(point));
                }
            }
            _ => {}
        }
        self.current_curve = TmpFlattenIter::None;
        return match self.it.next() {
            Some(PrimitiveEvent::MoveTo(to)) => { Some(FlattenedEvent::MoveTo(to)) }
            Some(PrimitiveEvent::LineTo(to)) => { Some(FlattenedEvent::LineTo(to)) }
            Some(PrimitiveEvent::Close) => { Some(FlattenedEvent::Close) }
            Some(PrimitiveEvent::QuadraticTo(ctrl, to)) => {
                let current = self.current_position();
                self.current_curve = TmpFlattenIter::Quadratic(
                    QuadraticBezierSegment {
                      from: current, cp: ctrl, to: to
                    }.flatten_iter(self.tolerance)
                );
                return self.next();
            }
            None => { None }
            unknown => {
                println!(" -- Unimplemented event: {:?}", unknown);
                unimplemented!();
            }
        }
    }
}

//impl<T: PathIterator<PrimitiveEvent>> T {
//  pub fn to_svg(self) -> PrimitiveToSvgIter<Sef> { PrimitiveToSvgIter::new(self) }
//}
//
//impl<T: PathIterator<SvgEvent>> T {
//  pub fn to_primitive(self) -> SvgToPrimitiveIter<Sef> { SvgToPrimitiveIter::new(self) }
//}

/*
pub struct VertexEvent {
  current: Point,
  previous: Point,
  next: Point,
}

struct VertexEventIter<Iter> {
  it: Iter,
  current: Point,
  previous: Point,
  first: Point,
  second: Point,
  done: bool,
}

impl<Iter: Iterator<Item=FlattenedEvent>> Iterator for VertexEventIter {
    pub fn new(mut it: Iter) -> Self { VertexEventIter::init(it.next(), it.next()) }

    fn init(first: Option<FlattenedEvent>, second: Option<FlattenedEvent>) -> Self {
        return match (first, second) {
            (Some(first), Some(second)) => {
                VertexEventIter {
                    it: it,
                    current = second,
                    previous = first,
                    first = first,
                    second = second,
                    done: false,
            }
            _ => {
                VertexEventIter {
                    it: it,
                    current = Point::new(0.0, 0.0),
                    previous = Point::new(0.0, 0.0),
                    first = Point::new(0.0, 0.0),
                    second = Point::new(0.0, 0.0),
                    done: true,
                }
            }
        }
    }
}

impl<Iter: Iterator<Item=FlattenedEvent>> Iterator for VertexEventIter {
    type Item = VertexEvent;
    fn next(&mut self) -> Option<VertexEvent> {
        if self.done {
          return None;
        }
        match it.next() {
            Some(FlattenedEvent::LineTo(next)) => {
                let evt = Some(VertexEvent {
                  current: self.current,
                  previous: self.previous,
                  next: next,            
                });
                self.current = next;
                self.previous = self.current;
                return evt;
            }
            Some(FlattenedEvent::MoveTo(next)) => {
                *self = VertexEventIter::init(Some(FlattenedEvent::MoveTo(next)), it.next());
                return self.next();
            }
            Some(FlattenedEvent::Close) => {
                let evt = Some(VertexEvent {
                  current: self.current,
                  previous: self.previous,
                  next: self.first,            
                });
                *self = VertexEventIter::init(Some(FlattenedEvent::MoveTo(next)), it.next());
                return evt;
            }
        }
        if let Some(FlattenedEvent::Close(next)) = self.it.next() {
          let evt = Some(VertexEvent {
            current: self.current,
            previous: self.previous,
            next: next,            
          });
          self.current = next;
          self.previous = self.current;
          return evt;
        }
        self.done = true;
        return Some(VertexEvent{
          current: self.first,
          next: self.second,
          previous: self.previous,
        })
    }
}
*/