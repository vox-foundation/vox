//! S2: the noisy printer.

use vox_diagram::ir::{Direction, Shape};

use super::{Item, Model, Noise, ident, walk, word};

const COMMENTS: [&str; 10] = [
    " note",
    "",
    "x",
    "  spaced  ",
    " // nested",
    " vox: fine",
    " \u{e9} \u{fc}",
    "\t tabbed",
    "->",
    " { } \" '",
];

pub(super) struct Printer {
    pub(super) out: String,
    pub(super) dice: Vec<u32>,
    pub(super) i: usize,
    paths: Vec<String>,
    ctr: usize,
}

impl Printer {
    /// A printer drawing its dice from `n` and carrying no graph paths (sequence printing).
    pub(super) fn new(n: &Noise) -> Self {
        Printer {
            out: String::new(),
            dice: n.dice.clone(),
            i: 0,
            paths: Vec::new(),
            ctr: 0,
        }
    }

    pub(super) fn d(&mut self, n: usize) -> usize {
        let v = self.dice[self.i % self.dice.len()] as usize % n;
        self.i += 1;
        v
    }

    /// True once in `k` rolls; never on a zero die, so shrinking drops the noise.
    pub(super) fn chance(&mut self, k: usize) -> bool {
        self.d(k) == k - 1
    }

    pub(super) fn blanks(&mut self) {
        let n = match self.d(8) {
            5 => 1,
            6 => 2,
            7 => 3,
            _ => 0,
        };
        self.out.push_str(&"\n".repeat(n));
    }

    pub(super) fn sp(&mut self) -> String {
        " ".repeat(1 + self.d(3))
    }

    pub(super) fn sp0(&mut self) -> String {
        " ".repeat(self.d(3))
    }

    pub(super) fn ind(&mut self, level: usize) -> String {
        ["    ", "  ", "\t", " ", ""][self.d(5)].repeat(level)
    }

    pub(super) fn comment_text(&mut self) -> &'static str {
        COMMENTS[self.d(COMMENTS.len())]
    }

    fn own_comment(&mut self, level: usize) {
        let (ind, text) = (self.ind(level), self.comment_text());
        self.out.push_str(&format!("{ind}//{text}\n"));
    }

    /// Blank lines, maybe an own-line comment, then the indent of the statement line.
    pub(super) fn begin(&mut self, level: usize) {
        self.blanks();
        if self.chance(6) {
            self.own_comment(level);
            if self.chance(4) {
                self.blanks();
            }
        }
        let ind = self.ind(level);
        self.out.push_str(&ind);
    }

    /// Spaces, maybe a trailing comment, then the newline.
    pub(super) fn end_line(&mut self) {
        let pad = self.sp0();
        self.out.push_str(&pad);
        if self.chance(4) {
            let text = self.comment_text();
            self.out.push_str(&format!("//{text}"));
        }
        self.out.push('\n');
    }

    pub(super) fn label(&mut self, l: &str) {
        self.out.push('"');
        for c in l.chars() {
            match c {
                '\\' => self.out.push_str("\\\\"),
                '"' => self.out.push_str("\\\""),
                '\n' => self.out.push_str("\\n"),
                c => self.out.push(c),
            }
        }
        self.out.push('"');
    }

    pub(super) fn colon(&mut self) {
        let (a, b) = (self.sp0(), self.sp0());
        self.out.push_str(&format!("{a}:{b}"));
    }

    fn block(&mut self, items: &[Item], level: usize, dir: Option<(Direction, usize)>) {
        let slot = |i: usize| dir.filter(|(_, p)| p % (items.len() + 1) == i);
        for (i, it) in items.iter().enumerate() {
            if let Some((d, _)) = slot(i) {
                self.direction(d, level);
            }
            self.item(it, level);
        }
        if let Some((d, _)) = slot(items.len()) {
            self.direction(d, level);
        }
    }

    fn direction(&mut self, d: Direction, level: usize) {
        self.begin(level);
        self.out.push_str("direction");
        self.colon();
        self.out.push_str(&word(d));
        self.end_line();
    }

    fn attr(&mut self, s: Shape, level: usize) {
        let w = word(s);
        if s == Shape::Rect {
            if self.chance(4) {
                self.out.push_str(" { shape: rect }");
            }
            return;
        }
        match self.d(4) {
            0 => self.out.push_str(&format!(" {{ shape: {w} }}")),
            1 => self.out.push_str(&format!(" {{shape:{w}}}")),
            2 => self.out.push_str(&format!(" {{ shape: {w}, }}")),
            _ => {
                let (inner, close) = (self.ind(level + 1), self.ind(level));
                self.out
                    .push_str(&format!(" {{\n{inner}shape: {w}\n{close}}}"));
            }
        }
    }

    fn item(&mut self, it: &Item, level: usize) {
        match it {
            Item::Node {
                prefix,
                kebab,
                label,
                shape,
            } => {
                let id = ident(*prefix, *kebab, self.ctr);
                self.ctr += 1;
                self.begin(level);
                let sp = self.sp();
                self.out.push_str(&format!("node{sp}{id}"));
                self.colon();
                self.label(label);
                self.attr(*shape, level);
                self.end_line();
            }
            Item::Container {
                prefix,
                kebab,
                label,
                items,
            } => {
                let id = ident(*prefix, *kebab, self.ctr);
                self.ctr += 1;
                self.begin(level);
                let sp = self.sp();
                self.out.push_str(&format!("container{sp}{id}"));
                self.colon();
                self.label(label);
                if items.is_empty() && self.chance(4) {
                    self.out.push_str(" { }");
                    return self.end_line();
                }
                let pad = self.sp0();
                self.out.push_str(&format!("{pad}{{"));
                if self.chance(4) {
                    let (pad, text) = (self.sp0(), self.comment_text());
                    self.out.push_str(&format!("{pad}//{text}"));
                }
                self.out.push('\n');
                self.block(items, level + 1, None);
                self.begin(level);
                self.out.push('}');
                self.end_line();
            }
            Item::Edge { from, to, label } => {
                let n = self.paths.len();
                if n == 0 {
                    return;
                }
                let (f, t) = (self.paths[from % n].clone(), self.paths[to % n].clone());
                self.begin(level);
                let (a, b) = (self.sp0(), self.sp0());
                self.out.push_str(&format!("{f}{a}->{b}{t}"));
                if let Some(l) = label {
                    self.colon();
                    self.label(l);
                }
                self.end_line();
            }
        }
    }

    pub(super) fn comment_lines(&mut self, k: usize) {
        for _ in 0..2 {
            if self.chance(k) {
                self.blanks();
                self.own_comment(0);
            }
        }
    }
}

/// S2: print `m` with `n`'s layout noise. The text is always valid and denotes `m`.
pub fn print(m: &Model, n: &Noise) -> String {
    let paths = walk(m).node_paths;
    let mut p = Printer {
        out: String::new(),
        dice: n.dice.clone(),
        i: 0,
        paths,
        ctr: 0,
    };
    p.comment_lines(3);
    p.blanks();
    let pad = p.sp0();
    p.out.push_str(&format!("graph{pad}{{"));
    if p.chance(4) {
        let (pad, text) = (p.sp0(), p.comment_text());
        p.out.push_str(&format!("{pad}//{text}"));
    }
    p.out.push('\n');
    p.block(&m.items, 1, m.direction);
    p.begin(0);
    p.out.push('}');
    p.end_line();
    p.comment_lines(3);
    if p.chance(5) {
        p.out.pop();
    }
    if n.crlf {
        p.out.replace('\n', "\r\n")
    } else {
        p.out
    }
}
