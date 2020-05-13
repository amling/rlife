use std::collections::HashMap;

struct Range(Option<(isize, isize)>);

impl Range {
    fn touch(&mut self, v: isize) {
        self.0 = Some(match self.0 {
            None => (v, v),
            Some((min, max)) => (min.min(v), max.max(v)),
        });
    }

    fn iter(&self) -> impl Iterator<Item=isize> {
        let (min, max) = self.0.unwrap();

        min..=max
    }
}

pub struct PrintBag {
    xr: Range,
    yr: Range,
    tr: Range,
    map: HashMap<(isize, isize, isize), char>,
}

impl PrintBag {
    pub fn new() -> Self {
        PrintBag {
            xr: Range(None),
            yr: Range(None),
            tr: Range(None),
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, x: isize, y: isize, t: isize, c: char) {
        self.map.insert((x, y, t), c);
        self.xr.touch(x);
        self.yr.touch(y);
        self.tr.touch(t);
    }

    pub fn format(&self) -> Vec<String> {
        if self.map.is_empty() {
            return vec![];
        }
        let mut ret = Vec::new();
        for y in self.yr.iter() {
            let mut r = String::new();
            for t in self.tr.iter() {
                if t > 0 {
                    r.push_str(" | ");
                }
                for x in self.xr.iter() {
                    r.push(self.map.get(&(x, y, t)).cloned().unwrap_or(' '));
                }
            }
            ret.push(r);
        }
        ret
    }
}
