// Drucken des Mitgliederblatts (Datei-Menue -> "Drucken...").
// Alle Mitglieder des gerade geladenen Datensatzes, je 8 Personen auf einer
// DIN-A4-Seite: 4 Reihen zu 2 Spalten (Rundungswunsch des Nutzers vom 28.10.,
// zuvor 6 je Seite in 2 Reihen zu 3 Spalten). Als Untergrund dienen die Windows-
// Druckfunktionen (winspool/GDI) — ohne zusaetzliche Crates, nur die noetigen
// Funktionserklaerungen. Die Druckeroberflaeche (Auswahlfenster) sitzt in
// main.rs; hier stehen die Aufteilung und das eigentliche Druckstueck.
// Das Papier ist die Voreinstellung des gewaehlten Druckers — bei deutschen
// Geraeten und bei "Microsoft Print to PDF" ist das DIN A4; Querformat wird
// nicht gedruckt.

use std::os::raw::c_void;

// ---------------- der druckfertige Datensatz einer Person ----------------

// Nur die Anzeigetexte, damit die Aufteilung ohne Windows getestet werden kann.
#[derive(Clone, Debug, Default)]
pub struct PrintPerson {
    pub name: String,
    pub b: u16,          // Personalschluessel (unveraenderlich)
    pub maennlich: bool,
    pub geburt: String,  // Anzeigetext ("" = unbekannt)
    pub tod: String,
    pub vater: String,   // "Name (B3)" oder ""
    pub mutter: String,
    pub ehen: Vec<String>,   // je Ehe ein Eintrag "Name (B5) verh. 12.05.1850"
    pub kinder: Vec<String>, // je Kind ein Eintrag "Name (B7)", Anzeigeposition
}

// Die Zeilen einer Personenkarte auf dem Blatt: Grundblock (Name fett, Nummer,
// Daten, Eltern), dann je Ehe und je Kind eine eigene Zeile (Wunsch des
// Nutzers vom 28.10.: mehrere Ehen und Kinder untereinander). Mehrere Ehen
// werden gezaehlt. Zu lange Zeilen werden mit "…" gekuerzt, damit nichts
// ueber den Kartenrand laeuft.
pub fn druckzeilen(p: &PrintPerson) -> Vec<String> {
    let geschlecht = if p.maennlich { "maennlich" } else { "weiblich" };
    let geb = if p.geburt.is_empty() { "?" } else { p.geburt.as_str() };
    let tod = if p.tod.is_empty() { "unbekannt" } else { p.tod.as_str() };
    let eltern = match (p.vater.is_empty(), p.mutter.is_empty()) {
        (true, true) => "Eltern: unbekannt".to_string(),
        (false, true) => format!("Eltern: {}", p.vater),
        (true, false) => format!("Eltern: {}", p.mutter),
        (false, false) => format!("Eltern: {} {}", p.vater, p.mutter),
    };
    let mut zeilen = vec![
        kiirzen(&p.name, 30),
        format!("Person Nr. {} · {}", p.b, geschlecht),
        format!("geb. {}   gest. {}", geb, tod),
        kiirzen(&eltern, 50),
    ];
    for (i, e) in p.ehen.iter().enumerate() {
        let kopf = if p.ehen.len() > 1 {
            format!("Ehe {}: ", i + 1)
        } else {
            "Ehe: ".to_string()
        };
        zeilen.push(kiirzen(&format!("{}{}", kopf, e), 50));
    }
    for kind in &p.kinder {
        zeilen.push(kiirzen(&format!("Kind: {}", kind), 50));
    }
    zeilen
}

fn kiirzen(s: &str, max: usize) -> String {
    let anzahl = s.chars().count();
    if anzahl <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

// Personen in Seiten zu je `pro_seite` Stueck aufteilen. Die Regelzahl passt
// zum Raster unten (4 Reihen zu 2 Spalten); main.rs benutzt dieselbe Konstante.
pub const PERSONEN_PRO_SEITE: usize = 8;

pub fn seiten_aufteilung(liste: &[PrintPerson], pro_seite: usize) -> Vec<Vec<PrintPerson>> {
    let mut seiten = Vec::new();
    for stueck in liste.chunks(pro_seite.max(1)) {
        seiten.push(stueck.to_vec());
    }
    seiten
}

// ---------------- das 4x2-Raster auf dem Blatt ----------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feld {
    pub x: i32,
    pub y: i32,
    pub breite: i32,
    pub hoehe: i32,
}

// Acht Kartenfelder in Geraeten-Pixeln: A4-Rand (mm) um das druckbare Feld,
// vier Reihen zu zwei Spalten, dazwischen derselbe Abstand (mm). Reihenfolge:
// erst die beiden Felder der obersten Reihe (links, rechts), dann die naechste
// Reihe — dieselbe Reihenfolge wie die Personenliste.
pub fn feld_rechtecke(
    druckbar_px: (i32, i32),
    dpi: (i32, i32),
    rand_mm: i32,
    spalten_abstand_mm: i32,
) -> Vec<Feld> {
    const REIHEN: i32 = 4;
    const SPALTEN: i32 = 2;
    let px_w = |mm: i32| mm * dpi.0 / 25; // Millimeter -> Geraeten-Pixel (gerundet)
    let px_h = |mm: i32| mm * dpi.1 / 25;
    let rand_x = px_w(rand_mm);
    let rand_y = px_h(rand_mm);
    let abstand_x = px_w(spalten_abstand_mm);
    let abstand_y = px_h(spalten_abstand_mm);
    let (bw, bh) = druckbar_px;
    if bw <= 2 * rand_x + (SPALTEN - 1) * abstand_x
        || bh <= 2 * rand_y + (REIHEN - 1) * abstand_y
    {
        return Vec::new(); // zu schmal/hoch fuer das Raster
    }
    let breite = (bw - 2 * rand_x - (SPALTEN - 1) * abstand_x) / SPALTEN;
    let hoehe = (bh - 2 * rand_y - (REIHEN - 1) * abstand_y) / REIHEN;
    let mut felder = Vec::with_capacity((REIHEN * SPALTEN) as usize);
    for reihe in 0..REIHEN {
        for spalte in 0..SPALTEN {
            felder.push(Feld {
                x: rand_x + spalte * (breite + abstand_x),
                y: rand_y + reihe * (hoehe + abstand_y),
                breite,
                hoehe,
            });
        }
    }
    felder
}

// ---------------- Windows: Drucker aufzaehlen ----------------

#[repr(C)]
struct DruckerInfo4 {
    drucker_name: *mut u16,
    server_name: *mut u16,
    eigenschaften: u32,
    merkmale: u32,
}

#[link(name = "winspool")]
extern "system" {
    // PRINTER_ENUM_LOCAL (2) | PRINTER_ENUM_CONNECTIONS (4)
    fn EnumPrintersW(
        flags: u32,
        name: *const u16,
        level: u32,
        p: *mut u8,
        cbbuf: u32,
        pcbneeded: *mut u32,
        pcreturned: *mut u32,
    ) -> i32;
    fn GetDefaultPrinterW(buf: *mut u16, pcch: *mut u32) -> i32;
}

// Text in der Drucker-Zaelle: die Namen zeigen, die der Windows-Druckerliste
// entsprechen. Beim ersten Aufruf mit leerem Puffer nennt Windows die noetige
// Groesse; was in der Zaelle steht (Abstand oder echte Adresse), schaetzt
// druckername() sicher ein.
pub fn drucker_liste() -> Vec<String> {
    unsafe {
        let mut benoetigt: u32 = 0;
        let mut zurueck: u32 = 0;
        EnumPrintersW(
            2 | 4,
            std::ptr::null(),
            4,
            std::ptr::null_mut(),
            0,
            &mut benoetigt,
            &mut zurueck,
        );
        if benoetigt == 0 {
            return Vec::new();
        }
        let mut puffer = vec![0u8; benoetigt as usize];
        let ok = EnumPrintersW(
            2 | 4,
            std::ptr::null(),
            4,
            puffer.as_mut_ptr(),
            benoetigt,
            &mut benoetigt,
            &mut zurueck,
        );
        if ok == 0 {
            return Vec::new();
        }
        // PRINTER_INFO_4 auf 64 Bit: zwei Zeiger (je 8) und zwei u32 = 24 Byte.
        // Die Windows-Handbuchung sagt, die Felder seien Abstaende zum Pufferanfang;
        // diese Windows-Fassung liefert aber echte Adressen (die Diagnose-Testrunde
        // 28.10. zeigte fallende Zeigerwerte um 2,2 Billionen). Beides wird getragen:
        // kleiner Wert = Abstand, Wert im eigenen Puffer = Adresse minus Pufferanfang,
        // Adresse ausserhalb = Speicher direkt lesen (er liegt im eigenen Prozess).
        const TRENNWEITE: usize = 24;
        let anfang_des_puffers = puffer.as_ptr() as usize;
        let mut namen = Vec::new();
        for i in 0..zurueck as usize {
            let stelle = i * TRENNWEITE;
            if stelle + TRENNWEITE > puffer.len() {
                break;
            }
            let wert = usize_aus(&puffer, stelle);
            if let Some(name) = druckername(&puffer, anfang_des_puffers, wert) {
                namen.push(name);
            }
        }
        let _ = core::mem::size_of::<DruckerInfo4>(); // Groesse der Struktur sichern
        namen
    }
}

// Den Namen aus einem der drei moeglichen Formate holen (siehe oben).
fn druckername(puffer: &[u8], anfang_des_puffers: usize, wert: usize) -> Option<String> {
    if wert < puffer.len() {
        return u16_text_aus(puffer, wert);
    }
    if wert >= anfang_des_puffers && wert < anfang_des_puffers + puffer.len() {
        return u16_text_aus(puffer, wert - anfang_des_puffers);
    }
    // Echte Adresse im eigenen Prozess: UTF-16-Text bis zum Nullzeichen ablesen.
    let zeiger = wert as *const u16;
    let mut worte = Vec::new();
    unsafe {
        let mut i = 0usize;
        while i < 1024 {
            let wort = *zeiger.add(i);
            if wort == 0 {
                break;
            }
            worte.push(wort);
            i += 1;
        }
    }
    if worte.is_empty() {
        return None;
    }
    String::from_utf16(&worte).ok()
}

// Der von Windows gemerkte Standarddrucker ("—" im Auswahlfenster).
pub fn standard_drucker() -> Option<String> {
    unsafe {
        let mut anzahl: u32 = 0;
        GetDefaultPrinterW(std::ptr::null_mut(), &mut anzahl);
        if anzahl == 0 {
            return None;
        }
        let mut puffer = vec![0u16; anzahl as usize + 1];
        let ok = GetDefaultPrinterW(puffer.as_mut_ptr(), &mut anzahl);
        if ok == 0 {
            return None;
        }
        let ende = puffer.iter().position(|&c| c == 0).unwrap_or(puffer.len());
        String::from_utf16(&puffer[..ende]).ok()
    }
}

fn usize_aus(puffer: &[u8], anfang: usize) -> usize {
    let mut b = [0u8; 8];
    b.copy_from_slice(&puffer[anfang..anfang + 8]);
    usize::from_le_bytes(b)
}

// UTF-16-Text bis zum Nullzeichen, von einer Stelle im Bytepuffer aus.
fn u16_text_aus(puffer: &[u8], abstand: usize) -> Option<String> {
    if abstand + 1 >= puffer.len() {
        return None;
    }
    let mut worte = Vec::new();
    let mut i = abstand;
    while i + 1 < puffer.len() {
        let wort = u16::from_le_bytes([puffer[i], puffer[i + 1]]);
        if wort == 0 {
            break;
        }
        worte.push(wort);
        i += 2;
    }
    String::from_utf16(&worte).ok()
}

// ---------------- Windows: drucken ----------------

#[repr(C)]
struct DokumentInfo1 {
    dokument_name: *const u16,
    ausgabe_datei: *const u16,
    daten_art: *const u16,
}

// GetDeviceCaps-Kennungen
const HORZRES: i32 = 8; // Breite des druckbaren Felds in Pixeln
const VERTRES: i32 = 10; // Hoehe des druckbaren Felds in Pixeln
const LOGPIXELSX: i32 = 88; // wagrechte Punktdichte
const LOGPIXELSY: i32 = 90; // senkrechte Punktdichte
// CreateFontW
const FW_NORMAL: u32 = 400;
const FW_BOLD: u32 = 700;
const DEFAULT_CHARSET: u32 = 1;
const CLEARTYPE_QUALITY: u32 = 5;

#[link(name = "gdi32")]
extern "system" {
    fn CreateDCW(
        treiber: *const u16,
        geraet: *const u16,
        ausgabe: *const u16,
        einstellungen: *const c_void,
    ) -> *mut c_void;
    fn DeleteDC(dc: *mut c_void) -> i32;
    fn StartDocW(dc: *mut c_void, info: *const DokumentInfo1) -> i32;
    fn StartPage(dc: *mut c_void) -> i32;
    fn EndPage(dc: *mut c_void) -> i32;
    fn EndDoc(dc: *mut c_void) -> i32;
    fn GetDeviceCaps(dc: *mut c_void, kennung: i32) -> i32;
    fn CreateFontW(
        hoehe: i32,
        breite: i32,
        neigung: i32,
        drehung: i32,
        gewicht: u32,
        kursiv: u8,
        unterstrichen: u8,
        durchgestrichen: u8,
        zeichensatz: u32,
        ausgabe_genauigkeit: u32,
        beschnitt_genauigkeit: u32,
        qualitaet: u32,
        schnitt_und_familie: u32,
        gesicht: *const u16,
    ) -> *mut c_void;
    fn SelectObject(dc: *mut c_void, objekt: *mut c_void) -> *mut c_void;
    fn DeleteObject(objekt: *mut c_void) -> i32;
    fn TextOutW(dc: *mut c_void, x: i32, y: i32, text: *const u16, anzahl: i32) -> i32;
    fn Rectangle(dc: *mut c_void, links: i32, oben: i32, rechts: i32, unten: i32) -> i32;
    fn SetBkMode(dc: *mut c_void, modus: i32) -> i32;
}

// Den Blattdruck an den benannten Drucker uebergeben. `seiten` ist die von
// seiten_aufteilung erzeugte Liste; gedruckt wird genau diese Reihenfolge.
// Rueckgabe: Menschlesbarer Bericht ueber die Seitenzahl.
pub fn drucke_personen(drucker: &str, seiten: &[Vec<PrintPerson>]) -> Result<String, String> {
    if seiten.is_empty() {
        return Err("Es gibt nichts zu drucken: der Datensatz hat keine Mitglieder.".to_string());
    }
    let name16 = u16_z(drucker);
    let dokument16 = u16_z("Racines-Mitgliederblatt");
    unsafe {
        let dc = CreateDCW(std::ptr::null(), name16.as_ptr(), std::ptr::null(), std::ptr::null());
        if dc.is_null() {
            return Err(format!(
                "Der Drucker '{}' konnte nicht geoeffnet werden (existiert er noch?).",
                drucker
            ));
        }
        let dpi = (
            GetDeviceCaps(dc, LOGPIXELSX).max(1),
            GetDeviceCaps(dc, LOGPIXELSY).max(1),
        );
        let druckbar = (
            GetDeviceCaps(dc, HORZRES).max(1),
            GetDeviceCaps(dc, VERTRES).max(1),
        );
        // Zwei Schriftgroessen: Name fett, die uebrigen Zeilen gewoehnlich.
        let schrift_normal = CreateFontW(
            -(10 * dpi.1 / 72),
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            u16_z("Segoe UI").as_ptr(),
        );
        let fett = CreateFontW(
            -(13 * dpi.1 / 72),
            0,
            0,
            0,
            FW_BOLD,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            u16_z("Segoe UI").as_ptr(),
        );
        if schrift_normal.is_null() || fett.is_null() {
            DeleteDC(dc);
            return Err("Die Druckschriften konnten nicht erzeugt werden.".to_string());
        }
        // Hilfschriften fuer volle Karten: mehrere Ehen oder Kinder fuellen die
        // Karte; dann wird alles eine Stufe kleiner gesetzt statt abzusaegen.
        let schrift_klein = CreateFontW(
            -(8 * dpi.1 / 72),
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            u16_z("Segoe UI").as_ptr(),
        );
        let fett_klein = CreateFontW(
            -(11 * dpi.1 / 72),
            0,
            0,
            0,
            FW_BOLD,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            u16_z("Segoe UI").as_ptr(),
        );
        if schrift_klein.is_null() || fett_klein.is_null() {
            DeleteDC(dc);
            return Err("Die Druckschriften konnten nicht erzeugt werden.".to_string());
        }
        let info = DokumentInfo1 {
            dokument_name: dokument16.as_ptr(),
            ausgabe_datei: std::ptr::null(),
            daten_art: std::ptr::null(),
        };
        let auftrag = StartDocW(dc, &info);
        if auftrag <= 0 {
            DeleteDC(dc);
            return Err(format!(
                "Der Druckauftrag konnte nicht gestartet werden (Fehler {}).",
                auftrag
            ));
        }
        SetBkMode(dc, 1); // TRANSPARENT: kein weisser Kasten hinter der Schrift
        let felder = feld_rechtecke(druckbar, dpi, 12, 7);
        let rand_px = 8 * dpi.0 / 100; // Innenabstand in der Karte
        let zeile_normal = 10 * dpi.1 / 72 * 13 / 10; // Zeilenabstand der normalen Schrift
        let zeile_fett = 13 * dpi.1 / 72 * 12 / 10;
        let zeile_normal_k = 8 * dpi.1 / 72 * 13 / 10; // dieselben Masze der kleinen Stufe
        let zeile_fett_k = 11 * dpi.1 / 72 * 12 / 10;
        for seite in seiten {
            if StartPage(dc) <= 0 {
                EndDoc(dc);
                DeleteDC(dc);
                return Err("Eine Seite konnte nicht begonnen werden.".to_string());
            }
            for (nr, person) in seite.iter().enumerate() {
                if nr >= felder.len() {
                    break; // die Aufteilung liefert hoechstens 8 je Seite
                }
                let feld = &felder[nr];
                // Duenner Rand um die Personenkarte (Standardstift, schwarz).
                Rectangle(
                    dc,
                    feld.x,
                    feld.y,
                    feld.x + feld.breite,
                    feld.y + feld.hoehe,
                );
                let zeilen = druckzeilen(person);
                // Volle Karten (mehrere Ehen/Kinder) bekommen die kleinere
                // Schriftstufe, damit alle Zeilen in den Rand passen.
                let voll = zeile_fett
                    + (zeilen.len().saturating_sub(1) as i32) * zeile_normal
                    > feld.hoehe - 2 * rand_px;
                let (schrift_f, schrift_n) =
                    if voll { (fett_klein, schrift_klein) } else { (fett, schrift_normal) };
                let (zeile_f, zeile_n) =
                    if voll { (zeile_fett_k, zeile_normal_k) } else { (zeile_fett, zeile_normal) };
                SelectObject(dc, schrift_f);
                let mut y = feld.y + rand_px;
                for (i, zeile) in zeilen.iter().enumerate() {
                    let schritt = if i == 0 { zeile_f } else { zeile_n };
                    if y + schritt > feld.y + feld.hoehe {
                        break; // auch die kleine Stufe reicht nicht — Rest abbrechen
                    }
                    if i == 1 {
                        SelectObject(dc, schrift_n);
                    }
                    let t = u16_z(zeile);
                    TextOutW(dc, feld.x + rand_px, y, t.as_ptr(), (t.len() - 1) as i32);
                    y += schritt;
                }
            }
            if EndPage(dc) <= 0 {
                EndDoc(dc);
                DeleteDC(dc);
                return Err("Eine Seite konnte nicht abgeschlossen werden.".to_string());
            }
        }
        EndDoc(dc);
        DeleteDC(dc);
        // Die vier Schriften haengen nicht mehr am Gerat — jetzt freigeben.
        for schrift in [schrift_normal, fett, schrift_klein, fett_klein] {
            DeleteObject(schrift);
        }
    }
    Ok(format!(
        "Druckauftrag uebergeben: {} Seite(n) mit {} Mitgliedern an Drucker '{}'.",
        seiten.len(),
        seiten.iter().map(|s| s.len()).sum::<usize>(),
        drucker
    ))
}

// Null-abgeschlossener UTF-16-Puffer (die Windows-Funktionen verlangen ihn).
fn u16_z(text: &str) -> Vec<u16> {
    let mut v: Vec<u16> = text.encode_utf16().collect();
    v.push(0);
    v
}

// ---------------- Tests ----------------

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, b: u16, maennlich: bool) -> PrintPerson {
        PrintPerson {
            name: name.to_string(),
            b,
            maennlich,
            geburt: "12.05.1850".to_string(),
            tod: String::new(),
            vater: "Vater (B3)".to_string(),
            mutter: String::new(),
            ..Default::default()
        }
    }

    #[test]
    fn seiten_aufteilung_vierzehn_personen_auf_acht_je_seite() {
        let liste: Vec<PrintPerson> = (0..14).map(|i| person("X", i + 1, true)).collect();
        let seiten = seiten_aufteilung(&liste, PERSONEN_PRO_SEITE);
        assert_eq!(seiten.len(), 2, "14 Personen brauchen zwei Seiten");
        assert_eq!(seiten[0].len(), 8);
        assert_eq!(seiten[1].len(), 6, "die letzte Seite traegt den Rest");
        assert_eq!(seiten[1][0].b, 9, "die Reihenfolge bleibt erhalten");
    }

    #[test]
    fn feld_rechtecke_liegen_in_vier_reihen_und_innerhalb_des_blatts() {
        // wie ein 300-dpi-Drucker auf A4 (Horzres/Vertres des druckbaren Felds)
        let felder = feld_rechtecke((3301, 4537), (300, 300), 12, 7);
        assert_eq!(felder.len(), 8);
        // zwei Felder je Reihe, vier Reihen untereinander
        assert_eq!(felder[0].y, felder[1].y);
        assert_eq!(felder[2].y, felder[3].y);
        assert_eq!(felder[4].y, felder[5].y);
        assert_eq!(felder[6].y, felder[7].y);
        assert!(
            felder[0].y < felder[2].y && felder[2].y < felder[4].y && felder[4].y < felder[6].y,
            "die Reihen folgen von oben nach unten"
        );
        // von links nach rechts aufsteigend
        assert!(felder[0].x < felder[1].x);
        assert_eq!(felder[0].x, felder[6].x, "die Spalten stehen senkrecht uebereinander");
        // alles innerhalb des druckbaren Felds, mit gleich grossen Karten
        for f in &felder {
            assert!(f.x >= 0 && f.y >= 0);
            assert!(f.x + f.breite <= 3301 && f.y + f.hoehe <= 4537);
            assert!(f.breite > 1200 && f.hoehe > 700, "eine Karte muss Text aufnehmen koennen");
        }
        assert_eq!(felder[0].breite, felder[7].breite);
    }

    #[test]
    fn zu_kleines_druckfeld_ergibt_keine_felder() {
        assert!(feld_rechtecke((100, 100), (300, 300), 12, 7).is_empty());
    }

    #[test]
    fn druckzeilen_nennen_name_schluessel_daten_und_eltern() {
        let zeilen = druckzeilen(&person("Anna Maria", 7, false));
        assert_eq!(zeilen[0], "Anna Maria");
        assert_eq!(zeilen[1], "Person Nr. 7 · weiblich");
        assert_eq!(zeilen[2], "geb. 12.05.1850   gest. unbekannt");
        assert_eq!(zeilen[3], "Eltern: Vater (B3)");
        // ohne alle Einzelheiten: unbekannte Werte als Text
        let leer = PrintPerson {
            name: "Ohne Alles".to_string(),
            b: 1,
            maennlich: true,
            ..Default::default()
        };
        let z2 = druckzeilen(&leer);
        assert_eq!(z2[2], "geb. ?   gest. unbekannt");
        assert_eq!(z2[3], "Eltern: unbekannt");
        assert_eq!(z2.len(), 4, "ohne Ehe und Kinder bleibt es beim Grundblock");
        // eine Ehe, ein Kind: je eine Zeile ohne Nummerierung
        // NAMEN-ALT: der bisherige Kunstname ahnelte dem echten Familien-
        // vorbild aus .006; neutral umbenannt auf "Erika" (Suchwort NAMEN-ALT).
        let familie = PrintPerson {
            ehen: vec!["Erika (B5) verh. 12.05.1850".to_string()],
            kinder: vec!["Kind A (B7)".to_string()],
            ..person("Vater", 3, true)
        };
        let z3 = druckzeilen(&familie);
        assert_eq!(z3.len(), 6);
        assert_eq!(z3[4], "Ehe: Erika (B5) verh. 12.05.1850");
        assert_eq!(z3[5], "Kind: Kind A (B7)");
        // mehrere Ehen und Kinder: jede Verbindung eigene Zeile, Ehen gezaehlt
        let viele = PrintPerson {
            ehen: vec![
                "Erika (B5) verh. 12.05.1850".to_string(),
                "Berta (B9)".to_string(),
            ],
            kinder: vec![
                "Kind A (B7)".to_string(),
                "Kind B (B8)".to_string(),
                "X".repeat(60),
            ],
            ..person("Vater", 3, true)
        };
        let z4 = druckzeilen(&viele);
        assert_eq!(z4.len(), 9, "2 Ehen und 3 Kinder als 5 eigene Zeilen");
        assert_eq!(z4[4], "Ehe 1: Erika (B5) verh. 12.05.1850");
        assert_eq!(z4[5], "Ehe 2: Berta (B9)");
        assert_eq!(z4[6], "Kind: Kind A (B7)");
        assert_eq!(z4[7], "Kind: Kind B (B8)");
        assert_eq!(z4[8].chars().count(), 50, "zu lange Zeilen gekuerzt, nicht umbrochen");
        // zu lange Namen werden mit Auslassungspunkt gekuerzt
        let lang = person(&"L".repeat(40), 2, true);
        assert_eq!(druckzeilen(&lang)[0].chars().count(), 30);
        assert!(druckzeilen(&lang)[0].ends_with('…'));
    }

    // Die Drucker-Aufzaehlung ist Windows-Arbeit; getestet wird nur, dass sie
    // auf diesem Rechner fehlerfrei durchlaeuft (liste darf ruhig gefuellt sein).
    #[test]
    fn druckerliste_und_standarddrucker_arbeiten_fehlerfrei() {
        let liste = drucker_liste();
        let standard = standard_drucker();
        if let Some(s) = &standard {
            assert!(!s.is_empty(), "ein Standarddrucker hat einen Namen");
            assert!(
                liste.iter().any(|n| n == s) || liste.is_empty(),
                "der Standarddrucker ({}) steht auch in der Aufzaehlung",
                s
            );
        }
        assert!(
            !liste.is_empty(),
            "auf diesem Rechner sind Drucker eingerichtet — die Liste darf nicht leer sein"
        );
    }
}
