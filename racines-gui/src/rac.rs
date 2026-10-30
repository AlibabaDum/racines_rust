// racines-core: Lesen/Schreiben der FICHES-Kartendateien der Ahnengalerie 1.51.
// Bytegenaues Spiegelbild von racfiche.c (parse_card Zeile 95, write_card Zeile 166,
// ensure_backup Zeile 699, patch_dossier Zeile 719, set_date Zeile 774).
// Karten: ab Offset 0, je 01 01 01 01 Marke, dann festes Feldgrammatik, dicht gepackt,
// letzte Karte endet an der Datei-EOF. Texte auf der Platte in cp1252.

use std::path::{Path, PathBuf};

// ---------------- cp1252 ----------------

const CP1252_HI: [char; 32] = [
    '\u{20AC}', '\u{F781}', '\u{201A}', '\u{192}',  '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2C6}',  '\u{2030}', '\u{160}',  '\u{2039}', '\u{152}',  '\u{F78D}', '\u{17D}',  '\u{F78F}',
    '\u{F790}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',  '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2DC}',  '\u{2122}', '\u{161}',  '\u{203A}', '\u{153}',  '\u{F79D}', '\u{17E}',  '\u{178}',
];

// Undefinierte Bytes 0x81,0x8D,0x8F,0x90,0x9D werden in den lateinischen
// Ergaenzungs-Sonderbereich U+F780..U+F7FF gepackt, damit jede Datei
// verlustfrei hin- und zurueckkonvertiert.
pub fn decode_cp1252(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len());
    for &x in b {
        if x < 0x80 {
            s.push(x as char);
        } else if x < 0xA0 {
            s.push(CP1252_HI[(x - 0x80) as usize]);
        } else {
            s.push(x as char);
        }
    }
    s
}

pub fn encode_cp1252(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len());
    for c in s.chars() {
        let u = c as u32;
        if u < 0x80 {
            v.push(u as u8);
        } else if u >= 0xF781 && u <= 0xF7FF {
            v.push(u as u8); // nur 0x81..0x9F relevant; 0xA0.. ausserhalb des Bereichs
        } else if u >= 0xA0 && u <= 0xFF {
            v.push(u as u8);
        } else if let Some(i) = CP1252_HI.iter().position(|&x| x == c) {
            v.push(0x80 + i as u8);
        } else {
            v.push(b'?');
        }
    }
    v
}

// ---------------- Kartenmodell ----------------

#[derive(Clone, Debug, Default)]
pub struct DBlock {
    pub raw: [u8; 12],   // [q u16][d u16][m u16][y u16][x1 u16][x2 u16] (logische Lesart)
    pub place: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Ev {
    pub v: u16,
    pub raw: [u8; 12],
    pub txt: Vec<u8>,
    pub a: u16,
    pub b: u16,
}

#[derive(Clone, Debug)]
pub struct Card {
    pub name: Vec<u8>,
    pub a: u16,
    pub b: u16,          // Personalschluessel: persistent, NICT verwechseln mit Listenposition
    pub c: u16,
    pub s: [Vec<u8>; 5],
    pub asz: u16,
    pub arr: [Vec<u8>; 5],
    pub dt: [DBlock; 3], // Dateireihenfolge = Geburt, Heirat, Tod
    pub sex: u8,         // b'H' / b'F'
    pub u67: [u16; 6],
    pub style: [u8; 46],
    pub w1: u16,         // Vater-B
    pub w2: u16,         // Mutter-B
    pub l1: Vec<(u16, u16)>,
    pub l2: Vec<Ev>,
    pub l3: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Default for Card {
    fn default() -> Self {
        Card {
            name: Vec::new(),
            a: 0, b: 0, c: 0,
            s: Default::default(),
            asz: 0,
            arr: Default::default(),
            dt: Default::default(),
            sex: 0,
            u67: [0; 6],
            style: [0; 46],
            w1: 0, w2: 0,
            l1: Vec::new(),
            l2: Vec::new(),
            l3: Vec::new(),
        }
    }
}

impl Card {
    // Neue Karte: Schluessel B = max(B) + 1, wie der C-Schreiber new_card.
    pub fn new(b: u16) -> Card {
        let mut c = Card::default();
        c.b = b;
        c.sex = b'H';
        c
    }
}

// ---------------- Leser ----------------

struct Reader<'a> {
    buf: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn u16(&mut self) -> Result<u16, String> {
        if self.p + 2 > self.buf.len() {
            return Err("u16 ueber Dateigrenze".into());
        }
        let v = u16::from_le_bytes([self.buf[self.p], self.buf[self.p + 1]]);
        self.p += 2;
        Ok(v)
    }
    fn u8(&mut self) -> Result<u8, String> {
        if self.p + 1 > self.buf.len() {
            return Err("u8 ueber Dateigrenze".into());
        }
        let v = self.buf[self.p];
        self.p += 1;
        Ok(v)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.p + n > self.buf.len() {
            return Err("Bytes ueber Dateigrenze".into());
        }
        let s = &self.buf[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn str(&mut self) -> Result<Vec<u8>, String> {
        let ln = self.u16()? as usize;
        Ok(self.take(ln)?.to_vec())
    }
    fn fix<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let s = self.take(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(s);
        Ok(a)
    }
    fn rest(&self) -> usize {
        self.buf.len() - self.p
    }
}

fn parse_card(r: &mut Reader) -> Result<Card, String> {
    let mut c = Card::default();
    c.name = r.str()?;
    c.a = r.u16()?;
    c.b = r.u16()?;
    c.c = r.u16()?;
    for i in 0..5 {
        c.s[i] = r.str()?;
    }
    c.asz = r.u16()?;
    for i in 0..5 {
        let n = r.u8()? as usize;
        c.arr[i] = r.take(n)?.to_vec();
    }
    for i in 0..3 {
        c.dt[i].raw = r.fix()?;
        c.dt[i].place = r.str()?;
    }
    c.sex = r.u8()?;
    for i in 0..6 {
        c.u67[i] = r.u16()?;
    }
    c.style = r.fix()?;
    c.w1 = r.u16()?;
    c.w2 = r.u16()?;

    let n = r.u16()? as usize;
    if n * 4 > r.rest() {
        return Err("L1 zu lang".into());
    }
    for _ in 0..n {
        c.l1.push((r.u16()?, r.u16()?));
    }

    let n = r.u16()? as usize;
    if n * 20 > r.rest() {
        return Err("L2 zu lang".into());
    }
    for _ in 0..n {
        let mut e = Ev::default();
        e.v = r.u16()?;
        e.raw = r.fix()?;
        e.txt = r.str()?;
        e.a = r.u16()?;
        e.b = r.u16()?;
        c.l2.push(e);
    }

    let n = r.u16()? as usize;
    if n * 4 > r.rest() {
        return Err("L3 zu lang".into());
    }
    for _ in 0..n {
        c.l3.push((r.str()?, r.str()?));
    }
    Ok(c)
}

// Datei komplett parsen; Fehlermeldung mit Kartennummer wie im C-Werkzeug.
pub fn parse_file(buf: &[u8]) -> Result<Vec<Card>, String> {
    let mut pos = 0usize;
    let mut out: Vec<Card> = Vec::new();
    while pos < buf.len() {
        if !(pos + 3 < buf.len()
            && buf[pos] == 1
            && buf[pos + 1] == 1
            && buf[pos + 2] == 1
            && buf[pos + 3] == 1)
        {
            let g = |i: usize| *buf.get(i).unwrap_or(&0);
            return Err(format!(
                "Karte {} @{}: Marke 01 01 01 01 fehlt ({:02X} {:02X} {:02X} {:02X})",
                out.len(),
                pos,
                g(pos),
                g(pos + 1),
                g(pos + 2),
                g(pos + 3)
            ));
        }
        let mut r = Reader {
            buf: &buf[pos + 4..],
            p: 0,
        };
        // Reader relativ: rest() bezieht sich auf den Teilabschnitt; parse verschiebt r.p
        let k = match parse_card(&mut r) {
            Ok(k) => k,
            Err(why) => return Err(format!("Karte {} @{}: PARSEFEHLER: {}", out.len(), pos, why)),
        };
        let len = r.p + 4;
        out.push(k);
        pos += len;
    }
    Ok(out)
}

// ---------------- Schreiber (Spiegelbild des Lesers) ----------------

fn w_u16(w: &mut Vec<u8>, v: u16) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_str(w: &mut Vec<u8>, s: &[u8]) {
    w_u16(w, s.len() as u16);
    w.extend_from_slice(s);
}

pub fn write_card(w: &mut Vec<u8>, k: &Card) {
    w.extend_from_slice(&[1, 1, 1, 1]);
    w_str(w, &k.name);
    w_u16(w, k.a);
    w_u16(w, k.b);
    w_u16(w, k.c);
    for i in 0..5 {
        w_str(w, &k.s[i]);
    }
    w_u16(w, k.asz);
    for i in 0..5 {
        w.push(k.arr[i].len() as u8);
        w.extend_from_slice(&k.arr[i]);
    }
    for i in 0..3 {
        w.extend_from_slice(&k.dt[i].raw);
        w_str(w, &k.dt[i].place);
    }
    w.push(k.sex);
    for i in 0..6 {
        w_u16(w, k.u67[i]);
    }
    w.extend_from_slice(&k.style);
    w_u16(w, k.w1);
    w_u16(w, k.w2);
    w_u16(w, k.l1.len() as u16);
    for &(x, y) in &k.l1 {
        w_u16(w, x);
        w_u16(w, y);
    }
    w_u16(w, k.l2.len() as u16);
    for e in &k.l2 {
        w_u16(w, e.v);
        w.extend_from_slice(&e.raw);
        w_str(w, &e.txt);
        w_u16(w, e.a);
        w_u16(w, e.b);
    }
    w_u16(w, k.l3.len() as u16);
    for (x, y) in &k.l3 {
        w_str(w, x);
        w_str(w, y);
    }
}

pub fn encode(cards: &[Card]) -> Vec<u8> {
    let mut w = Vec::new();
    for k in cards {
        write_card(&mut w, k);
    }
    w
}

// ---------------- Kinder-Beziehungen (L1) bearbeiten ----------------
// L1 haelt die Kind-Schluessel B einer Karte als Paare (B, Zweitwert); die
// Bedeutung des Zweitwerts ist offen (in .006 durchgehend 0, in .001 oft 1),
// neue Paare bekommen deshalb eine 0. Der Stammbaum haengt an den Eltern-
// schluesseln w1/w2 der Kindkarte, damit beides nicht auseinanderlaeuft,
// aendert jede Beziehungsbearbeitung IMMER beide Seiten: die Elternkarte
// traegt das Kind in L1 ein, und das Kind erhaelt den Elternschluessel in
// dem Fach, das zum Geschlecht des Elternteils passt (w1 = Vater, w2 = Mutter).

pub fn find_b(cards: &[Card], b: u16) -> Option<usize> {
    cards.iter().position(|k| k.b == b)
}

// Kurzname fuer Statusmeldungen.
pub fn name_zu(cards: &[Card], b: u16) -> String {
    match find_b(cards, b) {
        Some(i) => {
            let n = decode_cp1252(&cards[i].name);
            if n.is_empty() {
                "(ohne Namen)".to_string()
            } else {
                n
            }
        }
        None => "(fehlend)".to_string(),
    }
}

// Kind kind_b zur Elternkarte eltern_b hinzuuegen. Fehler: unbekannter
// Schluessel, Selbstbeziehung, Paar schon vorhanden, oder das passende
// Elternfach zeigt schon auf eine andere Person (dann zuerst dort austragen).
pub fn l1_add(cards: &mut [Card], eltern_b: u16, kind_b: u16) -> Result<String, String> {
    if eltern_b == 0 || kind_b == 0 {
        return Err("Schluessel 0 ist weder Kind noch Elternteil".into());
    }
    if eltern_b == kind_b {
        return Err("Eine Karte kann nicht ihr eigenes Kind sein".into());
    }
    let e = find_b(cards, eltern_b)
        .ok_or_else(|| format!("Elternteil B{} fehlt in diesem Datensatz", eltern_b))?;
    let k = find_b(cards, kind_b)
        .ok_or_else(|| format!("Kind B{} fehlt in diesem Datensatz", kind_b))?;
    let vater = cards[e].sex != b'F';
    let fach = if vater { cards[k].w1 } else { cards[k].w2 };
    if fach != 0 && fach != eltern_b {
        return Err(format!(
            "Kind B{} hat bereits {} B{} — bitte erst diese Beziehung loesen",
            kind_b,
            if vater { "Vater" } else { "Mutter" },
            fach
        ));
    }
    if cards[e].l1.iter().any(|(x, _)| *x == kind_b) {
        return Err(format!("Kind B{} ist bei B{} bereits eingetragen", kind_b, eltern_b));
    }
    cards[e].l1.push((kind_b, 0));
    if vater {
        cards[k].w1 = eltern_b;
    } else {
        cards[k].w2 = eltern_b;
    }
    Ok(format!(
        "Kind B{} {} ist jetzt Kind von B{} {} (Elternschluessel gesetzt)",
        kind_b,
        name_zu(cards, kind_b),
        eltern_b,
        name_zu(cards, eltern_b)
    ))
}

// Beziehung eltern_b -> kind_b austragen: alle passenden L1-Paare der
// Elternkarte fortnehmen und dem Kind den Schluessel aus den Fachern
// loeschen, die auf diese Elternkarte zeigen.
// 30.10. auf Nutzerwunsch: Das KIND darf fehlen. Zeigt eine Elternkarte auf
// eine nie dagewesene oder geloeschte Karte ("B82 (fehlt in diesem
// Datensatz)"), laesst sich der Haengerei-Eintrag jetzt ebenfalls entfernen —
// es wird eben nur in der Elternkarte ausgetragen. FRUEHER (suchbar ueber
// L1-ENTFERNEN-ALT): Fehler "Kind B{} fehlt in diesem Datensatz".
pub fn l1_entfernen(cards: &mut [Card], eltern_b: u16, kind_b: u16) -> Result<String, String> {
    let e = find_b(cards, eltern_b)
        .ok_or_else(|| format!("Elternteil B{} fehlt in diesem Datensatz", eltern_b))?;
    // L1-ENTFERNEN-ALT:
    // let k = find_b(cards, kind_b)
    //     .ok_or_else(|| format!("Kind B{} fehlt in diesem Datensatz", kind_b))?;
    let vorher = cards[e].l1.len();
    cards[e].l1.retain(|(x, _)| *x != kind_b);
    let entfernt = vorher - cards[e].l1.len();
    let mut faecher = 0;
    if let Some(k) = find_b(cards, kind_b) {
        if cards[k].w1 == eltern_b {
            cards[k].w1 = 0;
            faecher += 1;
        }
        if cards[k].w2 == eltern_b {
            cards[k].w2 = 0;
            faecher += 1;
        }
    }
    if entfernt == 0 && faecher == 0 {
        return Err(format!("Keine Beziehung B{} -> B{} gefunden", eltern_b, kind_b));
    }
    Ok(format!(
        "Beziehung B{} {} -> Kind B{} {} geloescht{}",
        eltern_b,
        name_zu(cards, eltern_b),
        kind_b,
        name_zu(cards, kind_b),
        if entfernt > 0 && faecher == 0 && find_b(cards, kind_b).is_none() {
            " (Kindkarte fehlt im Datensatz — nur Elternseite ausgetragen)"
        } else {
            ""
        }
    ))
}

// Nach dem Uebernehmen des Formulars: haben sich die Elternschluessel einer
// Karte geaendert (oder ueberhaupt welche gesetzt), die Kinderlisten der
// Elternkarten mitziehen. Kein Elternteil zu sein nimmt den Eintrag fort,
// Elternteil zu sein setzt ihn, falls er fehlt.
pub fn l1_aktualisieren(cards: &mut [Card], kind_b: u16, vater_b: u16, mutter_b: u16) {
    if kind_b == 0 {
        return;
    }
    for p in cards.iter_mut() {
        if p.b == kind_b || p.b == 0 {
            continue;
        }
        let soll_elternteil = (vater_b != 0 && p.b == vater_b) || (mutter_b != 0 && p.b == mutter_b);
        if soll_elternteil {
            if !p.l1.iter().any(|(x, _)| *x == kind_b) {
                p.l1.push((kind_b, 0));
            }
        } else {
            p.l1.retain(|(x, _)| *x != kind_b);
        }
    }
}

// Beim Loeschen einer Karte bleiben keine Beziehungen als Haengerei stehen
// (30.10. auf Nutzerwunsch; bis dahin blieb alles wie im Original stehen —
// solche Resten zeigen dann "B82 (fehlt in diesem Datensatz)" in der
// Elternkarte und sind mit "entfernen" inzwischen auch nachtraeglich
// tilgbar). In allen VERBLEIBENDEN Karten werden fortgenommen:
// - L1-Paare, deren Kind-Schluessel der geloeschte ist (Elternkarten),
// - Elternfaecher w1/w2, die auf den geloeschten zeigen (Kinderkarten),
// - L2-Ereignisse, deren Ehepartner-Schluessel v der geloeschte ist
//   (Partnerkarten).
// Rueckgabe: deutsche Meldungen (eine je Art) fuer die Statuszeile.
pub fn beziehungen_entfernen(cards: &mut [Card], geloeschter_b: u16) -> Vec<String> {
    let mut m = Vec::new();
    if geloeschter_b == 0 {
        return m;
    }
    let mut l1_stellen = 0usize;
    let mut faecher = 0usize;
    let mut l2_stellen = 0usize;
    for p in cards.iter_mut() {
        let vorher = p.l1.len();
        p.l1.retain(|(x, _)| *x != geloeschter_b);
        l1_stellen += vorher - p.l1.len();
        if p.w1 == geloeschter_b {
            p.w1 = 0;
            faecher += 1;
        }
        if p.w2 == geloeschter_b {
            p.w2 = 0;
            faecher += 1;
        }
        let vorher = p.l2.len();
        p.l2.retain(|e| e.v != geloeschter_b);
        l2_stellen += vorher - p.l2.len();
    }
    if l1_stellen > 0 {
        m.push(format!(
            "Beziehungen zu B{}: {} Kind-Eintrag(e) (L1) aus Elternkarten entfernt.",
            geloeschter_b, l1_stellen
        ));
    }
    if faecher > 0 {
        m.push(format!(
            "Beziehungen zu B{}: {} Elternfach/Faecher (w1/w2) der Kinder geleert.",
            geloeschter_b, faecher
        ));
    }
    if l2_stellen > 0 {
        m.push(format!(
            "Beziehungen zu B{}: {} Ehe-Eintrag(e) (L2) aus Partnerkarten entfernt.",
            geloeschter_b, l2_stellen
        ));
    }
    m
}

// ---------------- Ehepartner-Beziehungen (L2) bearbeiten ----------------
// NAMEN-ALT (30.10.2026): In diesem und den folgenden Kommentaren standen
// echte Familiennamen aus dem Datensatz .006; sie sind bewusst auch im
// auskommentierten Zustand NICHT uebernommen, damit sie nicht auf GitHub
// auftauchen. Suchwort: NAMEN-ALT.
// In den L2-Ereignissen ist das v-Feld der B-Schluessel des Ehepartners
// (bestaetigt an FICHES.006: Karte B2 traegt v=5 auf Karte B5, Karte B5
// traegt v=2 auf Karte B2;
// ledige Karten haben dort keinen Eintrag). Eine Heirat ist eine Paarbeziehung
// und wird deshalb BEIDSEITIG gepflegt: Eintrag in der L2-Liste beider Karten
// (mit v auf den jeweils anderen zeigt). Das Heiratsdatum sitzt im 12-Byte-
// Block (wie die Datumsgruppen, 0xFFFF/Null = unbekannt); neu getraute Paare
// bekommen einen unbekannten Datum-Block und einen leeren Text — das Datum kann
// spaeter gesetzt werden, wichtig ist erst einmal die Paarbindung selbst.

pub fn l2_heirat_add(cards: &mut [Card], person_b: u16, partner_b: u16) -> Result<String, String> {
    if person_b == 0 || partner_b == 0 {
        return Err("Der Schluessel 0 bedeutet 'niemand' — bitte erst eine Person waehlen.".to_string());
    }
    if person_b == partner_b {
        return Err("Niemand kann sich selbst heiraten.".to_string());
    }
    let (ei, ki) = match (find_b(cards, person_b), find_b(cards, partner_b)) {
        (Some(a), Some(b)) => (a, b),
        (None, _) => return Err(format!("Person B{} fehlt in diesem Datensatz.", person_b)),
        (_, None) => return Err(format!("Person B{} fehlt in diesem Datensatz.", partner_b)),
    };
    if cards[ei].l2.iter().any(|e| e.v == partner_b) {
        return Err(format!(
            "B{} und B{} sind bereits als Ehepartner eingetragen.",
            person_b, partner_b
        ));
    }
    // Die a/b-Werte neuer Eintraege: Bedeutung offen; uebernehmen, wenn der
    // Partner schon einen Gegen-Eintrag hat, sonst das in beiden Ehen von
    // FICHES.006 beobachtete Muster (a=0, b=1).
    let (na, nb) = cards[ki]
        .l2
        .iter()
        .find(|e| e.v == person_b)
        .map(|e| (e.a, e.b))
        .unwrap_or((0, 1));
    cards[ei].l2.push(Ev {
        v: partner_b,
        raw: [0; 12],
        txt: Vec::new(),
        a: na,
        b: nb,
    });
    cards[ki].l2.push(Ev {
        v: person_b,
        raw: [0; 12],
        txt: Vec::new(),
        a: na,
        b: nb,
    });
    Ok(format!(
        "B{} {} und B{} {} sind jetzt als Eheleute eingetragen (L2 in beiden Karten).",
        person_b,
        name_zu(cards, person_b),
        partner_b,
        name_zu(cards, partner_b)
    ))
}

// 30.10. auf Nutzerwunsch: Der PARTNER darf fehlen (Haengerei-Ereignis aus
// einer geloeschten Karte). Dann wird nur in der eigenen Karte ausgetragen;
// die Gegenseite entfaellt, weil es sie nicht mehr gibt. FRUEHER (suchbar
// ueber L2-ENTFERNEN-ALT): Fehler "Person B{} fehlt in diesem Datensatz.".
pub fn l2_heirat_entfernen(cards: &mut [Card], person_b: u16, partner_b: u16) -> Result<String, String> {
    if person_b == 0 || partner_b == 0 {
        return Err("Der Schluessel 0 bedeutet 'niemand'.".to_string());
    }
    let ei = match find_b(cards, person_b) {
        Some(a) => a,
        None => return Err(format!("Person B{} fehlt in diesem Datensatz.", person_b)),
    };
    // L2-ENTFERNEN-ALT:
    // let (ei, ki) = match (find_b(cards, person_b), find_b(cards, partner_b)) {
    //     (Some(a), Some(b)) => (a, b),
    //     (None, _) => return Err(format!("Person B{} fehlt in diesem Datensatz.", person_b)),
    //     (_, None) => return Err(format!("Person B{} fehlt in diesem Datensatz.", partner_b)),
    // };
    let ki = find_b(cards, partner_b);
    let vorher = cards[ei].l2.iter().filter(|e| e.v == partner_b).count()
        + ki.map(|k| cards[k].l2.iter().filter(|e| e.v == person_b).count())
            .unwrap_or(0);
    if vorher == 0 {
        return Err(format!("Keine Ehe B{} -> B{} gefunden.", person_b, partner_b));
    }
    cards[ei].l2.retain(|e| e.v != partner_b);
    if let Some(k) = ki {
        cards[k].l2.retain(|e| e.v != person_b);
    }
    Ok(format!(
        "Die Ehe B{} {} <-> B{} {} ist ausgetragen ({}).",
        person_b,
        name_zu(cards, person_b),
        partner_b,
        name_zu(cards, partner_b),
        match ki {
            Some(_) => "L2 in beiden Karten",
            None => "Partnerkarte fehlt im Datensatz — nur eigene Seite ausgetragen",
        }
    ))
}

// ---------------- Datum ----------------

pub fn fmt_date(raw: &[u8; 12]) -> String {
    // Teilwert 0xFFFF heisst in den Originaldaten "unbekannt" (das Formular zeigte
    // sonst "65535.65535.65535"): wie ein leeres Teil zählen.
    let part = |at: usize| {
        let v = u16::from_le_bytes([raw[at], raw[at + 1]]);
        if v == 0xFFFF {
            0
        } else {
            v
        }
    };
    let d = part(2);
    let m = part(4);
    let y = part(6);
    if d == 0 && m == 0 && y == 0 {
        String::new() // Datum unbekannt -> leeres Feld statt "-"
    } else if d == 0 || m == 0 {
        format!("{:04}", y)
    } else {
        format!("{:02}.{:02}.{:04}", d, m, y)
    }
}

// wie set_date in racfiche.c: "TT.MM.JJJJ", "JJJJ" oder "-" (loeschen).
// q-Wort (raw+0) und x1/x2 (raw+8/+10, Tabellenzeiger) bleiben erhalten.
pub fn set_date(raw: &mut [u8; 12], v: &str) -> bool {
    let v = v.trim();
    let (d, m, y): (i32, i32, i32);
    if v == "-" {
        d = 0;
        m = 0;
        y = 0;
    } else {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() == 3 {
            d = match parts[0].parse::<i32>() { Ok(x) => x, Err(_) => return false };
            m = match parts[1].parse::<i32>() { Ok(x) => x, Err(_) => return false };
            y = match parts[2].parse::<i32>() { Ok(x) => x, Err(_) => return false };
            if d < 0 || d > 31 || m < 1 || m > 12 {
                return false;
            }
        } else if parts.len() == 1 {
            y = match parts[0].parse::<i32>() { Ok(x) => x, Err(_) => return false };
            d = 0;
            m = 0;
        } else {
            return false;
        }
    }
    if y < 0 || y > 9999 {
        return false;
    }
    raw[2] = d as u8;
    raw[3] = 0;
    raw[4] = m as u8;
    raw[5] = 0;
    raw[6] = (y & 0xFF) as u8;
    raw[7] = (y >> 8) as u8;
    true
}

// ---------------- Datensatz-Kopf (DOSSIER) ----------------

fn head_str(b: &[u8], off: usize) -> String {
    let mut e = off;
    while e < off + 30 && e < b.len() && b[e] != 0 {
        e += 1;
    }
    decode_cp1252(&b[off..e])
}

#[derive(Clone, Debug)]
pub struct DatasetInfo {
    pub ext: String,        // ".001"
    pub file_name: String,  // "FICHES.001"
    pub is_sub: bool,       // FICHESS: Unterdatei, DOSSIER-Patch gilt nicht
    pub fiches: PathBuf,
    pub tree: String,
    pub owner: String,
    pub dossier_count: Option<u16>,
}

// Ordner nach Datenspaetzen durchsuchen: FICHES.* / FICHESS.* plus DOSSIER-Kopf.
pub fn scan_folder(dir: &Path) -> Result<Vec<DatasetInfo>, String> {
    let rd = std::fs::read_dir(dir)
        .map_err(|e| format!("Ordner nicht lesbar: {} ({})", dir.display(), e))?;
    let mut v: Vec<DatasetInfo> = Vec::new();
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        let dot = match name.rfind('.') {
            Some(d) => d,
            None => continue,
        };
        let base = &name[..dot];
        let ub = base.to_ascii_uppercase();
        if ub != "FICHES" && ub != "FICHESS" {
            continue;
        }
        let ext = name[dot..].to_string();
        let mut info = DatasetInfo {
            ext: ext.clone(),
            file_name: name.clone(),
            is_sub: ub == "FICHESS",
            fiches: dir.join(&name),
            tree: String::new(),
            owner: String::new(),
            dossier_count: None,
        };
        if let Ok(bytes) = std::fs::read(&dir.join(format!("DOSSIER{}", ext))) {
            if bytes.len() == 456 {
                info.tree = head_str(&bytes, 2);
                info.owner = head_str(&bytes, 82);
                info.dossier_count = Some(u16::from_le_bytes([bytes[0x1B0], bytes[0x1B1]]));
            }
        }
        v.push(info);
    }
    v.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(v)
}

// ---------------- Speichern: backup -> schreiben -> nachpruefen -> DOSSIER ----------------

pub struct SaveReport {
    pub backup: PathBuf,
    pub bytes: usize,
    pub cards: usize,
    pub dossier: Option<String>,
}

// Rotation .bak, .bak~1 .. .bak~9: erster freier Name (wie ensure_backup).
pub fn ensure_backup(path: &Path) -> Result<PathBuf, String> {
    for i in 0..10 {
        let bak = if i == 0 {
            format!("{}.bak", path.display())
        } else {
            format!("{}.bak~{}", path.display(), i)
        };
        let p = PathBuf::from(&bak);
        if p.exists() {
            continue; // Name belegt: naechsten nehmen
        }
        std::fs::copy(path, &p)
            .map_err(|e| format!("Sicherungskopie FEHLGESCHLAGEN: {} ({})", bak, e))?;
        return Ok(p);
    }
    Err("Sicherungskopie FEHLGESCHLAGEN: zu viele .bak-Kopien".into())
}

// DOSSIER.NNN derselben Datenbank: Personenzaehler u16 @0x1B0 setzen.
// Nur bei sicherer Erkennung (Name genau FICHES.<ext>, Groesse 456) — FICHESS ausgenommen.
fn patch_dossier(fiches: &Path, newcount: u16) -> Option<String> {
    let fname = match fiches.file_name().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => return None,
    };
    let dot = match fname.rfind('.') {
        Some(d) => d,
        None => return None,
    };
    if dot != 6 || !fname[..6].eq_ignore_ascii_case("FICHES") {
        return None;
    }
    let ext = &fname[dot..];
    let doss = fiches.with_file_name(format!("DOSSIER{}", ext));
    let bytes = match std::fs::read(&doss) {
        Ok(b) => b,
        Err(_) => {
            return Some(format!(
                "DOSSIER nicht gefunden ({}) — Zaehler nicht angepasst",
                doss.display()
            ))
        }
    };
    if bytes.len() != 456 {
        return Some(format!(
            "DOSSIER ueberraschend: {} ist {} Byte (erwartet 456) — unveraendert",
            doss.display(),
            bytes.len()
        ));
    }
    if let Err(e) = ensure_backup(&doss) {
        return Some(e);
    }
    let old = u16::from_le_bytes([bytes[0x1B0], bytes[0x1B1]]);
    let mut data = bytes;
    data[0x1B0] = (newcount & 0xFF) as u8;
    data[0x1B1] = (newcount >> 8) as u8;
    if let Err(e) = std::fs::write(&doss, &data) {
        return Some(format!("DOSSIER-Schreibfehler @0x1B0: {}", e));
    }
    Some(format!(
        "DOSSIER {}: Personenzaehler @0x1B0 {} -> {} (Cursor @0x1C6 unberuehrt)",
        doss.display(),
        old,
        newcount
    ))
}

// Kompletter Sicherer Speichern-Vorgang wie im C-Werkzeug:
// 1) backup, 2) Datei neu schreiben, 3) zuruecklesen, neu parsen, erneut kodieren,
//    Byte-Gleichheit verlangen, 4) DOSSIER-Zaehler anpassen.
pub fn save_cards(path: &Path, cards: &[Card]) -> Result<SaveReport, String> {
    let bytes = encode(cards);
    let backup = ensure_backup(path)?;
    std::fs::write(path, &bytes).map_err(|e| format!("Schreiben fehlgeschlagen: {} ({})", path.display(), e))?;
    let back = std::fs::read(path).map_err(|e| format!("Nachpruefung: Lesen fehlgeschlagen ({})", e))?;
    let reparsed = parse_file(&back).map_err(|e| format!("Nachpruefung: {}", e))?;
    if reparsed.len() != cards.len() {
        return Err(format!(
            "Nachpruefung: Datei enthaelt {} Karten, erwartet {}",
            reparsed.len(),
            cards.len()
        ));
    }
    if encode(&reparsed) != bytes {
        return Err("Nachpruefung: Rueckschreiben weicht von der Datei ab".into());
    }
    let dossier = patch_dossier(path, cards.len() as u16);
    Ok(SaveReport {
        backup,
        bytes: bytes.len(),
        cards: cards.len(),
        dossier,
    })
}

// ---------------- Ereignistafeln (NAISS/BAPTEMES/MARIAGES/BENEDICT/DECES/SEPULTUR) ----------------
// Layout (siehe raclook.c -event, BESTAETIGT an Originaldateien):
//   fester Basisteil 0..50267: Kopfworter @+262 (Zaehler), @+264 (Satzgroesse), @+266 (Kapazitaet),
//   IndexA u16[10000] @268, IndexB u16[10000] @20268, Statusbyte[10000] @40268.
//   Satze ab 50268, feste Groesse = Kopf @+264 (variiert je Datei!).
// ACHTUNG: nur lesend — der Schreiber ist nicht portiert, die Layouts von
// BAPTEMES/BENEDICT/SEPULTUR sind noch nicht kartiert (deshalb Stringlauf-Ansicht).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EvKind {
    Nais,
    Bapt,
    Mari,
    Deces,
    Bene,
    Sepu,
    Fremd,
}

pub fn ev_title(kind: EvKind) -> &'static str {
    match kind {
        EvKind::Nais => "Geburt",
        EvKind::Bapt => "Taufe",
        EvKind::Mari => "Heirat",
        EvKind::Deces => "Tod",
        EvKind::Bene => "Benediktion",
        EvKind::Sepu => "Begraebnis",
        EvKind::Fremd => "Unbekannt",
    }
}

// (Dateiname ohne Erweiterung, Tafeltyp, Reiterueberschrift) — Anordnung = Reiterreihenfolge.
pub const EV_TABLES: [(&str, EvKind, &str); 6] = [
    ("NAISS", EvKind::Nais, "Geburt"),
    ("BAPTEMES", EvKind::Bapt, "Taufe"),
    ("MARIAGES", EvKind::Mari, "Heirat"),
    ("DECES", EvKind::Deces, "Tod"),
    ("BENEDICT", EvKind::Bene, "Benediktion"),
    ("SEPULTUR", EvKind::Sepu, "Begraebnis"),
];

#[derive(Clone, Debug)]
pub struct EvRec {
    pub idx: usize,
    pub off: usize,
    pub used: bool, // Satzanfang nicht FF FF (freie Saetze stehen in der Freiliste)
    pub ia: u16,
    pub ib: u16,
    pub status: u8,
    pub key: Option<u16>,  // NAISS @+122 bzw. Ehemann-Schluessel MARIAGES @+332
    pub key2: Option<u16>, // Ehefrau-Schluessel MARIAGES @+1108
    pub fields: Vec<(&'static str, String)>, // kartierte Felder
    pub strings: Vec<(usize, String)>,       // druckbare Textlaeufen, Offset relativ zum Satz
}

#[derive(Clone, Debug)]
pub struct EvTable {
    pub kind: EvKind,
    // Die Reiterueberschrift: die Knopfe, die sie lesen, sind seit 28.10. per
    // Nutzerwunsch auskommentiert (Marker REITER-AUS-KOMMENTAR in main.rs).
    #[allow(dead_code)]
    pub title: &'static str,
    pub path: PathBuf,
    pub recsize: u16,
    pub capacity: u16,
    pub counter: u16,
    pub file_len: usize,
    pub model_fits: bool,
    pub n: usize,
    pub recs: Vec<EvRec>,
}

impl EvTable {
    pub fn used_count(&self) -> usize {
        self.recs.iter().filter(|r| r.used).count()
    }
}

fn rd16_at(b: &[u8], i: usize) -> u16 {
    if i + 2 > b.len() {
        return 0;
    }
    u16::from_le_bytes([b[i], b[i + 1]])
}

// Festes Feld: cp1252 bis NUL, maximal width Byte.
fn fixed_str(b: &[u8], off: usize, width: usize) -> String {
    if off >= b.len() {
        return String::new();
    }
    let end = (off + width).min(b.len());
    let mut e = off;
    while e < end && b[e] != 0 {
        e += 1;
    }
    decode_cp1252(&b[off..e]).trim().to_string()
}

// Druckbare ASCII-Laeufe (Laenge >= min) — Hilfssicht fuer unvollstaendig kartierte Tafeln.
fn printable_runs(b: &[u8], min: usize, maxhits: usize) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut i = 0usize;
    while i < b.len() && out.len() < maxhits {
        if b[i] >= 0x20 && b[i] < 0x7F {
            let s = i;
            while i < b.len() && b[i] >= 0x20 && b[i] < 0x7F {
                i += 1;
            }
            if i - s >= min {
                let t = String::from_utf8_lossy(&b[s..i]).trim().to_string();
                if !t.is_empty() {
                    out.push((s, t));
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

fn ev_kind_from_name(name: &str) -> EvKind {
    let up = name.to_ascii_uppercase();
    for (base, kind, _) in EV_TABLES.iter() {
        if up.contains(base) {
            return *kind;
        }
    }
    EvKind::Fremd
}

// Eine Ereignistafel lesen; nur lesend, nie zurueckschreiben.
pub fn parse_event_file(path: &Path) -> Result<EvTable, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Lesen fehlgeschlagen: {} ({})", path.display(), e))?;
    let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let kind = ev_kind_from_name(&name);
    let title = ev_title(kind);
    if bytes.len() < 50268 {
        return Err(format!("{}: nur {} Byte — zu klein fuer Ereignistabelle", name, bytes.len()));
    }
    let counter = rd16_at(&bytes, 262);
    let recsize = rd16_at(&bytes, 264);
    let capacity = rd16_at(&bytes, 266);
    let rest = bytes.len() - 50268;
    let model_fits = recsize > 0 && rest % recsize as usize == 0;
    let n = if model_fits { rest / recsize as usize } else { 0 };
    let mut recs: Vec<EvRec> = Vec::new();
    for i in 0..n {
        let off = 50268 + i * recsize as usize;
        let rec = &bytes[off..off + recsize as usize];
        let used = !(rec[0] == 0xFF && rec[1] == 0xFF);
        let ia = rd16_at(&bytes, 268 + i * 2);
        let ib = rd16_at(&bytes, 20268 + i * 2);
        let status = bytes.get(40268 + i).copied().unwrap_or(0);
        let mut key: Option<u16> = None;
        let mut key2: Option<u16> = None;
        let mut fields: Vec<(&'static str, String)> = Vec::new();
        if used {
            match kind {
                EvKind::Nais => {
                    key = Some(rd16_at(rec, 122));
                    let sex = match rec.get(178) {
                        Some(b'H') => Some("H"),
                        Some(b'F') => Some("F"),
                        _ => None,
                    };
                    let push = |fields: &mut Vec<(&'static str, String)>, lbl: &'static str, v: String| {
                        if !v.is_empty() {
                            fields.push((lbl, v));
                        }
                    };
                    push(&mut fields, "Geschlecht", sex.unwrap_or("").to_string());
                    push(&mut fields, "Nachname", fixed_str(rec, 179, 30));
                    push(&mut fields, "Vorname", fixed_str(rec, 209, 30));
                    push(&mut fields, "Geburtsort", fixed_str(rec, 321, 30));
                    let v = format!("{} {}", fixed_str(rec, 431, 30), fixed_str(rec, 461, 30));
                    push(&mut fields, "Vater", v.trim().to_string());
                    let m = format!("{} {}", fixed_str(rec, 817, 30), fixed_str(rec, 847, 30));
                    push(&mut fields, "Mutter", m.trim().to_string());
                }
                EvKind::Mari => {
                    key = Some(rd16_at(rec, 332));
                    key2 = Some(rd16_at(rec, 1108));
                    let e = format!("{} {}", fixed_str(rec, 202, 30), fixed_str(rec, 232, 30));
                    if !e.trim().is_empty() {
                        fields.push(("Ehemann", e.trim().to_string()));
                    }
                    let f = format!("{} {}", fixed_str(rec, 978, 30), fixed_str(rec, 1008, 30));
                    if !f.trim().is_empty() {
                        fields.push(("Ehefrau", f.trim().to_string()));
                    }
                }
                EvKind::Deces => {
                    // nur diese drei Felder BESTAETIGT (Name/nachname?), Rest noch offen
                    for (lbl, off2) in [("Feld+430", 430usize), ("Feld+450", 450), ("Feld+470", 470)] {
                        let v = fixed_str(rec, off2, 20);
                        if !v.is_empty() {
                            fields.push((lbl, v));
                        }
                    }
                }
                _ => {}
            }
        }
        let strings = if used { printable_runs(rec, 3, 48) } else { Vec::new() };
        recs.push(EvRec { idx: i, off, used, ia, ib, status, key, key2, fields, strings });
    }
    Ok(EvTable {
        kind,
        title,
        path: path.to_path_buf(),
        recsize,
        capacity,
        counter,
        file_len: bytes.len(),
        model_fits,
        n,
        recs,
    })
}

// Geschwisterdateien derselben Erweiterung neben FICHES.NNN einsammeln.
// Liefert (Tafeln, Probleme) — fehlende Dateien sind normal (nicht jede Datenbank fuehrt alle Tafeln).
pub fn load_event_tables(dir: &Path, ext: &str) -> (Vec<EvTable>, Vec<String>) {
    let mut tabs: Vec<EvTable> = Vec::new();
    let mut probs: Vec<String> = Vec::new();
    let ext = if ext.starts_with('.') { ext.to_string() } else { format!(".{}", ext) };
    for (base, _kind, title) in EV_TABLES.iter() {
        let p = dir.join(format!("{}{}", base, ext));
        if !p.exists() {
            continue;
        }
        match parse_event_file(&p) {
            Ok(t) => tabs.push(t),
            Err(e) => probs.push(format!("Tafel {}: {}", title, e)),
        }
    }
    (tabs, probs)
}

// ---------------- Tests: Regression gegen die Original-Kopien ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_pristine_copies() {
        let dir = Path::new("../racines-port/data");
        let files: [(&str, usize); 7] = [
            ("FICHES.001", 294),
            ("FICHES.002", 69),
            ("FICHES.003", 151),
            ("FICHES.004", 456),
            ("FICHES.005", 299),
            ("FICHES.006", 5),
            ("FICHESS.006", 4),
        ];
        for (name, want) in files {
            let p = dir.join(name);
            let bytes = std::fs::read(&p).expect("Kopie fehlt — racines-port/data vorher kopieren");
            let cards = parse_file(&bytes).unwrap_or_else(|e| panic!("{}: {}", name, e));
            assert_eq!(cards.len(), want, "{} Kartenanzahl", name);
            assert_eq!(encode(&cards), bytes, "{} Roundtrip nicht byte-identisch", name);
        }
    }

    #[test]
    fn cp1252_roundtrip_alle_bytes() {
        let all: Vec<u8> = (0..=255u8).collect();
        let s = decode_cp1252(&all);
        assert_eq!(encode_cp1252(&s), all);
        // deutsche Umlaute
        assert_eq!(encode_cp1252("Äöüß"), vec![0xC4, 0xF6, 0xFC, 0xDF]);
        assert_eq!(decode_cp1252(&[0xC4, 0xF6, 0xFC, 0xDF]), "Äöüß");
    }

    // ----- Kinder-Beziehungen (L1) bearbeiten: es arbeiten IMMER beide Seiten -----
    fn drei_personen() -> Vec<Card> {
        let mut vater = Card::new(1);
        vater.name = b"Vati".to_vec();
        vater.sex = b'H';
        let mut mutter = Card::new(2);
        mutter.name = b"Mutti".to_vec();
        mutter.sex = b'F';
        let mut zwerg = Card::new(3);
        zwerg.name = b"Zwerg".to_vec();
        zwerg.sex = b'H';
        vec![vater, mutter, zwerg]
    }

    #[test]
    fn l1_add_und_entfernen_halten_beide_seiten_konsistent() {
        let mut c = drei_personen();
        assert!(l1_add(&mut c, 1, 3).is_ok());
        assert_eq!(c[0].l1, vec![(3, 0)], "Vaterkarte traegt das Kind in L1");
        assert_eq!(c[2].w1, 1, "Kind erhaelt den Vater-Schluessel");
        assert_eq!(c[2].w2, 0, "das Mutter-Fach bleibt unberuehrt");
        assert!(l1_add(&mut c, 2, 3).is_ok());
        assert_eq!(c[1].l1, vec![(3, 0)]);
        assert_eq!(c[2].w2, 2, "Kind erhaelt den Mutter-Schluessel");
        assert!(l1_add(&mut c, 1, 3).is_err(), "Paar bereits vorhanden -> Fehler");
        let mut onkel = Card::new(4);
        onkel.name = b"Onkel".to_vec();
        onkel.sex = b'H';
        c.push(onkel);
        assert!(l1_add(&mut c, 4, 3).is_err(), "Vater-Fach schon belegt -> Fehler");
        assert!(l1_add(&mut c, 3, 3).is_err(), "Selbstbeziehung -> Fehler");
        assert!(l1_add(&mut c, 0, 3).is_err(), "Schluessel 0 -> Fehler");
        assert!(l1_add(&mut c, 9, 3).is_err(), "unbekannte Elternkarte -> Fehler");
        assert_eq!(c[0].l1, vec![(3, 0)], "kein Fehlerfall hat geaendert");
        // Austragen: L1-Eintrag fort, Fach geleert, die andere Seite bleibt heil.
        assert!(l1_entfernen(&mut c, 2, 3).is_ok());
        assert!(c[1].l1.is_empty());
        assert_eq!(c[2].w2, 0);
        assert_eq!(c[2].w1, 1, "Vaterseite bleibt");
        assert_eq!(c[0].l1, vec![(3, 0)], "Vater-L1 bleibt");
        assert!(l1_entfernen(&mut c, 2, 3).is_err(), "nicht vorhandene Beziehung -> Fehler");
        // Und das Ergebnis ist wieder sauber encodierbar/parsbar:
        let again = parse_file(&encode(&c)).expect("Rueckpfaden");
        assert_eq!(again[0].l1, vec![(3, 0)]);
        assert_eq!(again[2].w1, 1);
    }

    #[test]
    fn l1_aktualisieren_sichert_den_elternwechsel() {
        let mut c = drei_personen();
        let mut onkel = Card::new(4);
        onkel.name = b"Onkel".to_vec();
        onkel.sex = b'H';
        c.push(onkel);
        l1_add(&mut c, 1, 3).expect("Kind eintragen");
        // Formular der Zwergkarte: Vater auf B4 geaendert, Mutter auf B2 gesetzt.
        c[2].w1 = 4;
        c[2].w2 = 2;
        l1_aktualisieren(&mut c, 3, 4, 2);
        assert!(c[0].l1.is_empty(), "der alte Vater verliert den Eintrag");
        assert_eq!(c[3].l1, vec![(3, 0)], "der neue Vater bekommt ihn");
        assert_eq!(c[1].l1, vec![(3, 0)], "die Mutter bekommt ihn, obwohl sie ihn nie hatte");
        // Beide Eltern auf 0 (Formular "— ohne —"): alle Kinderlisten werden sauber.
        l1_aktualisieren(&mut c, 3, 0, 0);
        assert!(c.iter().all(|k| k.l1.iter().all(|(x, _)| *x != 3)));
    }

    // ----- Ehepartner-Beziehungen (L2): auch hier arbeiten beide Karten zusammen -----
    #[test]
    fn l2_heirat_add_und_entfernen_wirken_beidseitig() {
        let mut c = drei_personen();
        assert!(l2_heirat_add(&mut c, 1, 2).is_ok());
        assert_eq!(
            c[0].l2.iter().map(|e| e.v).collect::<Vec<_>>(),
            vec![2],
            "die Vati-Karte traegt die Gattin in L2"
        );
        assert_eq!(
            c[1].l2.iter().map(|e| e.v).collect::<Vec<_>>(),
            vec![1],
            "die Mutti-Karte traegt den Gatten in L2"
        );
        assert_eq!((c[0].l2[0].a, c[0].l2[0].b), (0, 1), "neue Eintraege folgen dem Muster aus .006");
        assert_eq!(fmt_date(&c[0].l2[0].raw), "", "das Heiratsdatum bleibt zunachst unbekannt");
        assert!(l2_heirat_add(&mut c, 1, 2).is_err(), "dieselbe Ehe noch einmal -> Fehler");
        assert!(l2_heirat_add(&mut c, 1, 1).is_err(), "Selbstheirat -> Fehler");
        assert!(l2_heirat_add(&mut c, 0, 2).is_err(), "Schluessel 0 -> Fehler");
        assert!(l2_heirat_add(&mut c, 7, 2).is_err(), "unbekannte Person -> Fehler");
        assert_eq!(c[0].l2.len(), 1, "kein Fehlerfall hat geaendert");
        // Austragen: beide Seiten leer, dritte Karte bleibt unbeteiligt.
        assert!(l2_heirat_entfernen(&mut c, 1, 2).is_ok());
        assert!(c[0].l2.is_empty() && c[1].l2.is_empty());
        assert!(c[2].l2.is_empty());
        assert!(l2_heirat_entfernen(&mut c, 1, 2).is_err(), "nicht vorhandene Ehe -> Fehler");
        // Ergebnis wieder sauber encodierbar/parsbar:
        let again = parse_file(&encode(&c)).expect("Rueckpfaden");
        assert!(again.iter().all(|k| k.l2.is_empty()));
    }

    #[test]
    fn l2_heirat_mehrere_ehen_hinters_einander_sind_moeglich() {
        let mut c = drei_personen();
        l2_heirat_add(&mut c, 1, 2).expect("erste Ehe");
        l2_heirat_entfernen(&mut c, 1, 2).expect("erste Ehe wieder fort");
        l2_heirat_add(&mut c, 1, 3).expect("Vati und Zwerg");
        l2_heirat_add(&mut c, 2, 3).expect("auch Mutti und Zwerg");
        assert_eq!(c[2].l2.iter().map(|e| e.v).collect::<Vec<_>>(), vec![1, 2]);
        // Austragen ueber eine Person erwischt nur die gewuenschte Verbindung.
        l2_heirat_entfernen(&mut c, 3, 1).expect("Ehe Zwerg-Vati fort");
        assert_eq!(c[2].l2.iter().map(|e| e.v).collect::<Vec<_>>(), vec![2]);
        assert_eq!(c[0].l2.iter().map(|e| e.v).collect::<Vec<_>>(), vec![] as Vec<u16>);
    }

    #[test]
    fn datum_wie_set_date() {
        let mut r = [0u8; 12];
        assert!(set_date(&mut r, "23.05.1799"));
        assert_eq!(fmt_date(&r), "23.05.1799");
        assert!(set_date(&mut r, "1799"));
        assert_eq!(fmt_date(&r), "1799");
        assert!(set_date(&mut r, "-"));
        assert_eq!(fmt_date(&r), "");
        assert!(!set_date(&mut r, "23.13.1799"));
        assert!(!set_date(&mut r, "x"));
        // q-Wort und x1/x2 bleiben erhalten
        let mut r2 = [0u8; 12];
        r2[0] = 0xAB;
        r2[1] = 0xCD;
        r2[8] = 0x11;
        r2[10] = 0x22;
        assert!(set_date(&mut r2, "01.02.2000"));
        assert_eq!((r2[0], r2[1], r2[8], r2[10]), (0xAB, 0xCD, 0x11, 0x22));
    }

    #[test]
    fn datum_65535_unbekannt_ist_leer() {
        // Originaldaten nutzen 0xFFFF pro Teil als "unbekannt".
        let mut r = [0u8; 12];
        r[2] = 0xFF;
        r[3] = 0xFF;
        r[4] = 0xFF;
        r[5] = 0xFF;
        r[6] = 0xFF;
        r[7] = 0xFF;
        assert_eq!(fmt_date(&r), "");
        // Nur Jahr bekannt (Tag/Monat 0xFFFF): Jahreszahl allein.
        r[6] = 0x07;
        r[7] = 0x07; // 1799 little-endian
        assert_eq!(fmt_date(&r), "1799");
        // Vollstaendiges Datum trotz 0xFFFF in q-Wort/Zeigern bleibt normal lesbar.
        r[0] = 0xFF;
        r[1] = 0xFF;
        r[8] = 0xFF;
        r[9] = 0xFF;
        r[10] = 0xFF;
        r[11] = 0xFF;
        r[2] = 23;
        r[3] = 0;
        r[4] = 5;
        r[5] = 0;
        assert_eq!(fmt_date(&r), "23.05.1799");
    }

    #[test]
    fn baumnamen_originales_layout() {
        let dir = Path::new("../racines-port/data");
        let v = scan_folder(dir).expect("Kopien-Ordner lesbar");
        let names: Vec<String> = v.iter().map(|d| d.tree.clone()).collect();
        assert!(names.contains(&"Royal Families of England".to_string()));
        assert!(names.iter().any(|n| n == "Mein Stammbaum"));
    }

    // End-to-End-Schreibtest ausschliesslich auf Arbeitskopien in tmp_wtest.
    // Prueft: Backup vor Schreibvorgang, Rotation .bak~1, DOSSIER-Zaehler nur bei
    // exakt FICHES-Name, Cursor @0x1C6 unberuehrt, FICHESS bleibt unangetastet.
    #[test]
    fn speichern_mit_backup_und_dossier() {
        let src = Path::new("../racines-port/data");
        let tmp = Path::new("tmp_wtest");
        if tmp.exists() {
            std::fs::remove_dir_all(tmp).expect("alter tmp_wtest loeschbar");
        }
        std::fs::create_dir_all(tmp).unwrap();
        for f in ["FICHES.006", "DOSSIER.006", "FICHESS.006"] {
            std::fs::copy(src.join(f), tmp.join(f)).expect("Kopie in tmp_wtest");
        }
        let fches = tmp.join("FICHES.006");
        let doss = tmp.join("DOSSIER.006");
        let alt_bytes = std::fs::read(&fches).unwrap();
        let doss_alt = std::fs::read(&doss).unwrap();
        assert_eq!(doss_alt.len(), 456);
        let cursor_alt = [doss_alt[0x1C6], doss_alt[0x1C7]];

        let mut cards = parse_file(&alt_bytes).unwrap();
        assert_eq!(cards.len(), 5);
        // umbenennen (mit Umlauten -> cp1252-Rundreise), eine Karte loeschen, eine neue anlegen
        cards[0].name = encode_cp1252("Müller, ÄöÜ");
        cards.pop();
        let nb = cards.iter().map(|c| c.b).max().unwrap() + 1;
        cards.push(Card::new(nb));
        let rep = save_cards(&fches, &cards).expect("Speichern erfolgreich");

        assert!(rep.backup.ends_with("FICHES.006.bak"));
        assert_eq!(std::fs::read(&rep.backup).unwrap(), alt_bytes, "Backup = alter Dateiinhalt");
        let neu = std::fs::read(&fches).unwrap();
        assert_eq!(parse_file(&neu).unwrap().len(), cards.len());
        assert_eq!(decode_cp1252(&parse_file(&neu).unwrap()[0].name), "Müller, ÄöÜ");

        // DOSSIER: nur @0x1B0/@0x1B1 (Zaehler) geaendert, Rest — auch Cursor — identisch
        let doss_neu = std::fs::read(&doss).unwrap();
        assert_eq!(doss_neu.len(), 456);
        let counter = u16::from_le_bytes([doss_neu[0x1B0], doss_neu[0x1B1]]);
        assert_eq!(counter as usize, cards.len(), "DOSSIER-Zaehler = Kartenanzahl");
        assert_eq!([doss_neu[0x1C6], doss_neu[0x1C7]], cursor_alt, "Cursor @0x1C6 unberuehrt");
        let mut erwart = doss_alt.clone();
        erwart[0x1B0] = doss_neu[0x1B0];
        erwart[0x1B1] = doss_neu[0x1B1];
        assert_eq!(doss_neu, erwart, "DOSSIER sonst bitgenau gleich");
        assert!(tmp.join("DOSSIER.006.bak").exists(), "DOSSIER ebenfalls gesichert");
        assert!(rep.dossier.unwrap().contains("0x1B0"));

        // zweiter Durchlauf: Backup-Rotation auf .bak~1
        let rep2 = save_cards(&fches, &cards).expect("zweites Speichern");
        assert!(rep2.backup.ends_with("FICHES.006.bak~1"), "Rotation: {:?}", rep2.backup);

        // FICHESS-Unterdatei: DOSSIER wird bewusst NICHT angefasst
        let fichess = tmp.join("FICHESS.006");
        let before = std::fs::read(&doss).unwrap();
        let mut sub = parse_file(&std::fs::read(&fichess).unwrap()).unwrap();
        sub.pop();
        let rep3 = save_cards(&fichess, &sub).expect("FICHESS speichern");
        assert!(rep3.dossier.is_none(), "FICHESS loest keinen DOSSIER-Patch aus");
        assert_eq!(std::fs::read(&doss).unwrap(), before, "DOSSIER bei FICHESS-Speichern unveraendert");

        std::fs::remove_dir_all(tmp).ok();
    }

    // Ereignistafeln auf den Kopien in racines-port/data.
    #[test]
    fn ereignistafeln_kopien() {
        let dir = Path::new("../racines-port/data");

        // .006: Geburt 2 aktive Saetze (Karten B=2 und B=5), Heirat 1 Satz.
        // NAMEN-ALT: in der alten Fassung standen hier die echten Namen der
        // beiden Personen (Suchwort NAMEN-ALT).
        let (tabs, probs) = load_event_tables(dir, ".006");
        assert!(probs.is_empty(), "Probleme: {:?}", probs);
        let geb = tabs.iter().find(|t| t.kind == EvKind::Nais).expect("NAISS.006 vorhanden");
        assert!(geb.model_fits, "Modellpasse NAISS.006");
        assert_eq!(geb.recsize, 3593);
        assert_eq!(geb.n, 2);
        assert_eq!(geb.used_count(), 2);
        // NAMEN-ALT (30.10.2026): Der Geburtssatz wurde frueher ueber den
        // echten Familiennamen gesucht:
        //   let d = geb.recs.iter()
        //       .find(|r| r.fields.iter().any(|(_, v)| v == "<Familienname>"))
        //       .expect("Satz <Familienname>");
        // und Geburtsort/Vater/Mutter gegen echte Namen und einen echten
        // Geburtsort geprueft. Die Namen bleiben dem Repo bewusst fern —
        // gesucht wird jetzt neutral ueber den B-Schluessel, geprueft nur
        // noch, dass die Felder vorhanden und gefüllt sind.
        let d = geb
            .recs
            .iter()
            .find(|r| r.key == Some(2))
            .expect("Geburtssatz mit B-Schluessel 2");
        assert_eq!(d.key, Some(2));
        assert_eq!(d.ib, 2);
        assert!(d.fields.iter().any(|(l, v)| *l == "Geburtsort" && !v.is_empty()));
        assert!(d.fields.iter().any(|(l, v)| *l == "Vater" && !v.is_empty()));
        assert!(d.fields.iter().any(|(l, v)| *l == "Mutter" && !v.is_empty()));
        let hei = tabs.iter().find(|t| t.kind == EvKind::Mari).expect("MARIAGES.006 vorhanden");
        assert_eq!(hei.n, 1);
        let h = hei.recs.iter().find(|r| r.used).unwrap();
        assert_eq!(h.key, Some(2));
        assert_eq!(h.key2, Some(5));
        // NAMEN-ALT: frueher "Ehemann ... <Vorname Mann>" bzw. "Ehefrau ...
        // <Vorname Frau>" mit echten Namen; jetzt nur vorhanden-und-gefuellt.
        assert!(h.fields.iter().any(|(l, v)| *l == "Ehemann" && !v.is_empty()));
        assert!(h.fields.iter().any(|(l, v)| *l == "Ehefrau" && !v.is_empty()));

        // .001: leere Taufen/Benediktion/Begraebnis/Tod — parsielbar, keine aktiven Saetze.
        let (t1, probs1) = load_event_tables(dir, ".001");
        assert!(probs1.is_empty(), "Probleme .001: {:?}", probs1);
        for kind in [EvKind::Bapt, EvKind::Bene, EvKind::Sepu, EvKind::Deces] {
            let t = t1.iter().find(|t| t.kind == kind);
            if let Some(t) = t {
                assert_eq!(t.used_count(), 0, "{:?} .001 sollte leer sein", kind);
            }
        }
    }
}
