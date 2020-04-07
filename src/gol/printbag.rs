use std::collections::HashMap;

pub struct PrintBag {
    mt: usize,
    map: HashMap<(isize, usize, usize), char>,
}

impl PrintBag {
    pub fn new(mt: usize) -> Self {
        PrintBag {
            mt: mt,
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, x: isize, y: usize, t: usize, c: char) {
        self.map.insert((x, y, t), c);
    }

    pub fn format(&self) -> Vec<String> {
        if self.map.is_empty() {
            return vec![];
        }
        let min_x = self.map.iter().map(|(&(x, _, _), _)| x).min().unwrap();
        let max_x = self.map.iter().map(|(&(x, _, _), _)| x).max().unwrap();
        let min_y = self.map.iter().map(|(&(_, y, _), _)| y).min().unwrap();
        let max_y = self.map.iter().map(|(&(_, y, _), _)| y).max().unwrap();
        let mut ret = Vec::new();
        for y in min_y..=max_y {
            let mut r = String::new();
            for t in 0..self.mt {
                if t > 0 {
                    r.push_str(" | ");
                }
                for x in min_x..=max_x {
                    r.push(self.map.get(&(x, y, t)).cloned().unwrap_or(' '));
                }
            }
            ret.push(r);
        }
        ret
    }
}
