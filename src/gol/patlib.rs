use ars_ds::scalar::UScalar;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

pub struct GolSlice {
    r0: HashSet<(isize, usize)>,
    r1: HashSet<(isize, usize)>,
    mt: usize,
}

impl GolSlice {
    pub fn encode<B: UScalar>(&self, mx: usize, mt: usize) -> Option<(B, B)> {
        assert_eq!(self.mt, mt);

        let min_x = self.r0.iter().chain(self.r1.iter()).map(|&(x, _)| x).min().unwrap();

        let mut r0 = B::zero();
        let mut r1 = B::zero();
        for &(x, t) in self.r0.iter() {
            let x = (x - min_x) as usize;
            if x >= mx {
                return None;
            }
            r0.set_bit(t * mx + x, true);
        }
        for &(x, t) in self.r1.iter() {
            let x = (x - min_x) as usize;
            if x >= mx {
                return None;
            }
            r1.set_bit(t * mx + x, true);
        }

        Some((r0, r1))
    }
}

struct GolPattern {
    cells: HashSet<(isize, isize, usize)>,
    ox: isize,
    oy: isize,
    mt: usize,
}

impl GolPattern {
    fn find_slices(&self, acc: &mut Vec<GolSlice>, ox: isize, oy: isize, mt: usize) {
        if mt % self.mt != 0 {
            return;
        }

        let mul = mt / self.mt;

        // for each orientation
        for &fx in &[-1, 1] {
            for &fy in &[-1, 1] {
                for &sw in &[false, true] {
                    let mut ox2 = self.ox * fx;
                    let mut oy2 = self.oy * fy;
                    if sw {
                        std::mem::swap(&mut ox2, &mut oy2);
                    }

                    if ox2 * (mul as isize) != ox {
                        continue;
                    }
                    if oy2 * (mul as isize) != oy {
                        continue;
                    }

                    // for each phase
                    for dt in 0..self.mt {
                        let cells2: HashSet<_> = self.cells.iter().map(|&(x, y, t)| {
                            // rephase (x, y, t) -> (x2, y2, t2)
                            let mut x2 = x;
                            let mut y2 = y;
                            let mut t2 = t + dt;
                            if t2 >= self.mt {
                                t2 -= self.mt;
                                x2 -= self.ox;
                                y2 -= self.oy;
                            }

                            (0..mul).map(|r| {
                                // duplicate (x2, y2, t2) -> (x3, y3, t3) after r reps of self.mt
                                let mut x3 = x2 + (r as isize) * self.ox;
                                let mut y3 = y2 + (r as isize) * self.oy;
                                let t3 = t2 + r * self.mt;

                                // reorient (x3, y3, t3)
                                x3 *= fx;
                                y3 *= fy;
                                if sw {
                                    std::mem::swap(&mut x3, &mut y3);
                                }

                                // update (x3, y3, t3) for mid-gen shifts
                                let sx = ((t3 as isize) * ox) / (mt as isize);
                                let sy = ((t3 as isize) * oy) / (mt as isize);
                                x3 -= sx;
                                y3 -= sy;

                                (x3, y3, t3)
                            }).collect::<Vec<_>>()
                        }).flatten().collect();

                        let min_y = cells2.iter().map(|&(_, y, _)| y).min().unwrap();
                        let max_y = cells2.iter().map(|&(_, y, _)| y).max().unwrap();

                        for y0 in (min_y - 1)..=max_y {
                            let filter_shift = |yc, (x, y, t)| {
                                if y == yc {
                                    Some((x, t))
                                }
                                else {
                                    None
                                }
                            };
                            let r0 = cells2.iter().filter_map(|&p| filter_shift(y0, p)).collect();
                            let y1 = y0 + 1;
                            let r1 = cells2.iter().filter_map(|&p| filter_shift(y1, p)).collect();
                            acc.push(GolSlice {
                                r0: r0,
                                r1: r1,
                                mt: self.mt * mul,
                            });
                        }
                    }
                }
            }
        }
    }
}

pub struct GolPatterns {
    vec: Vec<GolPattern>,
}

impl GolPatterns {
    pub fn new() -> Self {
        GolPatterns {
            vec: Vec::new(),
        }
    }

    pub fn load(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();

        if std::fs::metadata(path).unwrap().is_dir() {
            for e in std::fs::read_dir(path).unwrap() {
                self.load(e.unwrap().path());
            }
            return;
        }

        let file = File::open(path).unwrap();
        let br = BufReader::new(file);
        let lines: Vec<_> = br.lines().map(|r| r.unwrap()).collect();
        let mut lines = lines.iter().peekable();

        // strip comments
        while let Some(line) = lines.peek() {
            if line.starts_with("#") {
                lines.next();
                continue;
            }
            break;
        }

        assert!(lines.next().unwrap().starts_with("x = "));

        let mut cells = HashSet::new();
        let mut y = 0;
        let mut x = 0;
        for line in lines {
            let mut cs = line.chars().peekable();
            'parse_line: loop {
                // first read multiplier
                let mut m = 0;
                let mut ml = 0;
                while let Some(&c) = cs.peek() {
                    if '0' <= c && c <= '9' {
                        m *= 10;
                        m += (c as usize) - ('0' as usize);
                        ml += 1;
                        cs.next().unwrap();
                        continue;
                    }
                    break;
                }

                if ml == 0 {
                    m = 1;
                }

                // and interpret what it applies to
                let c = cs.next();
                for _ in 0..m {
                    match c {
                        Some('$') => {
                            x = 0;
                            y += 1;
                        }
                        Some('o') => {
                            cells.insert((x, y));
                            x += 1;
                        }
                        Some('b') => {
                            x += 1;
                        }
                        Some('!') => {
                            assert_eq!(0, ml);
                            // whatever...
                        }
                        None => {
                            assert_eq!(0, ml);
                            break 'parse_line;
                        }
                        _ => {
                            panic!("c {:?}", c);
                        }
                    }
                }
            }
        }

        let (cells0, _, _) = realign(&cells);
        let mut all_cells = HashSet::new();
        let mut all_links = Vec::new();
        let mut cells = cells0.clone();

        for t in 0..100 {
            for &(x, y) in cells.iter() {
                all_cells.insert((x, y, t));
            }
            let (cells2, links) = step(&cells);
            let (cells3, ox, oy) = realign(&cells2);
            if cells3 == cells0 {
                let mt = t + 1;
                for ((x1, y1, t1), (x2, y2, t2)) in links {
                    let p1 = (x1, y1, t + t1);
                    let p1 = wrap(p1, ox, oy, mt);
                    let p2 = (x2, y2, t + t2);
                    let p2 = wrap(p2, ox, oy, mt);
                    all_links.push((p1, p2));
                }
                for cells in components(all_cells, all_links) {
                    self.vec.push(GolPattern {
                        cells: cells,
                        ox: ox,
                        oy: oy,
                        mt: mt,
                    });
                }
                return
            }
            for ((x1, y1, t1), (x2, y2, t2)) in links {
                let p1 = (x1, y1, t + t1);
                let p2 = (x2, y2, t + t2);
                all_links.push((p1, p2));
            }
            cells = cells2;
        }

        panic!();
    }

    pub fn find_slices(&self, ox: isize, oy: isize, mt: usize) -> Vec<GolSlice> {
        let mut acc = Vec::new();
        for pat in self.vec.iter() {
            pat.find_slices(&mut acc, -ox, -oy, mt);
        }
        acc
    }
}

fn realign(cells: &HashSet<(isize, isize)>) -> (HashSet<(isize, isize)>, isize, isize) {
    let min_x = cells.iter().map(|&(x, _)| x).min().unwrap();
    let min_y = cells.iter().map(|&(_, y)| y).min().unwrap();
    (cells.iter().map(|&(x, y)| (x - min_x, y - min_y)).collect(), min_x, min_y)
}

fn wrap((x, y, t): (isize, isize, usize), ox: isize, oy: isize, mt: usize) -> (isize, isize, usize) {
    let mut x = x;
    let mut y = y;
    let mut t = t;
    while t >= mt {
        x -= ox;
        y -= oy;
        t -= mt;
    }
    (x, y, t)
}

fn step(cells: &HashSet<(isize, isize)>) -> (HashSet<(isize, isize)>, Vec<((isize, isize, usize), (isize, isize, usize))>) {
    let mut cts = HashMap::new();
    for &(x, y) in cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                let x2 = x + dx;
                let y2 = y + dy;
                cts.entry((x2, y2)).or_insert_with(|| Vec::new()).push((x, y));
            }
        }
    }
    let mut cells2 = HashSet::new();
    let mut links = Vec::new();
    for ((x, y), ns) in cts {
        let ct = ns.len();
        let cur = cells.contains(&(x, y));
        let fut = match cur {
            true => (3 <= ct && ct <= 4),
            false => (ct == 3),
        };
        if cur || ct >= 3 {
            // link neighborhood
            for (x2, y2) in ns {
                links.push(((x, y, 0), (x2, y2, 0)));
            }
        }
        if fut {
            // link present to future
            cells2.insert((x, y));
            links.push(((x, y, 0), (x, y, 1)));
        }
    }
    (cells2, links)
}

fn components<T: Hash + Eq + Copy>(ss: HashSet<T>, l0: Vec<(T, T)>) -> Vec<HashSet<T>> {
    let mut l = HashMap::new();
    for (t1, t2) in l0 {
        l.entry(t1).or_insert_with(|| HashSet::new()).insert(t2);
        l.entry(t2).or_insert_with(|| HashSet::new()).insert(t1);
    }
    let mut already = HashSet::new();
    let mut ret = Vec::new();
    for &s0 in ss.iter() {
        if already.contains(&s0) {
            continue;
        }
        let mut component = HashSet::new();
        let mut q = vec![s0];
        while let Some(s1) = q.pop() {
            if !already.insert(s1) {
                continue;
            }
            if ss.contains(&s1) {
                component.insert(s1);
            }
            if let Some(s2s) = l.get(&s1) {
                for &s2 in s2s {
                    q.push(s2);
                }
            }
        }
        ret.push(component);
    }
    ret
}
