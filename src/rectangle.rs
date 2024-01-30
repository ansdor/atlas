use std::cmp::{self, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AsRef<Rect> for Rect {
    fn as_ref(&self) -> &Rect { self }
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn cmp_by_distance<T: AsRef<Rect>>(a: T, b: T) -> Ordering {
        let (a, b) = (a.as_ref(), b.as_ref());
        let (da, db) = (a.distance_from_origin(), b.distance_from_origin());
        da.partial_cmp(&db).unwrap_or(Ordering::Equal)
    }

    pub fn cmp_by_area<T: AsRef<Rect>>(a: T, b: T) -> Ordering {
        let (a, b) = (a.as_ref(), b.as_ref());
        a.area().cmp(&b.area())
    }

    pub fn place_at(&mut self, x: u32, y: u32) {
        self.x = x;
        self.y = y;
    }

    pub fn rotate(&mut self) { (self.width, self.height) = (self.height, self.width); }

    pub fn slice_out(self, r: &Rect) -> Vec<Rect> {
        let mut pieces = Vec::<Rect>::new();
        if self.intersection(r).area() > 0 {
            let relative_position = (r.x.saturating_sub(self.x), r.y.saturating_sub(self.y));
            //top
            pieces.push(Rect {
                x: self.x,
                y: self.y,
                width: self.width,
                height: relative_position.1,
            });
            //bottom
            pieces.push(Rect {
                x: self.x,
                y: r.y.saturating_add(r.height),
                width: self.width,
                height: self
                    .height
                    .saturating_sub(relative_position.1.saturating_add(r.height)),
            });
            //right
            pieces.push(Rect {
                x: r.x.saturating_add(r.width),
                y: self.y,
                width: self
                    .width
                    .saturating_sub(relative_position.0.saturating_add(r.width)),
                height: self.height,
            });
            //left
            pieces.push(Rect {
                x: self.x,
                y: self.y,
                width: relative_position.0,
                height: self.height,
            });
            pieces.retain(|x| x.area() > 0);
        }
        pieces
    }

    pub fn can_contain(&self, r: &Rect) -> bool { self.width >= r.width && self.height >= r.height }

    pub fn contains(&self, r: &Rect) -> bool {
        !(r.x < self.x
            || r.y < self.y
            || r.x + r.width > self.x.saturating_add(self.width)
            || r.y + r.height > self.y.saturating_add(self.height))
    }

    pub fn distance_from_origin(&self) -> f64 {
        ((self.x.pow(2).saturating_add(self.y.pow(2))) as f64).sqrt()
    }

    pub fn area(&self) -> u32 { self.width.saturating_mul(self.height) }

    pub fn intersection(&self, r: &Rect) -> Rect {
        let x = cmp::max(self.x, r.x);
        let y = cmp::max(self.y, r.y);
        let width = cmp::min(
            self.x.saturating_add(self.width),
            r.x.saturating_add(r.width),
        )
        .saturating_sub(x);
        let height = cmp::min(
            self.y.saturating_add(self.height),
            r.y.saturating_add(r.height),
        )
        .saturating_sub(y);
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}
