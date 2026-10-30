// Stammbaum-Auswertung fuer den Reiter "Stammbaum" (reine Lese-Ansicht, es wird
// nichts veraendert).
//
// Beziehungen kommen direkt aus den Kartenfeldern w1 = Vater-B und w2 = Mutter-B,
// also genau der Paarung (w1,w2), aus der racfiche.c im FAM-CARD-LOOP (Zeile 589ff)
// die Familien des GEDCOM-Exports bildet: gleiche Elternpaarung => Geschwister,
// Kinder einer Person = Karten mit w1 == b oder w2 == b. Die L1/L2-Tabellen werden
// hierfuer nicht benttigt.
//
// Layout: ein Durchlauf "messen" (Breiten von unten nach oben), ein Durchlauf
// "setzen" (Positionen von oben nach unten). Teilbaeume werden seitlich aneinander
// gepackt, dadurch keine ueberlappungen. Vorfahren wachsen nach oben (Eltern-Kasten
// ueber dem Kind), Nachkommen nach unten (Familien-Kasten zwischen Eltern und Kindern).
// Jede Person wird nur ein Mal vollstaendig ausgeweitet; ein zweiter Verweis auf
// dieselbe Karte erscheint als Blatt "siehe oben". Das ist zugleich die
// Zyklus-Sicherung bei fehlerhaften Elternangaben.

use crate::rac::{decode_cp1252, fmt_date, Card};
use std::collections::HashMap;

pub const PW: f32 = 104.0; // Breite einer Personen-Box
pub const PH: f32 = 34.0; // Hoehe einer Personen-Box
pub const FAMW: f32 = 118.0; // Breite eines Familien-Kastens (Ehe-/Eltern-Marke)
pub const FAMH: f32 = 26.0; // Hoehe des Familien-Kastens
pub const GAPX: f32 = 16.0; // seitlicher Abstand zwischen Teilbaeumen
pub const ROWH: f32 = 104.0; // Reihenabstand Person -> Familienkasten -> Person
pub const ALL_GENS: usize = 99; // "alle Generationen"

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Anc, // Vorfahren: Eltern-Kaesten ueber der Person
    Desc, // Nachkommen: Familien-Kaesten unter der Person
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Person,
    Fam,
}

#[derive(Clone, Debug)]
pub struct TNode {
    pub kind: Kind,
    pub card: usize, // Person: Kartenindex; Fam: Karte des Kindes, das die Familie fuehrt
    pub gen: f32, // Generation relativ zur Wurzel (0), in Wachstumsrichtung wachsend
    pub x: f32, // Mittelpunkt in Layout-Koordinaten
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub slot: f32, // Platzbedarf des Teilbaums; nur w/h sind sichtbare Kastenmasse
    pub label: String,
    pub sub: String, // Lebensdaten bzw. Heirat der Eltern
    pub sex: u8, // b'H' / b'F' / 0
    pub royal: bool, // Beruf-Feld (s[2]) nennt king/queen, wie im GEDCOM-Baum
    pub dup: bool, // Person wurde schon gezeigt -> nicht erneut ausgeweitet
    pub ghost: bool, // eingeheirateter Partner (Desc): Box ohne eigene Verzweigung
    pub more: bool, // hier gaebe es weitere Generationen, nur die Grenze erreicht
    pub kids: Vec<usize>, // Person -> Familien-Knoten, Fam -> Personen-Knoten
    pub spouse: usize, // Fam (Desc): Partner-Box in der Elternreihe, usize::MAX = keiner
}

#[derive(Clone, Debug, Default)]
pub struct TreeView {
    pub nodes: Vec<TNode>,
    pub root: usize,
    pub root_card: usize,
    pub mode: Mode,
    pub gens: usize,
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub fam_count: usize,
    pub missing: usize, // Eltern-Schluessel ohne eigene Karte
    pub dups: usize, // Verweise auf eine bereits gezeigte Person
    pub truncated: bool,
}

impl TreeView {
    pub fn blank() -> TreeView {
        TreeView::default()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn person_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.kind == Kind::Person).count()
    }
    pub fn generations(&self) -> usize {
        let g = self.nodes.iter().map(|n| n.gen).fold(0.0f32, f32::max);
        g as usize + 1
    }
    pub fn width(&self) -> f32 {
        (self.max_x - self.min_x).max(1.0)
    }
    pub fn height(&self) -> f32 {
        (self.max_y - self.min_y).max(1.0)
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<usize> {
        // kleinste treffende Box zuerst: Personen vor Familienkaesten
        let mut best: Option<(f32, usize)> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if x >= n.x - n.w * 0.5
                && x <= n.x + n.w * 0.5
                && y >= n.y - n.h * 0.5
                && y <= n.y + n.h * 0.5
            {
                let a = n.w * n.h;
                if best.map_or(true, |(ba, _)| a < ba) {
                    best = Some((a, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "kein Baum".to_string();
        }
        let g = if self.gens >= ALL_GENS {
            "alle".to_string()
        } else {
            self.gens.to_string()
        };
        let mut s = format!(
            "{} Personen, {} Familien, {} Generationen ({}), {}",
            self.person_count(),
            self.fam_count,
            self.generations(),
            g,
            if self.mode == Mode::Anc {
                "aufwaerts (Vorfahren)"
            } else {
                "abwaerts (Nachkommen)"
            }
        );
        if self.missing > 0 {
            s += &format!(" \u{b7} {} Elternverweis ohne Karte", self.missing);
        }
        if self.dups > 0 {
            s += &format!(" \u{b7} {} Person zweimal erreicht", self.dups);
        }
        if self.truncated {
            s += " \u{b7} Tiefengrenze erreicht";
        }
        s
    }
}

// ---------------- Beziehungsindex ueber w1/w2 ----------------

#[derive(Clone, Debug, Default)]
pub struct Rel {
    by_b: HashMap<u16, usize>,
}

impl Rel {
    pub fn build(cards: &[Card]) -> Rel {
        let mut by_b = HashMap::new();
        for (i, c) in cards.iter().enumerate() {
            if c.b != 0 {
                by_b.entry(c.b).or_insert(i);
            }
        }
        Rel { by_b }
    }
    pub fn find(&self, b: u16) -> Option<usize> {
        if b == 0 {
            None
        } else {
            self.by_b.get(&b).copied()
        }
    }
    // Vater, Mutter; ein Verweis auf die eigene Karte wird ignoriert.
    pub fn parents(&self, cards: &[Card], i: usize) -> (Option<usize>, Option<usize>) {
        let k = &cards[i];
        (
            self.find(k.w1).filter(|j| *j != i),
            self.find(k.w2).filter(|j| *j != i),
        )
    }
    // Eltern-Schluessel gesetzt, aber Karte fehlt in der Datei (ueblich).
    pub fn parent_gap(&self, cards: &[Card], i: usize) -> usize {
        let k = &cards[i];
        let mut n = 0;
        if k.w1 != 0 && self.find(k.w1).is_none() {
            n += 1;
        }
        if k.w2 != 0 && self.find(k.w2).is_none() {
            n += 1;
        }
        n
    }
    // Kinder einer Person (w1/w2-Scan ueber alle Karten). Der Abwaerts-Zweig
    // arbeitet mit couples(); diese Einzelfunktion bleibt fuer Suchen erhalten.
    #[allow(dead_code)]
    pub fn children(&self, cards: &[Card], i: usize) -> Vec<usize> {
        let b = cards[i].b;
        if b == 0 {
            return Vec::new();
        }
        cards
            .iter()
            .enumerate()
            .filter(|(j, c)| *j != i && (c.w1 == b || c.w2 == b))
            .map(|(j, _)| j)
            .collect()
    }
    // Paare (Vater-B, Mutter-B) mit ihren gemeinsamen Kindern; i ist Teil des Paares.
    pub fn couples(&self, cards: &[Card], i: usize) -> Vec<(u16, u16, Vec<usize>)> {
        let b = cards[i].b;
        if b == 0 {
            return Vec::new();
        }
        let mut out: Vec<(u16, u16, Vec<usize>)> = Vec::new();
        for (j, c) in cards.iter().enumerate() {
            if j == i || (c.w1 != b && c.w2 != b) {
                continue;
            }
            let key = (c.w1, c.w2);
            match out.iter_mut().find(|(f, m, _)| (*f, *m) == key) {
                Some(e) => e.2.push(j),
                None => out.push((key.0, key.1, vec![j])),
            }
        }
        out
    }
    // Heirat der Eltern, abgelesen von einer ihrer Karten (Dateifeld Nr. 2 = Heirat).
    pub fn marriage(cards: &[Card], f: Option<usize>, m: Option<usize>) -> String {
        for k in [f, m].into_iter().flatten() {
            let d = fmt_date(&cards[k].dt[1].raw);
            if !d.is_empty() {
                return format!("verh. {}", d);
            }
        }
        String::new()
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n - 1).collect();
        format!("{}\u{2026}", t)
    }
}

// ---------------- Aufbau ----------------

struct Builder<'a> {
    cards: &'a [Card],
    rel: &'a Rel,
    nodes: Vec<TNode>,
    seen: HashMap<usize, usize>, // Karte -> Knoten mit voller Auswertung
    root: usize,
    mode: Mode,
    gens: usize,
    dir: f32, // Vorfahren: Reihen nach oben (-1), Nachkommen: nach unten (+1)
    fam_count: usize,
    missing: usize,
    dups: usize,
    truncated: bool,
}

impl<'a> Builder<'a> {
    fn y_of(&self, gen: f32) -> f32 {
        self.dir * gen * ROWH
    }

    fn person_node(&mut self, card: usize, gen: f32, dup: bool, ghost: bool) -> usize {
        let k = &self.cards[card];
        let name = decode_cp1252(&k.name);
        let label = if name.is_empty() {
            "(ohne Namen)".to_string()
        } else {
            clip(&name, 17)
        };
        let geb = fmt_date(&k.dt[0].raw);
        let tod = fmt_date(&k.dt[2].raw);
        // Die zweite Zeile ist knapp gehalten (schmale Box); die B-Nummer steht in
        // der Kopfzeile und im Hinweis oben links. Unbekannte Datumteile (leerer
        // Text, auch bei 0xFFFF in den Rohdaten) werden nicht mit "65535..." dargestellt.
        let sub = if dup {
            format!("siehe oben, B{}", k.b)
        } else if ghost {
            format!("eingeheiratet \u{b7} B{}", k.b)
        } else if geb.is_empty() && tod.is_empty() {
            format!("B{}", k.b)
        } else if geb.is_empty() {
            tod
        } else if tod.is_empty() {
            geb
        } else {
            format!("{} \u{2013} {}", geb, tod)
        };
        let occu = decode_cp1252(&k.s[2]).to_lowercase();
        let n = TNode {
            kind: Kind::Person,
            card,
            gen,
            x: 0.0,
            y: 0.0,
            w: PW,
            h: PH,
            slot: PW,
            label,
            sub,
            sex: k.sex,
            royal: occu.contains("king") || occu.contains("queen"),
            dup,
            ghost,
            more: false,
            kids: Vec::new(),
            spouse: usize::MAX,
        };
        let id = self.nodes.len();
        self.nodes.push(n);
        id
    }

    // Familien-Marke: kleiner, beschrifteter Balken zwischen den Personen-Reihen.
    // Er erhaelt immer eine Aufschrift, damit er nie als leerer weisser Kasten
    // erscheint (label = "Ehe" / "Eltern", sub = Heirat und Kinderzahl).
    fn fam_node(&mut self, card: usize, gen: f32, label: &str, sub: String) -> usize {
        let n = TNode {
            kind: Kind::Fam,
            card,
            gen,
            x: 0.0,
            y: 0.0,
            w: FAMW,
            h: FAMH,
            slot: FAMW,
            label: label.to_string(),
            sub,
            sex: 0,
            royal: false,
            dup: false,
            ghost: false,
            more: false,
            kids: Vec::new(),
            spouse: usize::MAX,
        };
        let id = self.nodes.len();
        self.nodes.push(n);
        self.fam_count += 1;
        id
    }

    // --- abwaerts: nachfolgende Generationen ---
    fn build_desc(&mut self, card: usize, gen: f32) -> usize {
        if self.seen.contains_key(&card) {
            self.dups += 1;
            return self.person_node(card, gen, true, false);
        }
        let pid = self.person_node(card, gen, false, false);
        self.seen.insert(card, pid);
        let mut couples = self.rel.couples(self.cards, card);
        if couples.is_empty() {
            return pid;
        }
        if gen as usize >= self.gens {
            self.nodes[pid].more = true;
            self.truncated = true;
            return pid;
        }
        let myb = self.cards[card].b;
        couples.sort_by_key(|(f, m, kids)| (kids.len(), *f, *m));
        for (fb, mb, kids) in couples.drain(..) {
            // Beschriftung der Ehe-Marke: Kinderzahl always, Heiratsdatum wenn bekannt
            let kin = if kids.len() == 1 {
                "1 Kind".to_string()
            } else {
                format!("{} Kinder", kids.len())
            };
            let mh = Rel::marriage(self.cards, self.rel.find(fb), self.rel.find(mb));
            let mh = mh.strip_prefix("verh. ").unwrap_or(&mh);
            let sub = if mh.is_empty() {
                kin
            } else {
                format!("{} \u{b7} verh. {}", kin, mh)
            };
            let fid = self.fam_node(card, gen + 0.5, "Ehe", sub);
            self.nodes[pid].kids.push(fid);
            for p in [fb, mb] {
                if p != 0 && self.rel.find(p).is_none() {
                    self.missing += 1;
                }
            }
            let other = if fb == myb { mb } else { fb };
            if let Some(j) = self.rel.find(other) {
                // Partner-Box: nicht weiter verzweigt und nicht in seen, sonst gaebe
                // die eigene Abstammung des Partners verloren.
                let sid = self.person_node(j, gen, false, true);
                self.nodes[fid].spouse = sid;
            }
            for c in kids {
                let cid = self.build_desc(c, gen + 1.0);
                self.nodes[fid].kids.push(cid);
            }
        }
        pid
    }

    // --- aufwaerts: vorhergehende Familien ---
    fn build_anc(&mut self, card: usize, gen: f32) -> usize {
        if self.seen.contains_key(&card) {
            self.dups += 1;
            return self.person_node(card, gen, true, false);
        }
        let pid = self.person_node(card, gen, false, false);
        self.seen.insert(card, pid);
        let (f, m) = self.rel.parents(self.cards, card);
        self.missing += self.rel.parent_gap(self.cards, card);
        if f.is_none() && m.is_none() {
            return pid;
        }
        if gen as usize >= self.gens {
            self.nodes[pid].more = true;
            self.truncated = true;
            return pid;
        }
        // Eltern-Marke zwischen dem Kind und der Elternreihe, immer beschriftet.
        let mh = Rel::marriage(self.cards, f, m);
        let mh = mh.strip_prefix("verh. ").unwrap_or(&mh);
        let sub = if mh.is_empty() {
            format!("Eltern von B{}", self.cards[card].b)
        } else {
            format!("verh. {}", mh)
        };
        let fid = self.fam_node(card, gen + 0.5, "Eltern", sub);
        self.nodes[pid].kids.push(fid);
        let mut members: Vec<usize> = Vec::new();
        for p in [f, m] {
            if let Some(j) = p {
                members.push(self.build_anc(j, gen + 1.0));
            }
        }
        self.nodes[fid].kids = members;
        pid
    }

    // --- Breiten messen (Postorder) ---
    fn pack_meas(&mut self, ids: &[usize]) -> f32 {
        let mut w = 0.0;
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                w += GAPX;
            }
            w += self.measure(*id);
        }
        w
    }
    fn measure(&mut self, id: usize) -> f32 {
        let kind = self.nodes[id].kind;
        let kids = self.nodes[id].kids.clone();
        let inner = self.pack_meas(&kids);
        let mut need = inner;
        if kind == Kind::Person && self.mode == Mode::Desc {
            // Partner-Boxen stehen in derselben Reihe wie die Person
            let mut nsp = 0usize;
            for f in &kids {
                let s = self.nodes[*f].spouse;
                if s != usize::MAX {
                    self.measure(s);
                    nsp += 1;
                }
            }
            if nsp > 0 {
                need = need.max(PW + nsp as f32 * (PW + 10.0));
            }
        }
        if kind == Kind::Fam {
            need = need.max(FAMW);
        }
        // Der Teilbaum bestimmt nur den frei gehaltenen Platz (slot); der
        // sichtbare Kasten bleibt klein, sonst wird ein Kasten so breit wie der
        // ganze Baum unter ihm.
        let slot = need.max(if kind == Kind::Fam { FAMW } else { PW });
        self.nodes[id].slot = slot;
        self.nodes[id].w = if kind == Kind::Fam { FAMW } else { PW };
        slot
    }
    fn kids_width(&self, ids: &[usize]) -> f32 {
        let mut w = 0.0;
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                w += GAPX;
            }
            w += self.nodes[*id].slot;
        }
        w
    }

    // --- Positionen setzen (Preorder) ---
    fn place(&mut self, id: usize, left: f32) {
        let kids = self.nodes[id].kids.clone();
        let kind = self.nodes[id].kind;
        let gen = self.nodes[id].gen;
        let w = self.nodes[id].slot;
        let cx = left + w * 0.5;
        let inner = self.kids_width(&kids);
        let mut px = cx;
        if kind == Kind::Person && self.mode == Mode::Desc {
            let mut spouses: Vec<usize> = Vec::new();
            for f in &kids {
                let s = self.nodes[*f].spouse;
                if s != usize::MAX {
                    spouses.push(s);
                }
            }
            if !spouses.is_empty() {
                // Paar-Reihe gemeinsam ueber den Kindern zentrieren
                let row = PW + spouses.len() as f32 * (PW + 10.0);
                px = left + (w - row) * 0.5 + PW * 0.5;
                for (i, s) in spouses.iter().enumerate() {
                    let sw = self.nodes[*s].w;
                    self.nodes[*s].x = px + PW * 0.5 + 10.0 + i as f32 * (PW + 10.0) + sw * 0.5;
                    self.nodes[*s].y = self.y_of(gen);
                }
            }
        }
        self.nodes[id].x = px;
        self.nodes[id].y = self.y_of(gen);
        let mut x = cx - inner * 0.5;
        for k in &kids {
            self.place(*k, x);
            x += self.nodes[*k].slot + GAPX;
        }
    }

    fn finish(mut self) -> TreeView {
        let root = self.root;
        self.measure(root);
        self.place(root, 0.0);
        let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for n in &self.nodes {
            mnx = mnx.min(n.x - n.w * 0.5);
            mxx = mxx.max(n.x + n.w * 0.5);
            mny = mny.min(n.y - n.h * 0.5);
            mxy = mxy.max(n.y + n.h * 0.5);
        }
        TreeView {
            nodes: self.nodes,
            root,
            root_card: usize::MAX,
            mode: self.mode,
            gens: self.gens,
            min_x: mnx,
            min_y: mny,
            max_x: mxx,
            max_y: mxy,
            fam_count: self.fam_count,
            missing: self.missing,
            dups: self.dups,
            truncated: self.truncated,
        }
    }
}

// Wurzel-Karte, Richtung und Tiefengrenze ("alle Generationen" = ALL_GENS).
pub fn build_tree(cards: &[Card], rel: &Rel, root_card: usize, mode: Mode, gens: usize) -> TreeView {
    if root_card >= cards.len() {
        return TreeView::blank();
    }
    let gens = if gens == 0 { 1 } else { gens };
    let mut b = Builder {
        cards,
        rel,
        nodes: Vec::new(),
        seen: HashMap::new(),
        root: 0,
        mode,
        gens,
        dir: if mode == Mode::Anc { -1.0 } else { 1.0 },
        fam_count: 0,
        missing: 0,
        dups: 0,
        truncated: false,
    };
    b.root = if mode == Mode::Anc {
        b.build_anc(root_card, 0.0)
    } else {
        b.build_desc(root_card, 0.0)
    };
    let mut v = b.finish();
    v.root_card = root_card;
    v
}

// ---------------- Tests ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rac::Card;

    fn person(name: &str, b: u16, sex: u8, w1: u16, w2: u16) -> Card {
        let mut c = Card::new(b);
        c.name = name.as_bytes().to_vec();
        c.sex = sex;
        c.w1 = w1;
        c.w2 = w2;
        c
    }

    // Grosseltern (B 1,2) -> Vater 3 + Mutter 4 -> Kinder 5 und 6
    fn family() -> Vec<Card> {
        vec![
            person("Grossvater", 1, b'H', 0, 0),
            person("Grossmutter", 2, b'F', 0, 0),
            person("Vater", 3, b'H', 1, 2),
            person("Mutter", 4, b'F', 0, 0),
            person("Kind A", 5, b'H', 3, 4),
            person("Kind B", 6, b'F', 3, 4),
        ]
    }

    #[test]
    fn beziehungen() {
        let cs = family();
        let rel = Rel::build(&cs);
        let (f, m) = rel.parents(&cs, 4);
        assert_eq!(f, Some(2)); // Karte 2 = Vater
        assert_eq!(m, Some(3));
        assert_eq!(rel.children(&cs, 2), vec![4, 5]);
        let cp = rel.couples(&cs, 2);
        assert_eq!(cp.len(), 1);
        assert_eq!(cp[0].2, vec![4, 5]);
        assert_eq!(rel.couples(&cs, 0).len(), 1); // Grossvater: eine Familie
        assert_eq!(rel.couples(&cs, 3).len(), 1); // Mutter: dieselbe Familie
    }

    #[test]
    fn vorfahren_drei_generationen() {
        let cs = family();
        let rel = Rel::build(&cs);
        let v = build_tree(&cs, &rel, 4, Mode::Anc, 3);
        assert_eq!(v.nodes[v.root].card, 4);
        assert_eq!(v.person_count(), 5); // Kind, Vater, Mutter, Grosseltern
        assert_eq!(v.fam_count, 2);
        assert_eq!(v.dups, 0);
        assert!(v
            .nodes
            .iter()
            .any(|n| n.kind == Kind::Person && n.card == 0 && (n.gen - 2.0).abs() < 0.01));
        assert!(v.min_x < v.max_x);
        assert!(v.min_y < v.max_y);
        // Eltern-Marke: zwischen dem Kind und der Elternreihe, immer beschriftet
        let fam = v.nodes.iter().find(|n| n.kind == Kind::Fam).unwrap();
        assert_eq!(fam.label, "Eltern");
        assert!(!fam.sub.is_empty());
        assert!(fam.y < v.nodes[v.root].y); // aufwaerts: ueber dem Kind
        assert!(fam.y > v.nodes[fam.kids[0]].y); // und unterhalb der Eltern
        assert_eq!(v.nodes[fam.kids[0]].y, v.nodes[fam.kids[1]].y);
    }

    #[test]
    fn nachkommen_und_partner() {
        let cs = family();
        let rel = Rel::build(&cs);
        let v = build_tree(&cs, &rel, 0, Mode::Desc, 3);
        assert_eq!(v.person_count(), 6);
        assert_eq!(v.fam_count, 2);
        assert_eq!(v.dups, 0);
        let root = &v.nodes[v.root];
        assert!(v.nodes.iter().any(|n| n.ghost && n.card == 1)); // Grossmutter als Partner-Box
        let sp = v.nodes.iter().find(|n| n.ghost && n.card == 1).unwrap();
        assert_eq!(sp.y, root.y); // Partner in derselben Reihe
        assert!(sp.x > root.x); // rechts neben der Person
        let kind = v.nodes.iter().find(|n| n.kind == Kind::Person && n.card == 4).unwrap();
        assert!((kind.gen - 2.0).abs() < 0.01);
        assert!(kind.y > root.y); // Nachkommen wachsen nach unten
        let fam = v.nodes.iter().find(|n| n.kind == Kind::Fam).unwrap();
        assert!(fam.y > root.y && fam.y < kind.y); // Kasten zwischen Eltern und Kind
        // jeder Familien-Kasten traegt eine Aufschrift, nie ein leeres Feld
        for n in v.nodes.iter().filter(|n| n.kind == Kind::Fam) {
            assert_eq!(n.label, "Ehe");
            assert!(!n.sub.is_empty());
        }
        assert!(v
            .nodes
            .iter()
            .any(|n| n.kind == Kind::Fam && n.sub.contains("2 Kinder")));
    }

    // Kein sichtbarer Kasten darf einen anderen verdecken, und keiner darf
    // breiter sein als ein Personen-Kasten: sonst wirkt der Baum vollig
    // ueberfuellt und die Familien-Marken sehen aus wie leere weisse Flaeche.
    fn keine_ueberlappung(v: &TreeView) {
        for i in 0..v.nodes.len() {
            let n = &v.nodes[i];
            assert!(
                n.w <= 120.0 && n.h <= 40.0,
                "Kasten {} zu gross: {} x {}",
                i,
                n.w,
                n.h
            );
            for j in i + 1..v.nodes.len() {
                let m = &v.nodes[j];
                let x = (n.x - m.x).abs() < (n.w + m.w) * 0.5 - 0.5;
                let y = (n.y - m.y).abs() < (n.h + m.h) * 0.5 - 0.5;
                assert!(!(x && y), "Kaesten {i} und {j} ueberlappen einander");
            }
        }
    }

    #[test]
    fn familienkaesten_ueberlappen_nicht() {
        // Die Familien-Marke sitzt immer ZWISCHEN zwei Reihen und ist kleiner als
        // ein Personen-Kasten; sonst steht sie als helle Flaeche hinter den
        // Personen und wirkt wie ein leerer Kasten.
        let cs = family();
        let rel = Rel::build(&cs);
        for mode in [Mode::Anc, Mode::Desc] {
            let v = build_tree(&cs, &rel, 4, mode, 3);
            assert!(v
                .nodes
                .iter()
                .filter(|n| n.kind == Kind::Fam)
                .all(|f| !f.label.is_empty() && !f.sub.is_empty()));
            keine_ueberlappung(&v);
        }
    }

    #[test]
    fn partner_ohne_eigene_verzweigung() {
        // Kind 20 haengt an Paar (30,31); beide Elternteile werden gezeigt
        let cs = vec![
            person("Kind", 20, b'H', 30, 31),
            person("Vater", 30, b'H', 0, 0),
            person("Mutter", 31, b'F', 0, 0),
        ];
        let rel = Rel::build(&cs);
        let v = build_tree(&cs, &rel, 1, Mode::Desc, ALL_GENS);
        assert_eq!(v.person_count(), 3);
        assert_eq!(v.dups, 0);
        assert_eq!(v.fam_count, 1);
    }

    #[test]
    fn stammbaum_laeuft_nicht_endlos() {
        // 10 ist Kind von 11 und 11 Kind von 10 -> Kreis, muss terminieren
        let cs = vec![person("A", 10, b'H', 11, 0), person("B", 11, b'H', 10, 0)];
        let rel = Rel::build(&cs);
        let v = build_tree(&cs, &rel, 0, Mode::Anc, ALL_GENS);
        assert_eq!(v.dups, 1);
        assert_eq!(v.person_count(), 3); // A, B und A als Verweis
        assert!(v.nodes.len() < 10);
        // Stammbaum-Kollision: Groessmutter ueber Vater und ueber Mutter identisch
        let cs2 = vec![
            person("Kind", 100, b'H', 101, 102),
            person("Vater", 101, b'H', 0, 103),
            person("Mutter", 102, b'F', 0, 103),
            person("Groessmutter", 103, b'F', 0, 0),
        ];
        let rel2 = Rel::build(&cs2);
        let v2 = build_tree(&cs2, &rel2, 0, Mode::Anc, ALL_GENS);
        assert_eq!(v2.dups, 1);
        assert!(v2.nodes.iter().any(|n| n.dup));
        assert_eq!(v2.person_count(), 5);
    }

    // Echte Daten: ../racines-port/data/FICHES.004 = "Royal Families of England".
    #[test]
    #[ignore]
    fn echte_koenigsfamilie() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../racines-port/data/FICHES.004"
        );
        let buf = std::fs::read(path).expect("FICHES.004 lesen");
        let cards = crate::rac::parse_file(&buf).expect("parse_file");
        let rel = Rel::build(&cards);
        let mut root = 0usize;
        for (i, c) in cards.iter().enumerate() {
            if decode_cp1252(&c.name).contains("Victoria") && c.w1 != 0 {
                root = i;
                break;
            }
        }
        let a = build_tree(&cards, &rel, root, Mode::Anc, 3);
        assert!(a.person_count() > 3);
        assert!(a.fam_count > 1);
        let d = build_tree(&cards, &rel, root, Mode::Desc, ALL_GENS);
        assert!(d.person_count() > 3);
        let full = build_tree(&cards, &rel, 0, Mode::Desc, ALL_GENS);
        assert!(full.person_count() >= 1);
        keine_ueberlappung(&a);
        keine_ueberlappung(&d);
        keine_ueberlappung(&full);
        println!("anc: {}", a.summary());
        println!("desc: {}", d.summary());
    }
}
