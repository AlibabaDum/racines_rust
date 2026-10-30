// Racines-Kartei: grafische Oberflaeche (egui/eframe) ueber racines-core.
// Menueleiste: Datei / Karteikarte / Stammbaum. Unter Datei steht
// "Verzeichnis auswaehlen..."; das gewaehlte DATEIEN-Verzeichnis wird in
// config.ini gemerkt und beim Start automatisch geladen.
// Bearbeiten: Formular rechts; Speichern legt immer zuerst Sicherungskopie an
// (.bak / .bak~1..9). Der frihere Schreibschutz von C:\ahnen ist aufgehoben —
// es existieren mehrere Sicherungen, und beim Speichern kommt stets eine neue dazu.
// Senkrechte Leiste ganz links: Bildknoepfe fuer neue Karte, Loeschen und
// Speichern, jeweils mit Mouseover-Erklartext (Bestaetigung direkt darunter).
// Kinder-Beziehungen (L1) sind im Formular bearbeitbar: Hinzuwaehlen und
// Austragen arbeitet immer beidseitig (L1-Paar der Elternkarte + Elternfach
// w1/w2 der Kindkarte); andert das Formular die Elternschluessel, ziehen die
// Kinderlisten der betroffenen Elternkarten mit.
// Heirat (L2) ist in der Kartei wie die Kinder bearbeitbar: das v-Feld des
// Ereignisses ist der B-Schluessel des Ehepartners (bestaetigt an .006: Karte
// B2 traegt v=5 auf B5, Karte B5 traegt v=2 auf B2 — NAMEN-ALT: dort standen
// frueher echte Familiennamen, sie bleiben bewusst auch im Kommentar weg);
// Hinzuwaehlen und Entfernen wirkt auf beide Karten.
// Das Heiratsdatum sitzt als Eingabefeld in der Zeile (anwenden mit "Datum setzen").
// Datei -> "Drucken...": Mitgliederblatt aller Karten des Datensatzes, je 8
// Personen auf einer A4-Seite (4 Reihen zu 2 Spalten); der Drucker wird vorher
// in einem eigenen Fenster gewaehlt (print.rs, Windows-GDI ohne Zusatzcrates).
// Ereignistafeln (Geburt/Taufe/Heirat/Tod/Benediktion/Begraebnis): eigene Reiter, reine Lese-Ansicht.
// Die Tafeln werden weiter geladen; ihre Reiter-Knopfe sind seit 28.10. per Nutzerwunsch
// auskommentiert (Marker REITER-AUS-KOMMENTAR in der Reiterzeile) — sichtbar sind nur Karten
// und Stammbaum.
// Release: GUI-Anwendung ohne Konsole-Fenster (Cargo baut Windows-Programme
// sonst als Konsole-Anwendung, und zum Doppelstart oeffnet Windows dann immer
// ein zusaetzliches Terminalfenster). Im Debug-Build bleibt die Konsole erhalten,
// damit Fehlermeldungen beim Entwickeln sichtbar sind.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rac;
mod tree;
mod print;

use eframe::egui;
use rac::*;
use std::path::PathBuf;
use tree::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 720.0])
            .with_min_inner_size([820.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Racines-Kartei (Ahnengalerie 1.51)",
        opts,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
    .map_err(|e| format!("eFrame: {}", e))?;
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct DateForm {
    datum: String, // "" = unveraendert lassen
    ort: String,
}

#[derive(Clone, Debug, Default)]
struct Form {
    name: String,
    sex: char, // 'H' / 'F'
    a: String,
    c: String,
    s: [String; 5],
    dt: [DateForm; 3], // Geburt, Heirat, Tod
    vater: String,     // B-Schluessel als Zahlentext
    mutter: String,
}

fn load_form(k: &Card) -> Form {
    let mut f = Form {
        name: decode_cp1252(&k.name),
        a: k.a.to_string(),
        c: k.c.to_string(),
        vater: k.w1.to_string(),
        mutter: k.w2.to_string(),
        ..Default::default()
    };
    f.sex = if k.sex == b'F' { 'F' } else { 'H' };
    for i in 0..5 {
        f.s[i] = decode_cp1252(&k.s[i]);
    }
    for i in 0..3 {
        f.dt[i].datum = fmt_date(&k.dt[i].raw);
        f.dt[i].ort = decode_cp1252(&k.dt[i].place);
    }
    f
}

fn num(s: &str) -> Result<u16, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(0);
    }
    t.parse::<u16>().map_err(|_| format!("'{}' ist keine Zahl 0..65535", t))
}

// Formularwerte auf die Karte uebertragen (wie apply_kv), mit Fehlermeldungen.
// Datumsrohwerte nur aendern, wenn das Textfeld gegenueber der Anzeige geändert wurde —
// dann bleibt der Kartenrest (q-Wort, Tabellenzeiger) bitgenau erhalten.
fn apply_form(k: &mut Card, f: &Form) -> Result<(), String> {
    k.name = encode_cp1252(&f.name);
    for i in 0..5 {
        k.s[i] = encode_cp1252(&f.s[i]);
    }
    for i in 0..3 {
        k.dt[i].place = encode_cp1252(&f.dt[i].ort);
        let want = f.dt[i].datum.trim();
        if !want.is_empty() && want != fmt_date(&k.dt[i].raw) {
            if !set_date(&mut k.dt[i].raw, want) {
                return Err(format!(
                    "Datum Feld {}: '{}' — erwartet TT.MM.JJJJ, JJJJ oder '-'",
                    i + 1,
                    want
                ));
            }
        }
    }
    k.a = num(&f.a)?;
    k.c = num(&f.c)?;
    k.w1 = num(&f.vater)?;
    k.w2 = num(&f.mutter)?;
    k.sex = if f.sex == 'F' { b'F' } else { b'H' };
    Ok(())
}

fn short_name(k: &Card) -> String {
    let n = decode_cp1252(&k.name);
    if n.is_empty() {
        "(ohne Namen)".to_string()
    } else {
        n
    }
}

// Kurztext fuer die Tafel-Listen: kartierte Felder, sonst die beiden ersten Textfunde.
fn ev_summary(r: &EvRec) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for (l, v) in &r.fields {
        if *l != "Geschlecht" {
            parts.push(v.as_str());
        }
    }
    if parts.is_empty() && r.used {
        for (_, s) in r.strings.iter().take(2) {
            parts.push(s.as_str());
        }
    }
    if parts.is_empty() {
        if r.used {
            "(kein lesbarer Text)".to_string()
        } else {
            "(frei)".to_string()
        }
    } else {
        parts.truncate(4);
        parts.join(" | ")
    }
}

// ---------- Senkrechte Werkzeugleiste: quadratische Bildknoepfe ----------
// Ein Knopf aus festem Quadrat: das Symbol malt `malen` direkt als Linien und
// Rechtecke (egui bringt keine Bildsymbole mit), und der Erklaertext erscheint
// bei Mouseover als Sprechblase. Deaktivierte Knopfe bleiben grau und tun nichts.
fn ikon_knopf(
    ui: &mut egui::Ui,
    enabled: bool,
    hover: &str,
    malen: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(42.0, 34.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if enabled && (resp.hovered() || resp.has_focus()) {
            p.rect_filled(rect, 6.0, ui.visuals().faint_bg_color);
        }
        if enabled && resp.is_pointer_button_down_on() {
            p.rect_filled(rect, 6.0, ui.visuals().widgets.active.bg_fill);
        }
        let farbe = if enabled {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        malen(p, rect, farbe);
    }
    resp.on_hover_text(hover)
}

// Symbol "neue Karte": Karteikartenblatt mit Zeilen, davor ein grosses Plus.
fn malen_neue_karte(p: &egui::Painter, r: egui::Rect, farbe: egui::Color32) {
    let duenn = egui::Stroke::new(1.4f32, farbe);
    let dick = egui::Stroke::new(2.0f32, farbe);
    let c = r.center();
    let blatt = egui::Rect::from_center_size(c + egui::vec2(-4.0, 0.0), egui::vec2(14.0, 18.0));
    p.rect_stroke(blatt, 2.0, duenn);
    p.line_segment([blatt.min + egui::vec2(2.5, 5.0), blatt.min + egui::vec2(11.5, 5.0)], duenn);
    p.line_segment([blatt.min + egui::vec2(2.5, 8.5), blatt.min + egui::vec2(9.0, 8.5)], duenn);
    let pc = c + egui::vec2(9.0, 6.0);
    p.line_segment([pc - egui::vec2(5.0, 0.0), pc + egui::vec2(5.0, 0.0)], dick);
    p.line_segment([pc - egui::vec2(0.0, 5.0), pc + egui::vec2(0.0, 5.0)], dick);
}

// Symbol "Karte loeschen": Muelleimer mit Henkel, Deckel und zwei Streben.
fn malen_karte_loeschen(p: &egui::Painter, r: egui::Rect, farbe: egui::Color32) {
    let linie = egui::Stroke::new(1.4f32, farbe);
    let c = r.center();
    p.line_segment([c + egui::vec2(-2.5, -10.0), c + egui::vec2(2.5, -10.0)], linie);
    p.line_segment([c + egui::vec2(-9.0, -7.0), c + egui::vec2(9.0, -7.0)], linie);
    let korb = egui::Rect::from_min_max(c + egui::vec2(-7.0, -4.5), c + egui::vec2(7.0, 11.0));
    p.rect_stroke(korb, 1.5, linie);
    p.line_segment([c + egui::vec2(-2.5, -2.0), c + egui::vec2(-2.5, 8.5)], linie);
    p.line_segment([c + egui::vec2(2.5, -2.0), c + egui::vec2(2.5, 8.5)], linie);
}

// Symbol "speichern": Diskette mit abgeschaegter Ecke, Schieber und Etikett.
fn malen_speichern(p: &egui::Painter, r: egui::Rect, farbe: egui::Color32) {
    let linie = egui::Stroke::new(1.4f32, farbe);
    let g = egui::Rect::from_center_size(r.center(), egui::vec2(22.0, 20.0));
    p.add(egui::Shape::closed_line(
        vec![
            g.min,
            egui::pos2(g.max.x - 6.0, g.min.y),
            egui::pos2(g.max.x, g.min.y + 6.0),
            g.max,
            egui::pos2(g.min.x, g.max.y),
        ],
        linie,
    ));
    let schieber = egui::Rect::from_min_max(
        egui::pos2(g.min.x + 5.0, g.min.y + 1.0),
        egui::pos2(g.max.x - 9.0, g.min.y + 7.0),
    );
    p.rect_filled(schieber, 1.0, farbe);
    let etikett = egui::Rect::from_min_max(
        egui::pos2(g.min.x + 3.0, g.center().y),
        egui::pos2(g.max.x - 3.0, g.max.y - 3.0),
    );
    p.rect_filled(etikett, 1.0, farbe);
}

// Rechter fuer die drei Bildknoepfe: egui 0.27 hat kein Rect::default().
#[derive(Clone, Copy)]
struct IkonRechtecke([egui::Rect; 3]);

impl Default for IkonRechtecke {
    fn default() -> Self {
        IkonRechtecke([egui::Rect::NOTHING; 3])
    }
}

#[derive(Default)]
struct MyApp {
    folder: String,
    datasets: Vec<DatasetInfo>,
    loaded: Option<PathBuf>, // Pfad der geladenen Kartendatei
    loaded_title: String,
    cards: Vec<Card>,
    dirty: bool,
    selected: usize,
    form: Form,
    form_for: usize, // Index, fuer den das Formular gilt (usize::MAX = ungueltig)
    // Auswahl im Combo "Kind hinzuwaehlen" (Zahlentext des B-Schluessels,
    // "" oder "0" = nichts gewaehlt); wird bei Kartenwechsel zurueckgesetzt.
    kind_neu: String,
    // Auswahl im Combo "Ehepartner hinzuwaehlen" (gleiches Muster wie kind_neu).
    heirat_neu: String,
    // Zahlentexte der Datumsfelder in den Heirats-Zeilen (L2), eine pro
    // Ereignis der markierten Karte; bei Kartenwechsel oder L2-Operationen
    // neu aus den Rohblöcken aufgebaut, sonst bleibt Tipparbeit erhalten.
    ehen_dat: Vec<String>,
    filter: String,
    status: Vec<String>,
    confirm_save: bool,
    confirm_del: bool,
    // Schliess-Rueckfrage (3. Runde 30.10., Nutzerwunsch): wahr, solange die
    // Frage nach Sichern oder Verwerfen offen ist. Ausgeloest wird sie vom
    // Schliessbefehl (X-Knopf des Fensters oder Menue Datei -> Beenden),
    // solange der Datensatz ungespeicherte Aenderungen hat (dirty).
    confirm_quit: bool,
    // Rechtecke der drei Bildknoepfe in der senkrechten Leiste ganz links
    // (0 = neue Karte, 1 = loeschen, 2 = speichern); Tests klicken damit gezielt.
    icon_rects: IkonRechtecke,
    show_tech: bool,
    scan_requested: bool,
    // Dateiauswahl (Menue Datei -> "Verzeichnis auswaehlen..."):
    picker_open: bool,
    picker_dir: PathBuf, // aktuell angezeigter Ordner im Auswahlfenster
    picker_dir_text: String, // dazu der editierbare Text im Eingabefeld
    picker_entries: Vec<(String, PathBuf)>, // Unterordner des angezeigten Ordners
    picker_files: Vec<(String, PathBuf)>, // gefundene FICHES-Dateien dort
    picker_note: String, // Rueckmeldung im Fenster
    // Datei-Menue -> "Drucken...": Mitgliederblatt, je 8 Personen je A4-Seite
    // (4 Reihen zu 2 Spalten). Der Drucker wird im Fenster vorher gewaehlt.
    print_open: bool,
    print_drucker: String, // aktuell gewaehlter Druckername
    print_drucker_liste: Vec<String>, // Drucker-namen aus Windows
    print_standard: String, // von Windows gemerkter Standarddrucker (Info)
    print_note: String, // Fehler im Druckfenster (es bleibt offen)
    // Pfad der config-Datei (gemerkter DATEIEN-Ordner), beim Start ermittelt:
    config_path: PathBuf,
    cfg_dir_override: Option<PathBuf>, // nur fuer Tests: eigener Config-Ordner
    // Ereignistafeln (Reiter, reine Lese-Ansicht):
    ev_tabs: Vec<EvTable>,
    ev_problems: Vec<String>,
    tab: usize, // 0 = Karten, i+1 = ev_tabs[i]
    ev_filter: String,
    ev_show_free: bool,
    ev_selected: usize, // Satz-Index (r.idx) des markierten Satzes
    ev_tech: bool,      // auch bei kartierten Tafeln die Textlaeufe zeigen
    // Stammbaum (Reiter, reine Lese-Ansicht): Eltern-Kind-Beziehungen als Graph.
    tree_view: TreeView, // fertiger Layout-Baum, nur bei Klick neu aufgebaut
    tree_mode: Mode,     // Desc = nachfolgende Generationen, Anc = Vorfahren
    tree_gens: usize,    // Tiefengrenze (3 = drei vorhergehende Familien)
    tree_sel: usize,     // Kartenindex der Wurzel (Index in cards)
    tree_filter: String, // Suche fuer die Wurzel-Auswahl
    tree_off: egui::Vec2, // Verschieben in Pixeln (Ziehen)
    tree_rect: Option<egui::Rect>, // zuletzt benutzte Zeichenflaeche (Zoom, Tests)
    tree_scale: f32,      // Zoomfaktor
    tree_fit: bool,       // Baum beim naechsten Zeichnen einpassen
    tree_fit_min: f32,    // untere Zoomgrenze des Einpassens (0.7 automatisch —
                          // Namen bleiben lesbar; 0.06 nach "Einpassen": alles sichtbar)
}

const DEFAULT_FOLDER: &str = "C:\\ahnen\\DATEIEN";
const CONFIG_FILE: &str = "config.ini";

// Config: eine Handvoll "schluessel = wert"-Zeilen, ohne zusaetzliches Crate.
// Einzler Schluessel bisher: dateien_ordner — das vom Benutzer gewaehlte
// DATEIEN-Verzeichnis (Menue Datei -> "Verzeichnis auswaehlen...").
fn config_read(path: &std::path::Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                let v = v.trim().trim_matches('"').to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

// Ganze Datei neu schreiben (nur ein Schluessel bisher) und dem Aufrufer
// eine Statuszeile geben, damit klar ist, was mit dem Gemerken passiert ist.
fn config_write(path: &std::path::Path, key: &str, value: &str) -> String {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = format!(
        "# Racines-Kartei: hier steht das gemerkte DATEIEN-Verzeichnis.\n{key} = {value}\n"
    );
    match std::fs::write(path, content) {
        Ok(()) => format!("Gemerkt in {}: {}", path.display(), value),
        Err(e) => format!("Config {} nicht beschreibbar: {}", path.display(), e),
    }
}

// Standardpfad der Config: zuerst im Arbeitsverzeichnis, sonst neben der
// Exe-Datei (doppelter Klick auf das Programm arbeitet oft im Zielordner).
fn default_config_path() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let im_cwd = cwd.join(CONFIG_FILE);
    if im_cwd.exists() {
        return im_cwd;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let neben_exe = dir.join(CONFIG_FILE);
            if neben_exe.exists() {
                return neben_exe;
            }
        }
    }
    im_cwd
}

// Unterordner eines Ordners, alphabetisch sortiert; Lesefehler bleiben still.
fn list_subdirs(dir: &std::path::Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    Some((n, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    v
}

// Karten-Dateien (FICHES*/FICHESS*) in einem Ordner: das Auswahlfenster zeigt
// als Hinweis, ob der angezeigte Ordner wirklich Karten enthaelt.
fn list_fiches_in(dir: &std::path::Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_ascii_uppercase();
                    e.path().is_file() && (n.starts_with("FICHES") || n.starts_with("FICHESS"))
                })
                .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    v
}

impl MyApp {
    // Verzeichnis festlegen: in der Config merken und die Liste neu scannen.
    fn select_folder(&mut self, dir: &str) {
        self.folder = dir.to_string();
        self.scan_requested = true;
        let msg = config_write(&self.config_path, "dateien_ordner", dir);
        self.status.push(msg);
    }

    // Die config.ini ERNEUT lesen (30.10. auf Nutzerwunsch): hat der Nutzer die
    // Datei von Hand geaendert, wird der neue DATEIEN-Pfad ohne Programmneustart
    // uebernommen. Ohne config.ini oder bei unveraendertem Wert bleibt alles so.
    // Gibt true zurueck, wenn ein neuer Pfad uebernommen wurde.
    fn ordner_aus_config_uebernehmen(&mut self) -> bool {
        match config_read(&self.config_path, "dateien_ordner") {
            Some(neu) if neu != self.folder => {
                self.folder = neu;
                self.status.push(format!(
                    "Ordner aus config.ini uebernommen: {}",
                    self.folder
                ));
                true
            }
            _ => false,
        }
    }

    // Auswahlfenster oeffnen: zeigt den aktuellen Ordner und seine Unterordner.
    fn open_picker(&mut self) {
        let base = if self.folder.is_empty() {
            PathBuf::from(DEFAULT_FOLDER)
        } else {
            PathBuf::from(&self.folder)
        };
        self.picker_dir = base;
        self.refresh_picker();
        self.picker_open = true;
    }

    // Unterordner, FICHES-Dateien und Anzeigetext des Ordners nachladen.
    fn refresh_picker(&mut self) {
        self.picker_entries = list_subdirs(&self.picker_dir);
        self.picker_files = list_fiches_in(&self.picker_dir);
        self.picker_dir_text = self.picker_dir.display().to_string();
    }

    // Verzeichnisauswahl als eigenes Fenster: ohne zusaetzliches Crate gibt es
    // keinen nativen Windows-Dialog, also werden Unterordner angeklickt.
    // Uebernehmen merkt den Ordner in der Config und scannt ihn sofort.
    fn draw_folder_picker(&mut self, ctx: &egui::Context) {
        let mut ziel: Option<String> = None; // ueberzunehmender Pfad (Text)
        let mut schliessen = false; // Abbrechen oder Escape
        let mut hoch = false;
        let mut hinunter: Option<PathBuf> = None;
        egui::Window::new("DATEIEN-Ordner auswaehlen")
            .id(egui::Id::new("dateien_picker"))
            .resizable(false)
            .collapsible(false)
            .default_pos([260.0, 140.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("<hoch").on_hover_text("eine Ebene zurueck").clicked() {
                        hoch = true;
                    }
                    // Der Pfad ist auch editierbar; Enter uebernimmt den Text.
                    let te = egui::TextEdit::singleline(&mut self.picker_dir_text)
                        .desired_width(400.0)
                        .hint_text("Pfad eintippen und Enter druecken");
                    let re = ui.add(te);
                    if re.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        ziel = Some(self.picker_dir_text.clone());
                    }
                });
                if !self.picker_note.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), self.picker_note.clone());
                }
                if self.picker_files.is_empty() {
                    ui.label("in diesem Ordner: keine FICHES-Kartendateien");
                } else {
                    let namen: Vec<&str> =
                        self.picker_files.iter().map(|(n, _)| n.as_str()).collect();
                    ui.label(format!("in diesem Ordner: Kartendateien {}", namen.join(", ")));
                }
                ui.separator();
                ui.label("Unterordner (anklicken = hinuntersteigen):");
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.picker_entries.is_empty() {
                            ui.label("(keine Unterordner)");
                        }
                        for (name, pfad) in &self.picker_entries {
                            if ui.selectable_label(false, name).clicked() {
                                hinunter = Some(pfad.clone());
                            }
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("Uebernehmen und scannen").strong())
                        .on_hover_text("Ordner wird in config.ini gemerkt und sofort neu gescannt")
                        .clicked()
                    {
                        ziel = Some(self.picker_dir_text.clone());
                    }
                    if ui.button("Abbrechen").clicked() {
                        schliessen = true;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    schliessen = true;
                }
                // Navigation erst am Schluss: sie ueberschreibt den Anzeigetext.
                if let Some(p) = hinunter {
                    self.picker_dir = p;
                    self.picker_note.clear();
                    self.refresh_picker();
                }
                if hoch {
                    if let Some(p) = self.picker_dir.parent() {
                        self.picker_dir = p.to_path_buf();
                        self.picker_note.clear();
                        self.refresh_picker();
                    }
                }
            });
        // Pfadpruefung ausserhalb des Closures: Ziel muss ein Ordner sein.
        if let Some(d) = ziel {
            let d = d.trim_matches('"').trim().to_string();
            if !d.is_empty() && PathBuf::from(&d).is_dir() {
                self.select_folder(&d);
                self.picker_open = false;
                return;
            }
            self.picker_note = format!("Kein Ordner (oder nicht lesbar): {}", d);
        }
        if schliessen {
            self.picker_open = false;
        }
    }

    // Datei-Menue -> "Drucken...": Windows-Druckerliste abfragen, den
    // Standarddrucker vorwaehlen und das Druckfenster oeffnen.
    fn open_print(&mut self) {
        self.print_drucker_liste = print::drucker_liste();
        self.print_standard = print::standard_drucker().unwrap_or_default();
        self.print_drucker = if self.print_drucker_liste.iter().any(|n| *n == self.print_standard) {
            self.print_standard.clone()
        } else {
            self.print_drucker_liste.first().cloned().unwrap_or_default()
        };
        self.print_note.clear();
        self.print_open = true;
    }

    // Die Karten des geladenen Datensatzes in druckfertige Datensaetze
    // uebersetzen (Anzeigetexte, Eltern, Ehepartner mit Heiratsdatum und
    // Kinder in Anzeigeposition).
    fn print_personen(&self) -> Vec<print::PrintPerson> {
        let karten = &self.cards;
        karten
            .iter()
            .map(|k| {
                let elternteil = |w: u16| -> String {
                    if w == 0 {
                        return String::new();
                    }
                    karten
                        .iter()
                        .find(|p| p.b == w)
                        .map(|p| format!("{} (B{})", decode_cp1252(&p.name), p.b))
                        .unwrap_or_else(|| format!("B{} (fehlt)", w))
                };
                // Ehen (L2): v ist der Schluessel des Partners, das Datum sitzt
                // im rohen Block. Mehrfachehen hintereinander werden aneinandergereiht.
                let ehen: Vec<String> = k
                    .l2
                    .iter()
                    .filter(|e| e.v != 0)
                    .map(|e| {
                        let partner = karten
                            .iter()
                            .find(|p| p.b == e.v)
                            .map(|p| format!("{} (B{})", decode_cp1252(&p.name), e.v))
                            .unwrap_or_else(|| format!("B{} (fehlt)", e.v));
                        match fmt_date(&e.raw) {
                            d if d.is_empty() => partner,
                            d => format!("{} verh. {}", partner, d),
                        }
                    })
                    .collect();
                // Kinder: die L1-Liste der Karte, zusaetzlich alle Karten, die
                // diese Person ueber w1/w2 als Elternteil fuehren (manche
                // Datensaetze pflegen die Liste nicht auf beiden Elternseiten).
                // Die Eintraege mit Anzeigeposition 0 haengen sich hinten an.
                let mut kinder: Vec<(u16, u16)> = k.l1.clone();
                if k.b != 0 {
                    for p in karten.iter() {
                        if p.w1 == k.b || p.w2 == k.b {
                            kinder.push((p.b, 0));
                        }
                    }
                }
                // Zwei Sortierschritte: erst nach Schluessel entdoppeln (dedup
                // entfernt nur benachbarte Wiederholungen; 0 gilt als bekannte
                // Position des anderen Elternteils), danach stabil nach
                // Anzeigeposition ordnen.
                kinder.sort_by_key(|(s, pos)| (*s, if *pos == 0 { u16::MAX } else { *pos }));
                kinder.dedup_by_key(|(s, _)| *s);
                kinder.sort_by_key(|(_, pos)| if *pos == 0 { u16::MAX } else { *pos });
                let kinder: Vec<String> = kinder
                    .iter()
                    .map(|(s, _)| {
                        karten
                            .iter()
                            .find(|p| p.b == *s)
                            .map(|p| format!("{} (B{})", decode_cp1252(&p.name), p.b))
                            .unwrap_or_else(|| format!("B{} (fehlt)", s))
                    })
                    .collect();
                print::PrintPerson {
                    name: decode_cp1252(&k.name),
                    b: k.b,
                    maennlich: k.sex != b'F',
                    geburt: fmt_date(&k.dt[0].raw),
                    tod: fmt_date(&k.dt[2].raw),
                    vater: elternteil(k.w1),
                    mutter: elternteil(k.w2),
                    ehen,
                    kinder,
                }
            })
            .collect()
    }

    // Das Druckfenster: Drucker waehlen (Combo), Seitenzahl anzeigen, dann
    // auf "Auf Drucker drucken" uebergeben. Fehler lassen das Fenster offen.
    fn draw_print_dialog(&mut self, ctx: &egui::Context) {
        let personen = self.cards.len();
        let seiten = (personen + print::PERSONEN_PRO_SEITE - 1) / print::PERSONEN_PRO_SEITE;
        let mut drucken = false;
        let mut neu_zaehlen = false;
        egui::Window::new("Drucken — alle Mitglieder")
            .id(egui::Id::new("druck_dialog"))
            .resizable(false)
            .collapsible(false)
            .default_pos([300.0, 180.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} Mitglieder — {} Seite(n) zu je 8 Personen (DIN A4, 4 Reihen zu 2 Spalten).",
                    personen, seiten
                ));
                ui.horizontal(|ui| {
                    ui.label("Drucker:");
                    let anzeige = if self.print_drucker.is_empty() {
                        "— bitte waehlen —".to_string()
                    } else {
                        self.print_drucker.clone()
                    };
                    egui::ComboBox::from_id_source("cbdrucker")
                        .selected_text(anzeige)
                        .width(320.0)
                        .show_ui(ui, |ui| {
                            for n in &self.print_drucker_liste {
                                ui.selectable_value(&mut self.print_drucker, n.clone(), n);
                            }
                        });
                    if ui
                        .button("Liste neu laden")
                        .on_hover_text("Windows-Druckerliste erneut abfragen")
                        .clicked()
                    {
                        neu_zaehlen = true;
                    }
                });
                if !self.print_standard.is_empty() {
                    ui.label(format!(
                        "Standarddrucker von Windows: {} (im Feld oben umstellbar)",
                        self.print_standard
                    ));
                }
                if self.print_drucker_liste.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 60, 60),
                        "keine Drucker gefunden — ist in Windows ein Drucker eingerichtet?",
                    );
                }
                if !self.print_note.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), self.print_note.clone());
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let bereit = !self.print_drucker.is_empty() && personen > 0;
                    if ui
                        .add_enabled(
                            bereit,
                            egui::Button::new(egui::RichText::new("Auf Drucker drucken").strong()),
                        )
                        .on_hover_text("Druckauftrag an den gewaehlten Drucker uebergeben")
                        .clicked()
                    {
                        drucken = true;
                    }
                    if ui.button("Abbrechen").clicked() {
                        self.print_open = false;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.print_open = false;
                }
            });
        if neu_zaehlen {
            self.print_drucker_liste = print::drucker_liste();
            self.print_standard = print::standard_drucker().unwrap_or_default();
            if !self.print_drucker_liste.iter().any(|n| *n == self.print_drucker) {
                self.print_drucker = self.print_drucker_liste.first().cloned().unwrap_or_default();
            }
        }
        // Der eigentliche Druck laeuft ausserhalb des Fensters, damit die
        // Windows-Aufrufe das Dialog-Update nicht aufhalten koennen.
        if drucken && self.print_open {
            let personen = self.print_personen();
            let seiten = print::seiten_aufteilung(&personen, print::PERSONEN_PRO_SEITE);
            match print::drucke_personen(&self.print_drucker, &seiten) {
                Ok(m) => {
                    self.status.push(m);
                    self.print_open = false;
                }
                Err(m) => self.print_note = m,
            }
        }
    }

    // Menuepunkt Stammbaum: zum Baumreiter wechseln, die in der Mitgliederliste
    // markierte Person als Wurzel nehmen, Richtung/Tiefe setzen und neu bauen.
    fn zeige_stammbaum(&mut self, mode: Mode, gens: usize) {
        if self.loaded.is_none() {
            return;
        }
        self.tab = self.ev_tabs.len() + 1;
        if self.selected < self.cards.len() {
            self.tree_sel = self.selected;
        }
        self.tree_mode = mode;
        self.tree_gens = gens;
        self.rebuild_tree();
    }

    fn do_scan(&mut self) {
        self.scan_requested = false;
        match scan_folder(std::path::Path::new(&self.folder)) {
            Ok(v) => {
                self.status
                    .push(format!("{} Datensatz-Datei(en) in {} gefunden", v.len(), self.folder));
                self.datasets = v;
            }
            Err(e) => {
                self.status.push(e);
                self.datasets.clear();
            }
        }
    }

    fn load_dataset(&mut self, ds: &DatasetInfo) {
        match std::fs::read(&ds.fiches) {
            Ok(bytes) => match parse_file(&bytes) {
                Ok(cards) => {
                    self.status.push(format!(
                        "Gelesen: {} — {} Karten ({} Byte){}",
                        ds.fiches.display(),
                        cards.len(),
                        bytes.len(),
                        ds.dossier_count
                            .map(|c| format!(", DOSSIER-Zaehler {}", c))
                            .unwrap_or_default()
                    ));
                    // Die Ueberschrift der rechten Anzeige nennt nur noch den
                    // Baumnamen (28.10. auf Nutzerwunsch, ohne "FICHES.XXX");
                    // fehlt der Name im DOSSIER-Kopf, bleibt der Dateiname als Notbehelf.
                    self.loaded_title = if ds.tree.is_empty() {
                        ds.file_name.clone()
                    } else {
                        ds.tree.clone()
                    };
                    self.loaded = Some(ds.fiches.clone());
                    self.cards = cards;
                    self.dirty = false;
                    self.selected = 0;
                    self.form_for = usize::MAX;
                    self.filter.clear();
                    self.confirm_save = false;
                    self.confirm_del = false;
                    // Geschwister-Tafeln (Geburt, Taufe, Heirat, Tod ...) derselben Erweiterung.
                    let dir = ds
                        .fiches
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."));
                    let (tabs, probs) = load_event_tables(&dir, &ds.ext);
                    let akt: usize = tabs.iter().map(|t| t.used_count()).sum();
                    self.status.push(format!(
                        "Ereignistafeln: {} Tafel(n), {} aktive Saetze — als Reiter sichtbar (nur Ansicht).",
                        tabs.len(),
                        akt
                    ));
                    self.ev_tabs = tabs;
                    self.ev_problems = probs;
                    self.tab = 0;
                    self.ev_selected = 0;
                    self.ev_filter.clear();
                    // Stammbaum an den neuen Datensatz anhaengen.
                    self.tree_view = TreeView::blank();
                    self.tree_sel = 0;
                    self.tree_filter.clear();
                    self.tree_fit = true;
                }
                Err(e) => self.status.push(format!("Lesen {} gescheitert: {}", ds.fiches.display(), e)),
            },
            Err(e) => self.status.push(format!("Oeffnen {} fehlgeschlagen: {}", ds.fiches.display(), e)),
        }
    }

    fn add_card(&mut self) {
        let nb = self.cards.iter().map(|c| c.b).max().map_or(0, |m| m + 1);
        self.cards.push(Card::new(nb));
        self.selected = self.cards.len() - 1;
        self.dirty = true;
        self.status.push(format!("Neue Karte mit B={} angelegt — noch ungespeichert", nb));
    }

    // Karte loeschen und dabei die Beziehungen der Karte mit bereinigen.
    // 30.10. auf ausdruecklichen Nutzerwunsch: bis dahin wurde nur die Karte
    // aus der Liste genommen, alle L1-/L2-Beziehungen und Elternfaecher
    // blieben als Haengerei stehen ("wie im Original") — in den Elternkarten
    // stand dann "B82 (fehlt in diesem Datensatz)", und der Eintrag war
    // zunechst auch nicht mehr zu entfernen. LOESCH-ALT (alte Meldung):
    // "Karte B={} ('{}') entfernt — Referenzen auf B={} bleiben als Haengerei
    //  erhalten (wie im Original). Noch ungespeichert."
    fn delete_card(&mut self) {
        if self.selected < self.cards.len() {
            let k = self.cards.remove(self.selected);
            let bereinigung = beziehungen_entfernen(&mut self.cards, k.b);
            self.status.push(format!(
                "Karte B={} ('{}') entfernt — noch ungespeichert.",
                k.b,
                short_name(&k)
            ));
            if bereinigung.is_empty() {
                self.status.push(format!(
                    "Keine Beziehungen zu B{} im Rest des Datensatzes gefunden.",
                    k.b
                ));
            }
            for m in bereinigung {
                self.status.push(m);
            }
            if self.selected >= self.cards.len() {
                self.selected = self.cards.len().saturating_sub(1);
            }
            self.dirty = true;
            self.form_for = usize::MAX;
        }
        self.confirm_del = false;
    }

    fn do_save(&mut self) {
        self.confirm_save = false;
        let path = match &self.loaded {
            Some(p) => p.clone(),
            None => return,
        };
        // Formular zuerst anwenden, sonst geht die letzte Bearbeitung verloren.
        if self.form_for == self.selected && self.selected < self.cards.len() {
            if let Err(e) = apply_form(&mut self.cards[self.selected], &self.form) {
                self.status.push(format!("Formular-Fehler, NICHT gespeichert: {}", e));
                return;
            }
        }
        match save_cards(&path, &self.cards) {
            Ok(rep) => {
                self.status.push(format!(
                    "GESPEICHERT: {} Karten ({} Byte) -> {}",
                    rep.cards, rep.bytes, path.display()
                ));
                self.status.push(format!("Sicherungskopie: {}", rep.backup.display()));
                if let Some(d) = rep.dossier {
                    self.status.push(d);
                }
                self.dirty = false;
            }
            Err(e) => self.status.push(format!("FELER beim Speichern: {}", e)),
        }
    }

    // Eine Ereignistafel als reine Lese-Ansicht: Kopfinfo, Satzliste, Detailbereich.
    // Der Ereignis-Schreiber ist nicht portiert — hier wird nichts geaendert.
    fn draw_event_tab(&mut self, ui: &mut egui::Ui, ti: usize) {
        let t = self.ev_tabs[ti].clone(); // lokale Kopie: Entleihe-Konflikte in Closure vermeiden
        let fname = t
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        ui.colored_label(
            egui::Color32::from_rgb(180, 120, 0),
            "Schreibgeschuetzte Ansicht — der Ereignis-Schreiber ist noch nicht portiert.",
        );
        if !t.model_fits {
            ui.colored_label(
                egui::Color32::from_rgb(200, 60, 60),
                format!(
                    "Modellpasse fehlt: {} Byte passen nicht glatt zur Satzlaenge {} — die Saetze koennen nicht sicher angezeigt werden.",
                    t.file_len, t.recsize
                ),
            );
            return;
        }
        ui.label(format!(
            "Datei {} — {} Byte · Satzlaenge {} · {} Saetze im Modell · Kapazitaetswort {} · Kopf-Zaehler {}",
            fname, t.file_len, t.recsize, t.n, t.capacity, t.counter
        ));
        if t.counter as usize != t.used_count() {
            ui.colored_label(
                egui::Color32::from_rgb(180, 120, 0),
                format!(
                    "Kopf-Zaehler {} weicht von {} belegten Saetzen ab — wie im Original ueblich (Zaehler veraltet); die Anzeige nutzt die Belegung selbst.",
                    t.counter,
                    t.used_count()
                ),
            );
        }

        // Personenbezug: Saetze, die auf die aktuell angewaehlte Karte (Schluessel B) zeigen.
        let cur_b = self.cards.get(self.selected).map(|c| c.b).unwrap_or(0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.ev_filter)
                    .desired_width(160.0)
                    .hint_text("Filter Text/Schluessel"),
            );
            ui.checkbox(&mut self.ev_show_free, "auch freie Saetze zeigen");
            ui.checkbox(&mut self.ev_tech, "Textfunde zeigen");
            if cur_b != 0 {
                ui.weak(format!("blau markiert = Bezug zur Karte B={}", cur_b));
            }
        });
        let filter = self.ev_filter.trim().to_lowercase();
        let show_free = self.ev_show_free;
        let mut sel_click: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for r in t.recs.iter() {
                    if !r.used && !show_free {
                        continue;
                    }
                    let bez = cur_b != 0
                        && (r.ia == cur_b
                            || r.ib == cur_b
                            || r.key == Some(cur_b)
                            || r.key2 == Some(cur_b));
                    let mut line = format!(
                        "#{}  {}  (IndexA {} / IndexB {})",
                        r.idx,
                        ev_summary(r),
                        r.ia,
                        r.ib
                    );
                    if let (Some(k), Some(k2)) = (r.key, r.key2) {
                        line.push_str(&format!("  [Schluessel {}/{}]", k, k2));
                    } else if let Some(k) = r.key {
                        line.push_str(&format!("  [Schluessel {}]", k));
                    }
                    if !r.used {
                        line.push_str("  [frei]");
                    }
                    if bez {
                        line.push_str("  << Bezug");
                    }
                    if !filter.is_empty() && !line.to_lowercase().contains(&filter) {
                        continue;
                    }
                    let txt = if bez {
                        egui::RichText::new(line).color(egui::Color32::from_rgb(70, 140, 220))
                    } else {
                        egui::RichText::new(line)
                    };
                    if ui.selectable_label(self.ev_selected == r.idx, txt).clicked() {
                        sel_click = Some(r.idx);
                    }
                }
            });
        if let Some(i) = sel_click {
            self.ev_selected = i;
        }

        ui.separator();
        match t.recs.iter().find(|r| r.idx == self.ev_selected) {
            None => {
                ui.label("Kein Satz ausgewaehlt — oben in der Liste einen Satz anklicken.");
            }
            Some(r) => {
                let stxt = if r.status == 0 {
                    "leer".to_string()
                } else if (0x20..0x7F).contains(&r.status) {
                    format!("'{}'", r.status as char)
                } else {
                    format!("0x{:02X}", r.status)
                };
                ui.strong(format!(
                    "Satz #{} — Dateioffset {} · IndexA {} · IndexB {} · Status {}",
                    r.idx, r.off, r.ia, r.ib, stxt
                ));
                if !r.used {
                    ui.label("(freier Satz — steht auf der Freiliste, Inhalt ist Altlast)");
                }
                if !r.fields.is_empty() {
                    egui::Grid::new("ev_felder")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            for (l, v) in &r.fields {
                                ui.weak(*l);
                                ui.label(v);
                                ui.end_row();
                            }
                        });
                }
                let unmapped =
                    matches!(t.kind, EvKind::Bapt | EvKind::Bene | EvKind::Sepu | EvKind::Fremd);
                if r.used && (unmapped || self.ev_tech) {
                    if unmapped {
                        ui.label(format!(
                            "Feldkarten fuer diese Tafel noch nicht erstellt — Hilfssicht: gefundene Textlaeufe ({} Fundstellen).",
                            r.strings.len()
                        ));
                    } else {
                        ui.strong("Textlaeufe im Satz (Offset relativ zum Satzanfang):");
                    }
                    egui::Grid::new("ev_texts")
                        .num_columns(2)
                        .spacing([10.0, 3.0])
                        .show(ui, |ui| {
                            for (off, s) in r.strings.iter() {
                                ui.label(format!("+{}", off));
                                ui.label(s.clone());
                                ui.end_row();
                            }
                        });
                }
            }
        }
    }

    // ---------- Stammbaum: Eltern-Kind-Beziehungen als Graph (Reiter, Lese-Ansicht) ----------

    // Zeile fuer die Wurzel-Auswahl.
    fn tree_root_line(&self) -> String {
        if self.tree_sel < self.cards.len() {
            let k = &self.cards[self.tree_sel];
            format!("B{} \u{2014} {}", k.b, short_name(k))
        } else {
            "(Person waehlen)".to_string()
        }
    }

    // Beziehungsindex und Layout werden nur bei einer Auswahl/Aktion aufgebaut,
    // nie pro Bildaufbau (Kinder-/Paarsuche ist O(n) ueber alle Karten).
    fn rebuild_tree(&mut self) {
        if self.cards.is_empty() {
            self.tree_view = TreeView::blank();
            return;
        }
        if self.tree_sel >= self.cards.len() {
            self.tree_sel = 0;
        }
        if self.tree_gens == 0 {
            self.tree_gens = 3;
        }
        let rel = Rel::build(&self.cards);
        let v = build_tree(&self.cards, &rel, self.tree_sel, self.tree_mode, self.tree_gens);
        let info = v.summary();
        self.tree_view = v;
        // Nach einer Auswahl moeglichst nah an der Person bleiben: das Einpassen
        // endet bei Zoom 0.7 (Namen bleiben lesbar), "Einpassen" zeigt alles.
        self.tree_fit = true;
        self.tree_fit_min = 0.7;
        self.status.push(format!("Stammbaum: {}", info));
    }

    fn set_tree_depth(&mut self, g: usize) {
        let g = if g >= ALL_GENS { ALL_GENS } else { g.clamp(1, 12) };
        if g != self.tree_gens {
            self.tree_gens = g;
            self.rebuild_tree();
        }
    }

    fn zoom_tree(&mut self, factor: f32, rect: egui::Rect, at: egui::Pos2) {
        let old = self.tree_scale;
        if old <= 0.0 {
            return;
        }
        let ns = (old * factor).clamp(0.06, 4.0);
        if (ns - old).abs() < 0.0005 {
            return;
        }
        // Punkt unter der Maus bleibt an derselben Stelle auf dem Bildschirm.
        let p = (at - rect.min - self.tree_off) / old;
        self.tree_scale = ns;
        self.tree_off += (at - rect.min - self.tree_off) - p * ns;
        self.tree_fit = false;
    }

    fn draw_tree_tab(&mut self, ui: &mut egui::Ui) {
        if self.cards.is_empty() {
            ui.label("Kein Datensatz geladen.");
            return;
        }
        if self.tree_scale <= 0.0 {
            self.tree_scale = 1.0;
        }
        if self.tree_fit_min <= 0.0 {
            self.tree_fit_min = 0.7;
        }
        if self.tree_view.is_empty() {
            self.rebuild_tree();
        }

        // ---------- Steuerung: Wurzel waehlen, Richtung, Tiefe ----------
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("Nachfolgende Generationen").strong())
                .on_hover_text("alle Nachkommen der gewaehlten Person, abwaerts gezeichnet")
                .clicked()
            {
                self.tree_mode = Mode::Desc;
                self.tree_gens = ALL_GENS;
                self.rebuild_tree();
            }
            if ui
                .button(egui::RichText::new("Vorhergehende 3 Familien").strong())
                .on_hover_text("Aufwaerts: Eltern, Grosseltern, Urgrosseltern (drei Familien-Ebenen)")
                .clicked()
            {
                self.tree_mode = Mode::Anc;
                self.tree_gens = 3;
                self.rebuild_tree();
            }
            ui.separator();
            ui.label("Tiefe:");
            if ui.button("-").clicked() {
                let g = if self.tree_gens >= ALL_GENS { 3 } else { self.tree_gens - 1 };
                self.set_tree_depth(g);
            }
            ui.label(if self.tree_gens >= ALL_GENS {
                "alle".to_string()
            } else {
                self.tree_gens.to_string()
            });
            if ui.button("+").clicked() {
                let g = if self.tree_gens >= ALL_GENS { ALL_GENS } else { self.tree_gens + 1 };
                self.set_tree_depth(g);
            }
            if ui.button("alle").clicked() {
                self.set_tree_depth(ALL_GENS);
            }
            if ui.button("Erneuern").clicked() {
                // nach einer Bearbeitung am Formular: Ansicht neu berechnen
                self.rebuild_tree();
            }
        });
        let q = self.tree_filter.trim().to_lowercase();
        let opts: Vec<(usize, String)> = self
            .cards
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                q.is_empty()
                    || format!("b{} {}", k.b, short_name(k)).to_lowercase().contains(&q)
            })
            .map(|(i, k)| (i, format!("B{} \u{2014} {}", k.b, short_name(k))))
            .collect();
        ui.horizontal(|ui| {
            ui.label("Mittelperson:");
            let before = self.tree_sel;
            egui::ComboBox::from_id_source("baum_wurzel")
                .selected_text(self.tree_root_line())
                .width(400.0)
                .show_ui(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            if opts.is_empty() {
                                ui.weak("keine passende Person");
                            }
                            for (i, lbl) in &opts {
                                ui.selectable_value(&mut self.tree_sel, *i, lbl.clone());
                            }
                        });
                });
            if self.tree_sel != before {
                self.selected = self.tree_sel;
                self.form_for = usize::MAX;
                self.rebuild_tree();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.tree_filter)
                    .hint_text("Name oder B suchen")
                    .desired_width(170.0),
            );
            ui.label(format!("{} Treffer", opts.len()));
        });
        ui.horizontal(|ui| {
            ui.label(self.tree_view.summary());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Einpassen")
                    .on_hover_text("den ganzen Baum zeigen (bei vielen Generationen sehr klein)")
                    .clicked()
                {
                    self.tree_fit_min = 0.06;
                    self.tree_fit = true;
                }
                if ui.button("Zoom -").clicked() {
                    let r = self
                        .tree_rect
                        .unwrap_or_else(|| ui.available_rect_before_wrap());
                    self.zoom_tree(1.0 / 1.25, r, r.center());
                }
                if ui.button("Zoom +").clicked() {
                    let r = self
                        .tree_rect
                        .unwrap_or_else(|| ui.available_rect_before_wrap());
                    self.zoom_tree(1.25, r, r.center());
                }
            });
        });
        ui.horizontal(|ui| {
            ui.weak(
                "Ziehen: verschieben \u{b7} Strg+Rad oder Zoom: vergroessern \u{b7} Klick auf eine Person: neuer Mittelpunkt \u{b7} Farben:",
            );
            for (txt, c) in [
                ("Ehe / Eltern (flacher Balken)", egui::Color32::from_rgb(230, 238, 229)),
                ("Koenigshaus", egui::Color32::from_rgb(246, 231, 183)),
                ("maennlich", egui::Color32::from_rgb(222, 235, 247)),
                ("weiblich", egui::Color32::from_rgb(250, 226, 231)),
                ("eingeheiratet", egui::Color32::from_rgb(238, 238, 238)),
                ("siehe oben", egui::Color32::from_rgb(232, 232, 232)),
            ] {
                let (r, p) = ui.allocate_painter(egui::vec2(13.0, 11.0), egui::Sense::hover());
                p.rect_filled(r.rect.shrink(1.0), 2.0, c);
                ui.label(txt);
            }
        });
        ui.separator();

        // ---------- Zeichenflaeche ----------
        let (resp, mut painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = resp.rect;
        self.tree_rect = Some(rect);
        painter.set_clip_rect(rect);
        if self.tree_fit && !self.tree_view.is_empty() {
            let (mnx, mxx, mny, mxy) = (
                self.tree_view.min_x,
                self.tree_view.max_x,
                self.tree_view.min_y,
                self.tree_view.max_y,
            );
            let sx = (rect.width() - 30.0) / self.tree_view.width();
            let sy = (rect.height() - 30.0) / self.tree_view.height();
            let voll = sx.min(sy);
            // Nie ueber die normale Boxgroesse hinaus zoomen (sonst wirken die
            // Kaesten riesig); beim automatischen Einpassen auch nicht unter die
            // Lesbarkeitsgrenze: dann bleibt ein Ausschnitt um die Mittelpunkt-Person.
            let s = voll.clamp(self.tree_fit_min, 1.0);
            self.tree_scale = s;
            if s > voll + 0.0001 {
                let root = self.tree_view.root;
                let (rx, ry) = (self.tree_view.nodes[root].x, self.tree_view.nodes[root].y);
                let rc = self.tree_view.nodes[root].card;
                let fy = if self.tree_view.mode == Mode::Desc {
                    0.28 // Nachkommen wachsen nach unten
                } else {
                    0.72 // Vorfahren wachsen nach oben
                };
                self.tree_off = egui::vec2(rect.width() * 0.5 - rx * s, rect.height() * fy - ry * s);
                let b = self.cards.get(rc).map_or(0, |k| k.b);
                self.status.push(format!(
                    "Stammbaum: Ausschnitt um B{} (Zoom {:.2}) \u{b7} \"Einpassen\" zeigt alles",
                    b, s
                ));
            } else {
                self.tree_off = egui::vec2(
                    (rect.width() - (mnx + mxx) * s) * 0.5,
                    (rect.height() - (mny + mxy) * s) * 0.5,
                );
            }
            self.tree_fit = false;
        }
        if resp.dragged() {
            self.tree_off += resp.drag_delta();
            self.tree_fit = false;
        }
        if resp.contains_pointer() {
            let (z, sd) = ui.ctx().input(|i| (i.zoom_delta(), i.raw_scroll_delta));
            if (z - 1.0).abs() > 0.001 {
                let mp = resp.hover_pos().unwrap_or(rect.center());
                self.zoom_tree(z, rect, mp);
            } else if sd.length_sq() > 0.5 {
                self.tree_off += sd;
                self.tree_fit = false;
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                let lx = (p.x - rect.min.x - self.tree_off.x) / self.tree_scale;
                let ly = (p.y - rect.min.y - self.tree_off.y) / self.tree_scale;
                let pick = self.tree_view.hit(lx, ly);
                if let Some(i) = pick {
                    if self.tree_view.nodes[i].kind == Kind::Person {
                        let c = self.tree_view.nodes[i].card;
                        if c != self.tree_sel {
                            self.tree_sel = c;
                            self.selected = c;
                            self.form_for = usize::MAX;
                            self.rebuild_tree();
                        }
                    }
                }
            }
        }

        let t = &self.tree_view;
        let s = self.tree_scale;
        let o = self.tree_off;
        let tp = |x: f32, y: f32| -> egui::Pos2 {
            egui::pos2(rect.min.x + o.x + x * s, rect.min.y + o.y + y * s)
        };
        let mut hover: Option<usize> = None;
        if let Some(p) = resp.hover_pos() {
            let lx = (p.x - rect.min.x - o.x) / s;
            let ly = (p.y - rect.min.y - o.y) / s;
            hover = t.hit(lx, ly);
        }
        // Kanten zuerst, die Boxes ueberdecken ihre Enden.
        let thick = egui::Stroke::new((1.3 * s).clamp(0.6, 3.0), egui::Color32::from_rgb(122, 118, 108));
        let thin = egui::Stroke::new((1.0 * s).clamp(0.5, 2.0), egui::Color32::from_rgb(150, 146, 138));
        for n in &t.nodes {
            if n.kind == Kind::Person {
                // duenne Linie = Ehe: kurz und wagerecht zwischen den Partnern,
                // nicht diagonal durch den Familien-Kasten hindurch
                for &f in &n.kids {
                    let sp = t.nodes[f].spouse;
                    if sp != usize::MAX {
                        let s2 = &t.nodes[sp];
                        painter.add(egui::Shape::line(vec![tp(n.x, n.y), tp(s2.x, s2.y)], thin));
                    }
                }
            }
            for &k in &n.kids {
                let a = &t.nodes[k];
                painter.add(egui::Shape::line(vec![tp(n.x, n.y), tp(a.x, a.y)], thick));
            }
        }
        for (i, n) in t.nodes.iter().enumerate() {
            let r = egui::Rect::from_center_size(tp(n.x, n.y), egui::vec2(n.w * s, n.h * s));
            let (fill, line) = if n.kind == Kind::Fam {
                // salbgruen, damit der flache Familien-Balken nie wie ein
                // (leerer) Personen-Kasten aussieht
                (
                    egui::Color32::from_rgb(230, 238, 229),
                    egui::Color32::from_rgb(140, 158, 138),
                )
            } else if n.dup {
                (
                    egui::Color32::from_rgb(232, 232, 232),
                    egui::Color32::from_rgb(150, 150, 150),
                )
            } else if n.royal {
                (
                    egui::Color32::from_rgb(246, 231, 183),
                    egui::Color32::from_rgb(176, 146, 60),
                )
            } else if n.sex == b'F' {
                (
                    egui::Color32::from_rgb(250, 226, 231),
                    egui::Color32::from_rgb(190, 140, 150),
                )
            } else if n.sex == b'H' {
                (
                    egui::Color32::from_rgb(222, 235, 247),
                    egui::Color32::from_rgb(130, 160, 190),
                )
            } else {
                (
                    egui::Color32::from_rgb(238, 238, 238),
                    egui::Color32::from_rgb(160, 160, 160),
                )
            };
            let lw = if i == t.root {
                2.6
            } else if n.kind == Kind::Person && !n.ghost {
                1.2
            } else {
                0.8
            };
            let rad = if n.kind == Kind::Fam { 3.0 } else { 5.0 };
            painter.rect_filled(r, rad, fill);
            painter.rect_stroke(
                r,
                rad,
                egui::Stroke::new((lw * s.max(0.6)).clamp(0.8, 3.2), line),
            );
            if hover == Some(i) {
                painter.rect_stroke(
                    r.expand(2.5),
                    7.0,
                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(40, 95, 170)),
                );
            }
            if n.kind == Kind::Person && n.more {
                painter.circle_filled(
                    r.right_top() + egui::vec2(-4.0, 4.0),
                    (5.0 * s).clamp(2.5, 9.0),
                    egui::Color32::from_rgb(210, 150, 40),
                );
            }
            if r.width() < 20.0 || r.height() < 9.0 {
                continue; // zu klein fuer Text
            }
            if n.kind == Kind::Fam {
                // Familien-Balken: immer beschriftet ("Ehe" / "Eltern"), die
                // zweite Zeile (Kinderzahl, Heiratsdatum) nur wenn Platz ist
                let zwei = r.height() >= 15.0 && !n.sub.is_empty();
                painter.text(
                    egui::pos2(r.center().x, r.center().y - if zwei { 2.0 * s } else { 0.0 }),
                    if zwei {
                        egui::Align2::CENTER_BOTTOM
                    } else {
                        egui::Align2::CENTER_CENTER
                    },
                    n.label.clone(),
                    egui::FontId::proportional((9.5 * s).clamp(5.5, 20.0)),
                    egui::Color32::from_rgb(66, 86, 64),
                );
                if zwei {
                    painter.text(
                        egui::pos2(r.center().x, r.center().y + 2.0 * s),
                        egui::Align2::CENTER_TOP,
                        n.sub.clone(),
                        egui::FontId::proportional((7.8 * s).clamp(4.5, 16.0)),
                        egui::Color32::from_rgb(92, 96, 88),
                    );
                }
            } else {
                let big = (11.5 * s).clamp(5.5, 24.0);
                let small = (8.5 * s).clamp(4.5, 18.0);
                let col = if n.dup {
                    egui::Color32::from_rgb(110, 110, 110)
                } else {
                    egui::Color32::from_rgb(28, 28, 28)
                };
                let name = n.label.clone();
                if r.height() >= 21.0 && !n.sub.is_empty() {
                    painter.text(
                        egui::pos2(r.center().x, r.center().y - 2.0 * s),
                        egui::Align2::CENTER_BOTTOM,
                        name,
                        egui::FontId::proportional(big),
                        col,
                    );
                    painter.text(
                        egui::pos2(r.center().x, r.center().y + 2.0 * s),
                        egui::Align2::CENTER_TOP,
                        n.sub.clone(),
                        egui::FontId::proportional(small),
                        egui::Color32::from_rgb(95, 92, 86),
                    );
                } else {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(big), col);
                }
            }
        }
        if let Some(i) = hover {
            let n = &t.nodes[i];
            let txt = if n.kind == Kind::Fam {
                format!(
                    "{}-Marke \u{b7} {} \u{b7} {} Anschlusszweige",
                    n.label,
                    if n.sub.is_empty() { "keine Angabe".to_string() } else { n.sub.clone() },
                    n.kids.len()
                )
            } else {
                let b = self.cards.get(n.card).map_or(0, |c| c.b);
                format!(
                    "B{} \u{b7} {} \u{b7} {} \u{b7} Klick = neuer Mittelpunkt",
                    b, n.label, n.sub
                )
            };
            painter.text(
                rect.left_top() + egui::vec2(8.0, 6.0),
                egui::Align2::LEFT_TOP,
                txt,
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(60, 60, 60),
            );
        }
    }
}

impl MyApp {
    // Ganze Oberflaeche in einem eigenen methode: update() ist nur noch der
    // eframe-Zugang, damit Tests die Menues mit ctx.run() durchspielen koennen.
    fn draw_ui(&mut self, ctx: &egui::Context) {
        // SCHLIESSEN-ALT (bis 30.10., 3. Runde): X-Knopf und der Menuepunkt
        // "Beenden" schlossen das Programm ohne weiteres — ungespeicherte
        // Aenderungen waren lautlos fort. Neue Regel (Nutzerwunsch): bei
        // ungespeicherten Aenderungen wird der Schliessbefehl zuerst
        // abgefangen (Antwort CancelClose, siehe eframe-Doku zu
        // ViewportCommand::CancelClose) und nach Sichern oder Verwerfen
        // gefragt; ohne Aenderungen schliesst das Programm wie bisher direkt.
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_quit = true;
            self.status.push("Schliessen aufgehoben — zuerst sichern oder verwerfen.".to_string());
        }
        // Beim ersten Anlauf: Configpfad festlegen und den gemerkten Ordner
        // laden; ohne Config-Datei bleibt der Standardordner C:\ahnen\DATEIEN.
        if self.config_path.as_os_str().is_empty() {
            self.config_path = match &self.cfg_dir_override {
                Some(d) => d.join(CONFIG_FILE),
                None => default_config_path(),
            };
        }
        if self.folder.is_empty() {
            self.folder = config_read(&self.config_path, "dateien_ordner")
                .unwrap_or_else(|| DEFAULT_FOLDER.to_string());
            self.scan_requested = true;
        }
        if self.scan_requested {
            self.do_scan();
        }

        // ---------- Menueleiste: Datei | Karteikarte | Stammbaum ----------
        // Die fruehere Auswahlzeile oben (Ordnerfeld + Presets) entfaellt;
        // alles laeuft ueber dieses Menu bzw. das Verzeichnisauswahlfenster.
        // Die Datensatzauswahl (frueher oben im linken Seitenpanel) sitzt jetzt
        // als Untermenue unter Datei. Der Klick merkt nur die Nummer; geladen
        // wird ausserhalb des Menues, damit das Menue erst sauber zugeht.
        let mut offener_datensatz: Option<usize> = None;
        egui::TopBottomPanel::top("menue").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Datei", |ui| {
                    // Sichtbar machen, welcher Ordner wirklich benutzt wird —
                    // der Wert kommt aus der config.ini bzw. der letzten Auswahl.
                    ui.label(format!("Aktueller Ordner: {}", self.folder));
                    ui.separator();
                    if ui.button("Verzeichnis auswaehlen...").clicked() {
                        self.open_picker();
                        ui.close_menu();
                    }
                    if ui
                        .button("Ordner neu scannen")
                        .on_hover_text("config.ini erneut lesen: ein von Hand geaenderter Pfad wird uebernommen, dann die Liste neu aufgebaut")
                        .clicked()
                    {
                        // 30.10.: zuerst die config.ini erneut lesen, damit eine
                        // von Hand geaenderte Datei ohne Neustart wirkt.
                        self.ordner_aus_config_uebernehmen();
                        self.scan_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    // Untermenue "Datensaetze": die Auswahl wie bisher als
                    // anklickbare Liste, nur eben jetzt innerhalb des Menues.
                    // Der aktuell geladene Datensatz ist angewaehlt.
                    ui.menu_button("Datensaetze", |ui| {
                        if self.datasets.is_empty() {
                            ui.label("keine FICHES-Dateien im Ordner gefunden");
                        }
                        for (i, ds) in self.datasets.iter().enumerate() {
                            // Anzeige (28.10. auf Nutzerwunsch): ohne "FICHES.XXX"
                            // und ohne den Besitzer-Zusatz ("Personal Soft" ...) —
                            // es zaehlt der Baumname aus dem DOSSIER-Kopf; fehlt er,
                            // hilft der Dateiname als Notbehelf.
                            let title = if ds.tree.is_empty() {
                                ds.file_name.clone()
                            } else {
                                ds.tree.clone()
                            };
                            let cnt = ds
                                .dossier_count
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "?".into());
                            let sub = if ds.is_sub { "  [Unterdatei]" } else { "" };
                            let sel = self.loaded.as_deref() == Some(ds.fiches.as_path());
                            let resp = ui.selectable_label(
                                sel,
                                egui::RichText::new(format!(
                                    "{} — {} Karten{}",
                                    title, cnt, sub
                                )),
                            );
                            if resp.clicked() {
                                offener_datensatz = Some(i);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    ui.label("Voreinstellungen:");
                    if ui.button("Original C:\\ahnen\\DATEIEN").clicked() {
                        self.select_folder(DEFAULT_FOLDER);
                        ui.close_menu();
                    }
                    // Die fruheren Zusatz-Voreinstellungen "Kopien" und
                    // "Arbeitskopien" sind auf Wunsch entfallen; zum Erreichen
                    // solcher Ordner dient der Verzeichniswaehler.
                    ui.separator();
                    if ui
                        .button("Drucken...")
                        .on_hover_text(
                            "Alle Mitglieder des Datensatzes (mit Eltern, Ehe und Kindern): je 8 Personen auf einer A4-Seite (4 Reihen zu 2 Spalten); Drucker vorher waehlbar",
                        )
                        .clicked()
                    {
                        self.open_print();
                        ui.close_menu();
                    }
                    if ui
                        .button("Beenden")
                        .on_hover_text(
                            "bei ungespeicherten Aenderungen fragt die App vorher: speichern oder verwerfen?",
                        )
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });
                let karten_da = self.loaded.is_some();
                ui.menu_button("Karteikarte", |ui| {
                    if ui
                        .add_enabled(karten_da, egui::Button::new("Neue Karte anlegen"))
                        .clicked()
                    {
                        self.add_card();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(karten_da, egui::Button::new("Karte loeschen..."))
                        .on_hover_text("Nach Rueckfrage wird die markierte Karte entfernt")
                        .clicked()
                    {
                        self.confirm_del = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Stammbaum", |ui| {
                    if ui
                        .add_enabled(karten_da, egui::Button::new("Nachkommen (alle Generationen)"))
                        .clicked()
                    {
                        self.zeige_stammbaum(Mode::Desc, ALL_GENS);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(karten_da, egui::Button::new("Vorfahren (3 Generationen)"))
                        .clicked()
                    {
                        self.zeige_stammbaum(Mode::Anc, 3);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(karten_da, egui::Button::new("Vorfahren (alle Generationen)"))
                        .clicked()
                    {
                        self.zeige_stammbaum(Mode::Anc, ALL_GENS);
                        ui.close_menu();
                    }
                    ui.separator();
                    let baum_aktiv =
                        self.loaded.is_some() && self.tab == self.ev_tabs.len() + 1;
                    if ui
                        .add_enabled(baum_aktiv, egui::Button::new("Einpassen"))
                        .on_hover_text("Baum so zoomen, dass alles in den sichtbaren Bereich passt")
                        .clicked()
                    {
                        self.tree_fit_min = 0.06;
                        self.tree_fit = true;
                        ui.close_menu();
                    }
                });
            });
        });
        // Der uebers Datei-Menue gewaehlte Datensatz wird hier ausserhalb des
        // Menues geladen (wie zuvor die Liste im linken Seitenpanel).
        if let Some(i) = offener_datensatz {
            let ds = self.datasets[i].clone();
            self.load_dataset(&ds);
        }

        // ---------- Statuszeile ----------
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if self.status.len() > 3 {
                let cut = self.status.len() - 3;
                self.status.drain(0..cut);
            }
            for line in &self.status {
                ui.label(egui::RichText::new(line.clone()).small());
            }
        });

        // ---------- Senkrechte Werkzeugleiste ganz links ----------
        // Die friheren Textknopfe unten im Mitgliederpanel (neue Karte, Karte
        // loeschen, Speichern) stehen jetzt als Bildknoepfe uebereinander ganz
        // links; der Erklaertext erscheint bei Mouseover als Sprechblase.
        // Bestaetigungsfragen (Ja/Nein) erscheinen direkt unter dem Symbol.
        egui::SidePanel::left("ikonleiste")
            .default_width(64.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                let karten_da = self.loaded.is_some();
                let r1 = ikon_knopf(ui, karten_da, "Neue Karte anlegen", malen_neue_karte);
                self.icon_rects.0[0] = r1.rect;
                if karten_da && r1.clicked() {
                    self.add_card();
                }
                ui.add_space(10.0);
                let frage_del = self.confirm_del;
                let hover_del = if frage_del {
                    "Noch einmal klicken (oder Ja unten): Karte wirklich entfernen"
                } else {
                    "Markierte Karte loeschen (Nachfrage folgt)"
                };
                let r2 = ikon_knopf(ui, karten_da, hover_del, malen_karte_loeschen);
                self.icon_rects.0[1] = r2.rect;
                if karten_da && r2.clicked() {
                    if frage_del {
                        self.delete_card();
                    } else {
                        self.confirm_del = true;
                    }
                }
                if frage_del {
                    ui.horizontal(|ui| {
                        if ui.small_button("Ja").clicked() {
                            self.delete_card();
                        }
                        if ui.small_button("Nein").clicked() {
                            self.confirm_del = false;
                        }
                    });
                }
                ui.add_space(10.0);
                let frage_sp = self.confirm_save;
                let hover_sp = if !self.dirty {
                    "Speichern — im Moment nichts ungespeichert"
                } else if frage_sp {
                    "Noch einmal klicken (oder Ja unten): erst Sicherungskopie, dann Datei schreiben"
                } else {
                    "Alle Aenderungen speichern (Sicherungskopie zuerst)"
                };
                let r3 = ikon_knopf(ui, self.dirty, hover_sp, malen_speichern);
                self.icon_rects.0[2] = r3.rect;
                if self.dirty {
                    // Kleiner roter Punkt: es warten ungespeicherte Aenderungen.
                    ui.painter().circle_filled(
                        r3.rect.max - egui::vec2(6.0, 5.0),
                        3.5,
                        egui::Color32::from_rgb(210, 60, 60),
                    );
                }
                if self.dirty && r3.clicked() {
                    if frage_sp {
                        self.do_save();
                    } else {
                        self.confirm_save = true;
                    }
                }
                if frage_sp {
                    ui.horizontal(|ui| {
                        if ui.small_button("Ja").clicked() {
                            self.do_save();
                        }
                        if ui.small_button("Nein").clicked() {
                            self.confirm_save = false;
                        }
                    });
                }
            });

        // ---------- Links: Mitgliederliste ----------
        // Die Datensatz-Auswahl ist ins Menue Datei -> Datensaetze umgezogen.
        egui::SidePanel::left("links")
            .default_width(340.0)
            .resizable(true)
            .show(ctx, |ui| {
                if self.loaded.is_none() {
                    ui.label("Erst im Menue Datei -> Datensaetze einen Datensatz waehlen.");
                    return;
                }

                ui.horizontal(|ui| {
                    ui.heading(format!("Mitglieder ({})", self.cards.len()));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .desired_width(120.0)
                            .hint_text("Filter Name/B"),
                    );
                });
                let filter = self.filter.trim().to_lowercase();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (i, k) in self.cards.iter().enumerate() {
                        let name = short_name(k);
                        let geb = fmt_date(&k.dt[0].raw);
                        let tod = fmt_date(&k.dt[2].raw);
                        let line = format!(
                            "B{} {} {}   {}–{}",
                            k.b,
                            if k.sex == b'F' { "w" } else { "m" },
                            name,
                            if geb.is_empty() { "?" } else { &geb },
                            if tod.is_empty() { "?" } else { &tod }
                        );
                        if !filter.is_empty()
                            && !line.to_lowercase().contains(&filter)
                        {
                            continue;
                        }
                        if ui.selectable_label(self.selected == i, line).clicked() {
                            self.selected = i;
                        }
                    }
                });

                ui.separator();
                // Die Knopfe selbst stehen jetzt in der senkrechten Leiste
                // ganz links; hier bleibt nur der Hinweis auf offene Aenderungen.
                if self.dirty {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 60, 60),
                        "Aenderungen ungespeichert",
                    );
                }
            });

        // ---------- Rechts: Detailformular ----------
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.loaded.is_none() {
                ui.heading("Racines-Kartei");
                ui.label(
                    "Im Menu Datei -> \"Verzeichnis auswaehlen...\" den DATEIEN-Ordner waehlen, \
                     Datensatz im Menue Datei -> Datensaetze anklicken, Mitglied aussuchen, rechts \
                     bearbeiten. Beim \
                     Speichern wird immer zuerst eine Sicherungskopie \
                     (.bak / .bak~1..9) angelegt, danach Datei und DOSSIER-Zaehler geschrieben.",
                );
                return;
            }
            ui.heading(self.loaded_title.clone());

            // ---------- Reiter: Karten + eine Tafel pro Ereignistabelle + Stammbaum ----------
            let tree_tab = self.ev_tabs.len() + 1;
            if self.tab > tree_tab {
                self.tab = 0;
                self.ev_selected = 0;
            }
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.tab == 0, egui::RichText::new("Karten").strong())
                    .clicked()
                {
                    self.tab = 0;
                }
                // Auf Nutzerwunsch (28.10.) zeigt die Reiterzeile nur noch
                // "Karten" und "Stammbaum". Die Knöpfe der Ereignistafeln
                // (Geburt/Taufe/Heirat/Tod ...) sind nur auskommentiert —
                // zwischen REITER-AUS-KOMMENTAR-ANFANG und -ENDE wieder
                // entkommentieren, der Rest (Inhalte, Zaehlung, Stammbaum-
                // Reiter, Laden der Tafeln) ist unveraendert geblieben.
                // REITER-AUS-KOMMENTAR-ANFANG
                /*
                for (i, t) in self.ev_tabs.iter().enumerate() {
                    let lbl = format!("{} ({} aktiv)", t.title, t.used_count());
                    if ui.selectable_label(self.tab == i + 1, lbl).clicked() {
                        self.tab = i + 1;
                        self.ev_selected = 0;
                    }
                }
                */
                // REITER-AUS-KOMMENTAR-ENDE
                if ui
                    .selectable_label(self.tab == tree_tab, egui::RichText::new("Stammbaum").strong())
                    .clicked()
                {
                    self.tab = tree_tab;
                    // Die in der Mitgliederliste markierte Person wird die Wurzel.
                    if self.selected < self.cards.len() {
                        self.tree_sel = self.selected;
                    }
                    self.tree_fit = true;
                    self.rebuild_tree();
                }
                // REITER-AUS-KOMMENTAR-ANFANG (gleicher Block wie oben)
                /*
                if self.ev_tabs.is_empty() {
                    ui.weak("(keine Ereignistafeln in diesem Ordner gefunden)");
                }
                */
                // REITER-AUS-KOMMENTAR-ENDE
            });
            if self.tab != 0 && self.tab < tree_tab {
                for p in self.ev_problems.clone() {
                    ui.colored_label(egui::Color32::from_rgb(180, 120, 0), p);
                }
            }
            if self.tab != 0 {
                let ti = self.tab - 1;
                if ti >= self.ev_tabs.len() {
                    self.draw_tree_tab(ui);
                } else {
                    self.draw_event_tab(ui, ti);
                }
                return;
            }
            ui.separator();

            if self.selected >= self.cards.len() {
                self.selected = self.cards.len().saturating_sub(1);
                self.form_for = usize::MAX;
            }
            if self.form_for != self.selected {
                self.form = load_form(&self.cards[self.selected]);
                self.form_for = self.selected;
                self.kind_neu = "0".to_string();
                self.heirat_neu = "0".to_string();
                self.ehen_dat =
                    self.cards[self.selected].l2.iter().map(|e| fmt_date(&e.raw)).collect();
            }
            // Arbeitskopie der Karte: entkoppelt die Lese-Anzeige vom Formular-Closure.
            let card = self.cards[self.selected].clone();
            let b = card.b;
            ui.label(format!(
                "Karte {} von {} — Personalschluessel B={} (unveraenderlich, Referenzen bleiben), Vater-Schluessel={}, Mutter-Schluessel={}",
                self.selected + 1,
                self.cards.len(),
                b,
                card.w1,
                card.w2
            ));
            ui.separator();

            let parent_opts: Vec<(u16, String)> = self
                .cards
                .iter()
                .map(|c| (c.b, short_name(c)))
                .collect();
            let f = &mut self.form;
            let mut apply_clicked = false;
            // Kinder-Operationen werden wie das Anwenden erst NACH dem Closure
            // ausgefuehrt; im Closure steht nur die Auswahl (Schluessel B).
            let mut kind_weg: Option<u16> = None;
            let mut kind_add: Option<u16> = None;
            // Gleiches Muster fuer die Ehepartner-Operationen (L2).
            let mut heirat_weg: Option<u16> = None;
            let mut heirat_add: Option<u16> = None;
            // Zeile, deren Datumsfeld mit "Datum setzen" ubernommen wurde.
            let mut ehen_dat_anwenden: Option<usize> = None;

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("kopfg").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(420.0));
                    ui.label("Geschlecht:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut f.sex, 'H', "Maennlich (H)");
                        ui.selectable_value(&mut f.sex, 'F', "Weiblich (F)");
                    });
                    ui.end_row();

                    ui.label("A-Wert:");
                    ui.add(egui::TextEdit::singleline(&mut f.a).desired_width(70.0));
                    ui.label("C-Wert:");
                    ui.add(egui::TextEdit::singleline(&mut f.c).desired_width(70.0));
                    ui.end_row();

                    ui.label("Vater:");
                    let lv = parent_opts
                        .iter()
                        .find(|(bb, _)| bb.to_string() == f.vater.trim())
                        .map(|(_, n)| format!("B{} {}", f.vater.trim(), n))
                        .unwrap_or_else(|| f.vater.clone());
                    egui::ComboBox::from_id_source("cbvater")
                        .selected_text(lv)
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut f.vater, "0".to_string(), "— ohne —");
                            for (bb, n) in &parent_opts {
                                if *bb != b {
                                    ui.selectable_value(
                                        &mut f.vater,
                                        bb.to_string(),
                                        format!("B{} {}", bb, n),
                                    );
                                }
                            }
                        });
                    ui.label("Mutter:");
                    let lm = parent_opts
                        .iter()
                        .find(|(bb, _)| bb.to_string() == f.mutter.trim())
                        .map(|(_, n)| format!("B{} {}", f.mutter.trim(), n))
                        .unwrap_or_else(|| f.mutter.clone());
                    egui::ComboBox::from_id_source("cbmutter")
                        .selected_text(lm)
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut f.mutter, "0".to_string(), "— ohne —");
                            for (bb, n) in &parent_opts {
                                if *bb != b {
                                    ui.selectable_value(
                                        &mut f.mutter,
                                        bb.to_string(),
                                        format!("B{} {}", bb, n),
                                    );
                                }
                            }
                        });
                    ui.end_row();
                });

                ui.add_space(6.0);
                // Ein Raster, drei Spalten: Geburt, Heirat, Tod. Seit 28.10.
                // (Nutzerwunsch) steht in jeder Spalte unter der Ueberschrift
                // das Datumsfeld mit Vorspann "Datum:" und darunter das
                // Ortsfeld mit Vorspann "Ort:". Alle drei Datumsfelder
                // bekommen dieselbe Breite — egui dehnt ein Textfeld sonst
                // passend zu seinem Inhalt.
                egui::Grid::new("dtg")
					.num_columns(3)
					.spacing([10.0, 6.0])
					.min_col_width(150.0)
					.show(ui, |ui| {
                    let ueberschriften = ["Geburt", "Heirat", "Tod"];
                    for i in 0..3 {
                        ui.vertical(|ui| {
                            ui.strong(ueberschriften[i]);
                            ui.horizontal(|ui| {
                                ui.label("Datum:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut f.dt[i].datum)
                                        .desired_width(110.0)
                                        .hint_text("TT.MM.JJJJ"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Ort:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut f.dt[i].ort)
                                        .desired_width(160.0),
                                );
                            });
                        });
                    }
                    ui.end_row();
                });

                ui.add_space(6.0);
                ui.strong("Zusatzzeilen (s1..s5)");
                egui::Grid::new("sg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    for i in 0..5 {
                        ui.label(format!("s{}:", i + 1));
                        ui.add(egui::TextEdit::singleline(&mut f.s[i]).desired_width(560.0));
                        ui.end_row();
                    }
                });

                ui.add_space(6.0);
                // ----- Kinder-Beziehungen (L1): bearbeiten -----
                // Jede Zeile zeigt ein Kind dieser Karte; "entfernen" traegt es
                // aus (L1-Paar fort + Elternfach im Kind geleert). Das Auswahlfeld
                // bietet alle Karten an, die weder diese Karte selbst noch schon
                // Kind sind; "Kind hinzuuegen" setzt zusaetzlich im Kind das zum
                // Geschlecht dieser Karte passende Elternfach (vater=w1, sonst w2).
                ui.strong("Kinder-Beziehungen (L1)");
                let kinder: Vec<(u16, u16)> = card.l1.clone();
                if kinder.is_empty() {
                    ui.weak("noch keine Kinder eingetragen");
                }
                for (x, _y) in &kinder {
                    let name = parent_opts
                        .iter()
                        .find(|(bb, _)| bb == x)
                        .map(|(_, n)| n.clone())
                        .unwrap_or_else(|| "(fehlt in diesem Datensatz)".to_string());
                    ui.horizontal(|ui| {
                        ui.label(format!("B{}  {}", x, name));
                        if ui.small_button("entfernen").clicked() {
                            kind_weg = Some(*x);
                        }
                    });
                }
                let kandidaten: Vec<(u16, String)> = parent_opts
                    .iter()
                    .filter(|(bb, _)| *bb != 0 && *bb != b && !kinder.iter().any(|(x, _)| x == bb))
                    .cloned()
                    .collect();
                ui.horizontal(|ui| {
                    ui.label("Kind hinzuwaehlen:");
                    let kt = self.kind_neu.clone();
                    let anzeige = kandidaten
                        .iter()
                        .find(|(bb, _)| bb.to_string() == kt.trim())
                        .map(|(bb, n)| format!("B{} {}", bb, n))
                        .unwrap_or_else(|| "— bitte waehlen —".to_string());
                    egui::ComboBox::from_id_source("cbkindneu")
                        .selected_text(anzeige)
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            for (bb, n) in &kandidaten {
                                ui.selectable_value(
                                    &mut self.kind_neu,
                                    bb.to_string(),
                                    format!("B{} {}", bb, n),
                                );
                            }
                        });
                    if ui.button("Kind hinzuuegen").clicked() {
                        kind_add = kt
                            .trim()
                            .parse::<u16>()
                            .ok()
                            .filter(|v| *v != 0);
                    }
                });

                ui.add_space(6.0);
                // ----- Heirat (L2): bearbeiten -----
                // In L2-Ereignissen ist v der B-Schluessel des Ehepartners
                // (bestaetigt an FICHES.006: Karte B2 traegt v=5 auf B5, Karte
                // B5 traegt v=2 auf B2 — NAMEN-ALT: fruehere echten Namen sind
                // bewusst entfernt). Analog zu den Kindern: "entfernen" traegt die Ehe
                // aus BEIDEN Karten aus, "Heirat hinzuuegen" setzt den Eintrag in
                // beide Karten. Der 12-Byte-Block traegt (wie die Datumsgruppen)
                // das Heiratsdatum; 0xFFFF bzw. lauter Nullen heisst unbekannt.
                // Das Datum laesst sich in der Zeile direkt aendern — derselbe
                // Text wie im Formular (TT.MM.JJJJ, JJJJ oder "-" = unbekannt);
                // q-Wort und Tabellenzeiger im Block bleiben bitgenau erhalten.
                ui.strong("Heirat (L2)");
                let ehen: Vec<Ev> = card.l2.clone();
                if self.ehen_dat.len() != ehen.len() {
                    // Anzahl geaendert (Kartenwechsel, anwenden der L2-
                    // Operationen): Datums-Puffer neu aus den Rohblöcken holen,
                    // sonst bleibt die laufende Tipparbeit in den Feldern stehen.
                    self.ehen_dat = ehen.iter().map(|e| fmt_date(&e.raw)).collect();
                }
                if ehen.is_empty() {
                    ui.weak("keine Heirat eingetragen");
                }
                for (i, e) in ehen.iter().enumerate() {
                    let partner = parent_opts
                        .iter()
                        .find(|(bb, _)| *bb == e.v && *bb != 0)
                        .map(|(_, n)| n.clone())
                        .unwrap_or_else(|| "(fehlt in diesem Datensatz)".to_string());
                    let txt = decode_cp1252(&e.txt);
                    let txt = if txt.is_empty() {
                        String::new()
                    } else {
                        format!("   Text: \"{}\"", txt)
                    };
                    ui.horizontal(|ui| {
                        ui.label(format!("B{}  {}{}", e.v, partner, txt));
                        if i < self.ehen_dat.len() {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ehen_dat[i])
                                    .desired_width(110.0)
                                    .hint_text("(Datum unbekannt)"),
                            );
                        }
                        if ui.small_button("Datum setzen").clicked() {
                            ehen_dat_anwenden = Some(i);
                        }
                        if e.v != 0 && ui.small_button("entfernen").clicked() {
                            heirat_weg = Some(e.v);
                        }
                    });
                }
                // Kandidaten: alle anderen Karten, die nicht schon als Partner
                // dieser Karte eingetragen sind.
                let partner_kandidaten: Vec<(u16, String)> = parent_opts
                    .iter()
                    .filter(|(bb, _)| {
                        *bb != 0 && *bb != b && !ehen.iter().any(|e| e.v == *bb)
                    })
                    .cloned()
                    .collect();
                ui.horizontal(|ui| {
                    ui.label("Ehepartner hinzuwaehlen:");
                    let ht = self.heirat_neu.clone();
                    let anzeige = partner_kandidaten
                        .iter()
                        .find(|(bb, _)| bb.to_string() == ht.trim())
                        .map(|(bb, n)| format!("B{} {}", bb, n))
                        .unwrap_or_else(|| "— bitte waehlen —".to_string());
                    egui::ComboBox::from_id_source("cbheiratneu")
                        .selected_text(anzeige)
                        .width(300.0)
                        .show_ui(ui, |ui| {
                            for (bb, n) in &partner_kandidaten {
                                ui.selectable_value(
                                    &mut self.heirat_neu,
                                    bb.to_string(),
                                    format!("B{} {}", bb, n),
                                );
                            }
                        });
                    if ui.button("Heirat hinzuuegen").clicked() {
                        heirat_add = ht
                            .trim()
                            .parse::<u16>()
                            .ok()
                            .filter(|v| *v != 0);
                    }
                });

                ui.add_space(6.0);
                ui.strong("Weitere Tabellen (nur Ansicht)");
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Ereignisse (L2): {} · Quellen (L3): {}",
                        card.l2.len(),
                        card.l3.len()
                    ));
                    ui.checkbox(&mut self.show_tech, "Details");
                });
                if self.show_tech {
                    let name_of = |bb: u16| {
                        parent_opts
                            .iter()
                            .find(|(x, _)| *x == bb)
                            .map(|(_, n)| n.clone())
                            .unwrap_or_else(|| "(fehlend)".to_string())
                    };
                    for (x, y) in &card.l1 {
                        ui.label(format!(
                            "  L1: {} <-> {}  ({}, {})",
                            x,
                            y,
                            name_of(*x),
                            name_of(*y)
                        ));
                    }
                    for e in &card.l2 {
                        ui.label(format!(
                            "  L2: v={}  {}  \"{}\"  a={} b={}",
                            e.v,
                            fmt_date(&e.raw),
                            decode_cp1252(&e.txt),
                            e.a,
                            e.b
                        ));
                    }
                    for (bezeichnung, datei) in &card.l3 {
                        ui.label(format!(
                            "  L3: {} = {}",
                            decode_cp1252(bezeichnung),
                            decode_cp1252(datei)
                        ));
                    }
                }

                ui.add_space(8.0);
                if ui.button("Aenderungen in die Liste uebernehmen").clicked() {
                    apply_clicked = true;
                }
            });

            // Anwenden erst nach dem Closure, damit kein Konflikt mit der Formularanleihe entsteht.
            if apply_clicked {
                let idx = self.selected;
                match apply_form(&mut self.cards[idx], &self.form) {
                    Ok(()) => {
                        // Elternschluessel geaendert? Dann die Kinderlisten (L1)
                        // der betroffenen Elternkarten mitziehen lassen.
                        let (kb, wv, wm) = {
                            let k = &self.cards[idx];
                            (k.b, k.w1, k.w2)
                        };
                        l1_aktualisieren(&mut self.cards, kb, wv, wm);
                        self.dirty = true;
                        self.status.push(format!(
                            "Karte B={} uebernommen (noch ungespeichert)",
                            self.cards[idx].b
                        ));
                    }
                    Err(e) => self.status.push(format!("Formular-Fehler: {}", e)),
                }
            }
            if let Some(kb) = kind_weg {
                match l1_entfernen(&mut self.cards, b, kb) {
                    Ok(m) => {
                        self.dirty = true;
                        self.status.push(m);
                    }
                    Err(m) => self.status.push(format!("L1-Fehler: {}", m)),
                }
            }
            if let Some(kb) = kind_add {
                match l1_add(&mut self.cards, b, kb) {
                    Ok(m) => {
                        self.dirty = true;
                        self.status.push(m);
                        self.kind_neu = "0".to_string();
                    }
                    Err(m) => self.status.push(format!("L1-Fehler: {}", m)),
                }
            }
            if let Some(pb) = heirat_weg {
                match l2_heirat_entfernen(&mut self.cards, b, pb) {
                    Ok(m) => {
                        self.dirty = true;
                        self.status.push(m);
                    }
                    Err(m) => self.status.push(format!("L2-Fehler: {}", m)),
                }
            }
            if let Some(pb) = heirat_add {
                match l2_heirat_add(&mut self.cards, b, pb) {
                    Ok(m) => {
                        self.dirty = true;
                        self.status.push(m);
                        self.heirat_neu = "0".to_string();
                    }
                    Err(m) => self.status.push(format!("L2-Fehler: {}", m)),
                }
            }
            if let Some(i) = ehen_dat_anwenden {
                // Das Datumsfeld einer Heiratszeile in den 12-Byte-Block des
                // Ereignisses schreiben; q-Wort und Tabellenzeiger bleiben
                // erhalten (set_date wie im Formular). Leer wie "-" verstehen.
                let idx = self.selected;
                if i < self.cards[idx].l2.len() {
                    let want = self.ehen_dat.get(i).cloned().unwrap_or_default();
                    let want = want.trim().to_string();
                    let mut raw = self.cards[idx].l2[i].raw;
                    if set_date(&mut raw, if want.is_empty() { "-" } else { &want }) {
                        if raw != self.cards[idx].l2[i].raw {
                            self.cards[idx].l2[i].raw = raw;
                            self.ehen_dat[i] = fmt_date(&raw);
                            self.dirty = true;
                            self.status.push(format!(
                                "Heiratsdatum in Karte B{} (Ehepartner B{}) geaendert: '{}'",
                                b,
                                self.cards[idx].l2[i].v,
                                fmt_date(&raw)
                            ));
                        }
                    } else {
                        self.status.push(format!(
                            "L2-Fehler: Datum '{}' — erwartet TT.MM.JJJJ, JJJJ oder '-'",
                            want
                        ));
                    }
                }
            }
        });

        // Verzeichnisauswahl als eigenes Fenster, ganz zum Schluss gezeichnet,
        // damit seine Klicks nicht auf den Listen dahinter landen.
        if self.picker_open {
            self.draw_folder_picker(ctx);
        }
        // Dasselbe Muster fuer das Druckfenster (Datei -> "Drucken...").
        if self.print_open {
            self.draw_print_dialog(ctx);
        }
        // Und fuer die Schliess-Rueckfrage (Marker SCHLIESSEN-ALT, siehe Anfang
        // von draw_ui): Frage mittig ueber dem Hauptfenster.
        if self.confirm_quit {
            self.draw_beenden_frage(ctx);
        }
    }

    // Frage beim Schliessen bei ungespeicherten Aenderungen (Nutzerwunsch
    // 30.10., 3. Runde). "Speichern und beenden" sichert erst (mit
    // Sicherungskopie); schlaegt die Sicherung fehl, bleibt die Frage offen
    // und die Fehlermeldung steht in der Statuszeile. "Verwerfen und beenden"
    // wirft die Aenderungen weg; "Abbrechen" laesst alles beim Alten.
    fn draw_beenden_frage(&mut self, ctx: &egui::Context) {
        egui::Window::new("Ungespeicherte Aenderungen")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Das Programm soll geschlossen werden, aber der");
                ui.label("Datensatz hat noch ungespeicherte Aenderungen.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Speichern und beenden")
                        .on_hover_text("erst sichern (mit Sicherungskopie), dann schliessen")
                        .clicked()
                    {
                        self.do_save();
                        if !self.dirty {
                            self.confirm_quit = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    if ui
                        .button("Verwerfen und beenden")
                        .on_hover_text("Aenderungen wegwerfen und schliessen — ohne zu speichern")
                        .clicked()
                    {
                        self.dirty = false;
                        self.status.push("Ungespeicherte Aenderungen verworfen.".to_string());
                        self.confirm_quit = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .button("Abbrechen")
                        .on_hover_text("das Fenster bleibt offen, nichts wird geaendert")
                        .clicked()
                    {
                        self.confirm_quit = false;
                    }
                });
            });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw_ui(ctx);
    }
}

// ---------------- Tests ----------------
// Der Reiter-Inhalt laesst sich ohne Fenster durchlaufen: egui erzeugt
// Formen (Kasten, Linien, Text), damit Pruefungen die Grafik sehen koennen.

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, b: u16, sex: u8, w1: u16, w2: u16) -> Card {
        let mut c = Card::new(b);
        c.name = name.as_bytes().to_vec();
        c.sex = sex;
        c.w1 = w1;
        c.w2 = w2;
        c
    }

    // Grosseltern (B 1,2) -> Vater 3 + Mutter 4 -> Kinder 5 und 6.
    fn family_app() -> MyApp {
        let mut app = MyApp {
            cards: vec![
                person("Grossvater", 1, b'H', 0, 0),
                person("Grossmutter", 2, b'F', 0, 0),
                person("Vater", 3, b'H', 1, 2),
                person("Mutter", 4, b'F', 0, 0),
                person("Kind A", 5, b'H', 3, 4),
                person("Kind B", 6, b'F', 3, 4),
            ],
            ..Default::default()
        };
        app.tree_sel = 2; // Vater
        app.tree_mode = Mode::Desc;
        app.tree_gens = ALL_GENS;
        app.rebuild_tree();
        app
    }

    fn eingabe() -> egui::RawInput {
        let mut ri = egui::RawInput::default();
        ri.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        ));
        ri
    }

    // Ein Bild des Reiters, ohne Fenster: egui liefert die gezeichneten Formen.
    fn zeichne(app: &mut MyApp, ri: egui::RawInput) -> egui::FullOutput {
        zeichne_mit(&egui::Context::default(), app, ri)
    }

    fn zeichne_mit(
        ctx: &egui::Context,
        app: &mut MyApp,
        ri: egui::RawInput,
    ) -> egui::FullOutput {
        ctx.run(ri, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.draw_tree_tab(ui);
            });
        })
    }

    // Zahl der (Kasten, Linien, Texte), die der Baum auf der Zeichenflaeche
    // malt: Kaesten in den Baumfarben und Kastengroesse (die Legende zeichnet
    // dieselben Farben, aber nur gut 13x11 Pixel), Linien als zweipunktiger
    // Pfad in den Kantenfarben, Texte innerhalb der Zeichenflaeche.
    fn baum_formen(out: &egui::FullOutput, flaeche: egui::Rect) -> (usize, usize, usize) {
        const FUEELLUNGEN: [(u8, u8, u8); 6] = [
            (230, 238, 229), // Familien-Balken (Ehe/Eltern)
            (222, 235, 247), // maennlich
            (250, 226, 231), // weiblich
            (246, 231, 183), // Koenigshaus
            (238, 238, 238), // unbekanntes Geschlecht
            (232, 232, 232), // "siehe oben"
        ];
        const KANTEN: [(u8, u8, u8); 2] = [(122, 118, 108), (150, 146, 138)];
        let farbe = |c: egui::Color32, liste: &[(u8, u8, u8)]| {
            liste.iter()
                .any(|t| c.r() == t.0 && c.g() == t.1 && c.b() == t.2)
        };
        let mut kasten = 0;
        let mut striche = 0;
        let mut texte = 0;
        for c in &out.shapes {
            match &c.shape {
                egui::Shape::Rect(r) => {
                    if farbe(r.fill, &FUEELLUNGEN)
                        && r.rect.width() >= 40.0
                        && r.rect.height() >= 20.0
                    {
                        kasten += 1;
                    }
                }
                egui::Shape::Path(p) => {
                    if p.fill == egui::Color32::TRANSPARENT
                        && p.points.len() == 2
                        && farbe(p.stroke.color, &KANTEN)
                    {
                        striche += 1;
                    }
                }
                egui::Shape::Text(t) => {
                    if flaeche.contains(t.pos) {
                        texte += 1;
                    }
                }
                _ => {}
            }
        }
        (kasten, striche, texte)
    }

    #[test]
    fn stammbaum_malt_kasten_und_linien() {
        let mut app = family_app();
        assert_eq!(app.tree_view.person_count(), 4, "Vater, Mutter, 2 Kinder");
        let out = zeichne(&mut app, eingabe());
        assert!(!app.tree_fit, "Einpassen wurde durchgefuehrt");
        assert!(
            app.tree_scale <= 1.0 + 1e-6,
            "automatisches Einpassen vergroessert nicht ueber die Normalgroesse: {}",
            app.tree_scale
        );
        let flaeche = app.tree_rect.expect("Zeichenflaeche bekannt");
        let (kasten, linien, texte) = baum_formen(&out, flaeche);
        assert_eq!(
            kasten,
            5,
            "4 Personen- und 1 Familienkasten, bekommen: {kasten}"
        );
        assert_eq!(
            linien,
            4,
            "3 Eltern-Kind-Kanten plus Partner-Linie, bekommen: {linien}"
        );
        assert!(
            texte >= app.tree_view.nodes.len(),
            "jeder Kasten traegt einen Namen, bekommen: {texte} von {} Kasten",
            app.tree_view.nodes.len()
        );
    }

    // Klick auf den Kasten eines Kindes macht dieses Kind zur Mittelperson.
    #[test]
    fn klick_auf_kasten_setzt_neue_wurzel() {
        let mut app = family_app();
        let ctx = egui::Context::default();
        let _ = zeichne_mit(&ctx, &mut app, eingabe()); // 1. Bildaufbau: Einpassen
        let (s, o, rect) = (
            app.tree_scale,
            app.tree_off,
            app.tree_rect.expect("Zeichenflaeche bekannt"),
        );
        let kind = app
            .tree_view
            .nodes
            .iter()
            .position(|n| n.kind == Kind::Person && n.card == 4)
            .expect("Kasten von Kind A im Baum");
        let n = &app.tree_view.nodes[kind];
        let ziel = egui::pos2(rect.min.x + o.x + n.x * s, rect.min.y + o.y + n.y * s);
        let mut ri = eingabe();
        ri.events = vec![
            egui::Event::PointerMoved(ziel),
            egui::Event::PointerButton {
                pos: ziel,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos: ziel,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        let _ = zeichne_mit(&ctx, &mut app, ri);
        let zweiter = zeichne_mit(&ctx, &mut app, eingabe());
        assert_eq!(app.tree_sel, 4, "Kind A ist Mittelperson");
        assert_eq!(app.tree_view.nodes[app.tree_view.root].card, 4);
        assert_eq!(app.tree_view.mode, Mode::Desc);
        assert!(!zweiter.shapes.is_empty());
    }

    // ---------- Menueleiste, Config-Datei und Verzeichnisauswahl ----------

    // Ganzen Bildschirm zeichnen (Menueleiste + Liste + Reiter), wie im Programm.
    fn zeichne_ganz(ctx: &egui::Context, app: &mut MyApp, ri: egui::RawInput) -> egui::FullOutput {
        ctx.run(ri, |ctx| app.draw_ui(ctx))
    }

    // Fundstellen eines sichtbaren Textes auf dem Bildschirm. Der Galley-Text
    // wird ohne Leerzeichen verglichen, sein Rechteck liegt relativ (bei 0,0)
    // und kommt zur Textposition des Shapes dazu.
    fn text_stellen(out: &egui::FullOutput, suchwort: &str) -> Vec<egui::Rect> {
        let soll: String = suchwort.chars().filter(|c| !c.is_whitespace()).collect();
        let mut treffer = Vec::new();
        for shape in &out.shapes {
            if let egui::Shape::Text(t) = &shape.shape {
                let gesamt: String = t
                    .galley
                    .rows
                    .iter()
                    .flat_map(|r| r.glyphs.iter().map(|g| g.chr))
                    .filter(|c| !c.is_whitespace())
                    .collect();
                if gesamt == soll {
                    // Der Galley-Rechteck liegt relativ (oben links bei 0,0);
                    // zurueckgezogen auf den Ursprung und an die Textstelle geschoben.
                    treffer.push(t.galley.rect.translate(t.pos.to_vec2()));
                }
            }
        }
        treffer
    }

    // Ein Mausklick (hinbewegen, druecken, loslassen) zu einem Rechteck.
    fn klick_stelle(ri: &mut egui::RawInput, r: egui::Rect) {
        ri.events.extend([
            egui::Event::PointerMoved(r.center()),
            egui::Event::PointerButton {
                pos: r.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos: r.center(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ]);
    }

    // Der Text des letzten Menuepunkts in einer Menuespalte, geoeffnet per Klick.
    // Wichtig: egui malt ein neu erzeugtes Area (die Menuespalte) erst ab seinem
    // zweiten Durchlauf — deshalb nach dem Klick noch ein neutraler Durchlauf,
    // dessen Ausgabe die Punkttexte enthaelt.
    fn menuepunkt_oeffnen(
        ctx: &egui::Context,
        app: &mut MyApp,
        menue: &str,
    ) -> egui::FullOutput {
        let aus = zeichne_ganz(ctx, app, eingabe());
        let knopf = text_stellen(&aus, menue)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("Menuepunkt {} fehlt", menue));
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let _ = zeichne_ganz(ctx, app, ri);
        zeichne_ganz(ctx, app, eingabe())
    }

    // App-Rumpf fuer die Menue-Tests: ohne echten Ordner (der Scan laeuft ins
    // Leere) und mit geladenem Datensatz, damit die Punkte nicht graut sind.
    fn menue_app() -> MyApp {
        let mut app = family_app();
        app.folder = "test-tmp\\kein-ordner".to_string();
        app.loaded = Some(PathBuf::from("test-tmp\\test.dat"));
        app
    }

    #[test]
    fn menueleiste_zeigt_drei_menues_ohne_alte_kopfzeile() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        for menue in ["Datei", "Karteikarte", "Stammbaum"] {
            assert!(
                !text_stellen(&aus, menue).is_empty(),
                "Menuepunkt {} fehlt oben",
                menue
            );
        }
        // Die alte Ordnerzeile oben im Fenster gibt es nicht mehr.
        assert!(text_stellen(&aus, "(noch kein Ordner gewaehlt)").is_empty());
    }

    #[test]
    fn datei_menue_offnet_den_verzeichnis_picker() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Datei");
        let punkt = text_stellen(&offen, "Verzeichnis auswaehlen...")
            .into_iter()
            .next()
            .expect("Menuepunkt 'Verzeichnis auswaehlen...' fehlt im Datei-Menue");
        let mut ri = eingabe();
        klick_stelle(&mut ri, punkt);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        // Das Picker-Fenster ist ein neues Area: Titel malt erst ab dem zweiten Durchlauf.
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(app.picker_open, "Picker-Fenster ist nicht offen");
        assert!(
            !text_stellen(&aus, "DATEIEN-Ordner auswaehlen").is_empty(),
            "Fenstertitel des Pickers fehlt"
        );
    }

    // Die Datensatzauswahl ist vom linken Seitenpanel ins Menue Datei ->
    // Datensaetze umgezogen: Eintrag anklicken lädt denselben Weg wie vorher.
    #[test]
    fn datei_menue_datensaetze_laden_einen_datensatz() {
        let mut app = menue_app();
        app.datasets = vec![DatasetInfo {
            ext: ".001".to_string(),
            file_name: "FICHES.001".to_string(),
            is_sub: false,
            fiches: PathBuf::from("..\\racines-port\\data\\FICHES.001"),
            tree: "TESTBAUM".to_string(),
            owner: "Hans".to_string(),
            dossier_count: Some(123),
        }];
        let ctx = egui::Context::default();
        let start = zeichne_ganz(&ctx, &mut app, eingabe());
        // Die fruehere Ueberschrift oben im linken Seitenpanel gibt es nicht mehr.
        assert!(
            text_stellen(&start, "Datensaetze").is_empty(),
            "die Ueberschrift im linken Seitenpanel ist entfernt"
        );
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Datei");
        let unter = text_stellen(&offen, "Datensaetze")
            .into_iter()
            .next()
            .expect("Untermenue 'Datensaetze' fehlt im Datei-Menue");
        let mut ri = eingabe();
        klick_stelle(&mut ri, unter);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        // Wie jedes neue Area malt das Untermenue seine Texte erst ab dem
        // zweiten Durchlauf: erst ein neutraler Durchlauf abwarten.
        let liste = zeichne_ganz(&ctx, &mut app, eingabe());
        // Ohne "FICHES.XXX" und ohne Besitzer-Zusatz: nur Baumname und Zahl.
        let eintrag = text_stellen(&liste, "TESTBAUM — 123 Karten")
            .into_iter()
            .next()
            .expect("Der Datensatz-Eintrag fehlt im Untermenue");
        let mut ri = eingabe();
        klick_stelle(&mut ri, eintrag);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(
            app.loaded.as_deref(),
            Some(PathBuf::from("..\\racines-port\\data\\FICHES.001").as_path()),
            "der angeklickte Datensatz wird ueber das Menue geladen"
        );
        assert!(!app.cards.is_empty(), "der geladene Datensatz enthaelt Karten");
        assert_eq!(app.selected, 0, "nach dem Laden steht die erste Karte");
        assert!(
            app.status.iter().any(|s| s.starts_with("Gelesen:")),
            "die Gelesen-Meldung fehlt in der Statuszeile"
        );
        // Rechte Anzeige: Ueberschrift nur mit Baumname, kein "FICHES.XXX".
        let geladen = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&geladen, "TESTBAUM").is_empty(),
            "die rechte Ueberschrift nennt den Baumnamen"
        );
        assert!(
            text_stellen(&geladen, "FICHES.001").is_empty(),
            "FICHES.XXX steht nicht mehr in der rechten Anzeige"
        );
    }

    #[test]
    fn picker_waehlt_ordner_und_merkt_config() {
        // Testordner bleiben im Paketordner: die Schreibsandbox erlaubt kein
        // System-Schreibverzeichnis.
        let basis = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-tmp-picker");
        let root = basis.join("pickroot");
        let _ = std::fs::remove_dir_all(&basis);
        std::fs::create_dir_all(root.join("DATEIEN")).expect("Testordner anlegen");
        std::fs::write(root.join("DATEIEN").join("FICHES001.DAT"), b"x").expect("Testdatei");
        let mut app = menue_app();
        app.cfg_dir_override = Some(basis.clone());
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // setzt config_path
        app.picker_open = true;
        app.picker_dir = root.clone();
        app.refresh_picker();
        // Das Fenster ist ein neues Area: erster Durchlauf erzeugt es, gemalt
        // wird es erst ab dem zweiten.
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        // Die Menuepunkte unten in der Datei-Auswahl enthalten denselben
        // Wortlaut ("... DATEIEN"); das Picker-Fenster malt als letztes.
        let ordner = text_stellen(&aus, "DATEIEN")
            .into_iter()
            .last()
            .expect("Unterordner DATEIEN nicht angeklickt werden koennen");
        let mut ri = eingabe();
        klick_stelle(&mut ri, ordner);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&aus, "in diesem Ordner: Kartendateien FICHES001.DAT").is_empty(),
            "Die Kartendatei im Ordner DATEIEN wird nicht angezeigt"
        );
        let ueber = text_stellen(&aus, "Uebernehmen und scannen")
            .into_iter()
            .last()
            .expect("Knopf 'Uebernehmen und scannen' fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, ueber);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.picker_open, "Picker hat sich nicht geschlossen");
        assert!(app.folder.ends_with("DATEIEN"), "Ordner ist nun: {}", app.folder);
        let cfg = basis.join(CONFIG_FILE);
        assert_eq!(
            config_read(&cfg, "dateien_ordner").as_deref(),
            Some(app.folder.as_str()),
            "config.ini hat einen anderen Ordner als die App gemerkt"
        );
        let _ = std::fs::remove_dir_all(&basis);
    }

    #[test]
    fn config_lesen_und_schreiben_einwandfrei() {
        let pfad = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-tmp-config.ini");
        let _ = std::fs::remove_file(&pfad);
        assert_eq!(config_read(&pfad, "dateien_ordner"), None, "fehlende Datei");
        let notiz = config_write(&pfad, "dateien_ordner", "C:\\ahnen\\DATEIEN");
        assert!(notiz.starts_with("Gemerkt in"), "Notiz: {}", notiz);
        assert_eq!(
            config_read(&pfad, "dateien_ordner").as_deref(),
            Some("C:\\ahnen\\DATEIEN")
        );
        // Zeilen mit # sind Kommentare, Anfuehrungszeichen am Wert fallen weg.
        std::fs::write(
            &pfad,
            "# Kommentar\nandere = x\ndateien_ordner = \"C:\\mit\\quot\"\n",
        )
        .expect("Testconfig schreiben");
        assert_eq!(
            config_read(&pfad, "dateien_ordner").as_deref(),
            Some("C:\\mit\\quot")
        );
        assert_eq!(config_read(&pfad, "anderer"), None, "fremder Schluessel ignoriert");
        let _ = std::fs::remove_file(&pfad);
    }

    // Von Hand geaenderte config.ini wirkt OHNE Programmneustart: der Punkt
    // "Ordner neu scannen" liest die Datei erneut und uebernimmt den neuen
    // DATEIEN-Pfad samt neuem Scan (Nutzerwunsch am 30.10.).
    #[test]
    fn config_aenderung_wird_bei_neuem_scan_uebernommen() {
        let basis = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-tmp-cfgscan");
        let _ = std::fs::remove_dir_all(&basis);
        let ordner_a = basis.join("ordner-a");
        let ordner_b = basis.join("ordner-b");
        std::fs::create_dir_all(&ordner_a).expect("ordner-a anlegen");
        std::fs::create_dir_all(&ordner_b).expect("ordner-b anlegen");
        std::fs::write(
            basis.join(CONFIG_FILE),
            format!("dateien_ordner = {}\n", ordner_a.display()),
        )
        .expect("config schreiben");
        let mut app = MyApp {
            cfg_dir_override: Some(basis.clone()),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // Configpfad + gemerkter Ordner
        assert_eq!(
            app.folder,
            ordner_a.display().to_string(),
            "der Startordner kommt aus der config.ini"
        );
        // Der Nutzer aendert die config.ini von aussen, die App laeuft weiter.
        std::fs::write(
            basis.join(CONFIG_FILE),
            format!("dateien_ordner = {}\n", ordner_b.display()),
        )
        .expect("config umbiegen");
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Datei");
        // Der aktuelle Ordner steht jetzt als Hinweis im Datei-Menue.
        assert!(
            !text_stellen(&offen, &format!("Aktueller Ordner: {}", app.folder)).is_empty(),
            "im Datei-Menue steht nicht, welcher Ordner benutzt wird"
        );
        let punkt = text_stellen(&offen, "Ordner neu scannen")
            .into_iter()
            .next()
            .expect("Menuepunkt 'Ordner neu scannen' fehlt im Datei-Menue");
        let mut ri = eingabe();
        klick_stelle(&mut ri, punkt);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        assert_eq!(
            app.folder,
            ordner_b.display().to_string(),
            "der neue Pfad aus der geaenderten config.ini wurde uebernommen"
        );
        assert!(
            app.status
                .iter()
                .any(|s| s.starts_with("Ordner aus config.ini uebernommen")),
            "Statusmeldung ueber die uebernommene Aenderung fehlt"
        );
        let _ = std::fs::remove_dir_all(&basis);
    }

    #[test]
    fn karteikarte_menue_macht_neue_karte() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Karteikarte");
        assert!(
            !text_stellen(&offen, "Neue Karte anlegen").is_empty(),
            "Punkt 'Neue Karte anlegen' fehlt"
        );
        assert!(
            !text_stellen(&offen, "Karte loeschen...").is_empty(),
            "Punkt 'Karte loeschen...' fehlt"
        );
        let punkt = text_stellen(&offen, "Neue Karte anlegen")[0];
        let mut ri = eingabe();
        klick_stelle(&mut ri, punkt);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(app.cards.len(), 7, "eine Karte kam hinzu");
        assert_eq!(app.cards[6].b, 7, "neue Karte traegt B=7");
        assert!(app.dirty, "die neue Karte ist noch ungespeichert");
        // Das Menue allein loescht nichts; erst die Bestaetigung danach.
        assert!(!app.confirm_del);
    }

    // ---------- Senkrechte Bildknopf-Leiste ganz links ----------

    // Die drei bisherigen Textknopfe unten links sind durch Bildknoepfe mit
    // Mouseover-Text in der senkrechten Leiste ersetzt.
    #[test]
    fn ikonleiste_bildknoepfe_ohne_alte_textknopfe_mit_mouseover() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        // Mauskopfgesteuerte Sprechblasen haben eine Wartezeit und eine
        // "Zeiger still"-Bedingung; im kopflosen Test beides abschalten,
        // sonst haengt ihr Erscheinen an der echten Laufzeit.
        ctx.style_mut(|s| {
            s.interaction.tooltip_delay = 0.0;
            s.interaction.show_tooltips_only_when_still = false;
        });
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        for alt in ["+ neue Karte", "- Karte loeschen", "Speichern"] {
            assert!(text_stellen(&aus, alt).is_empty(), "alter Textknopf {} ist noch da", alt);
        }
        // Mouseover ueber dem ersten Symbol: die Sprechblase ist ein neues Area
        // und wird erst ab dem zweiten Durchlauf gemalt.
        let mut ri = eingabe();
        ri.events.push(egui::Event::PointerMoved(app.icon_rects.0[0].center()));
        let _ = zeichne_ganz(&ctx, &mut app, ri.clone());
        let hover = zeichne_ganz(&ctx, &mut app, ri);
        assert!(
            !text_stellen(&hover, "Neue Karte anlegen").is_empty(),
            "Mouseover-Text des ersten Bildknopfs fehlt"
        );
        // Mouseover ueber dem Speichersymbol (nichts ungespeichert, grau):
        // nach dem Zeigerwechsel erst ein neutraler Durchlauf, dann zwei fuer
        // die neue Sprechblase.
        let mut weg = eingabe();
        weg.events.push(egui::Event::PointerMoved(egui::pos2(700.0, 450.0)));
        let _ = zeichne_ganz(&ctx, &mut app, weg);
        let mut ri = eingabe();
        ri.events.push(egui::Event::PointerMoved(app.icon_rects.0[2].center()));
        let _ = zeichne_ganz(&ctx, &mut app, ri.clone());
        let _ = zeichne_ganz(&ctx, &mut app, ri.clone());
        let hover = zeichne_ganz(&ctx, &mut app, ri);
        assert!(
            !text_stellen(&hover, "Speichern—imMomentnichtsungespeichert").is_empty(),
            "Mouseover-Text des Speichersymbols fehlt"
        );
    }

    #[test]
    fn ikonleiste_neues_und_loeschen_knopf_wirken_mit_frag() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let mut ri = eingabe();
        klick_stelle(&mut ri, app.icon_rects.0[0]);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(app.cards.len(), 7, "eine Karte kam hinzu");
        assert!(app.dirty, "die neue Karte ist noch ungespeichert");
        // Loeschen: erst die Nachfrage (Ja/Nein unter dem Symbol), dann Ja.
        let mut ri = eingabe();
        klick_stelle(&mut ri, app.icon_rects.0[1]);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.confirm_del, "Erstfrage ist aktiv");
        assert_eq!(app.cards.len(), 7, "der erste Klick loescht noch nicht");
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        let ja = text_stellen(&aus, "Ja")
            .into_iter()
            .next()
            .expect("Ja-Knopf unter dem Loeschsymbol fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, ja);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(app.cards.len(), 6, "Karte wurde nach Bestaetigung entfernt");
        assert!(!app.confirm_del, "die Nachfrage ist vorbei");
    }

    #[test]
    fn ikonleiste_speicherknopf_schreibt_sicherung_und_datei() {
        let basis = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-tmp-ikon");
        let _ = std::fs::remove_dir_all(&basis);
        std::fs::create_dir_all(&basis).expect("Testordner anlegen");
        let ziel = basis.join("test.dat");
        let mut app = menue_app();
        std::fs::write(&ziel, encode(&app.cards)).expect("Ausgangsdatei schreiben");
        app.loaded = Some(ziel.clone());
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        // Ohne Aenderung ist der Knopf grau: ein Klick darauf tut nichts.
        let mut ri = eingabe();
        klick_stelle(&mut ri, app.icon_rects.0[2]);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.confirm_save, "ohne Aenderungen ist Speichern undisponiert");
        // Aenderung anbringen, dann Speichern mit Nachfrage (Ja unter dem Symbol).
        app.dirty = true;
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let mut ri = eingabe();
        klick_stelle(&mut ri, app.icon_rects.0[2]);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.confirm_save, "Bestaetigungsfrage fehlt");
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        let ja = text_stellen(&aus, "Ja")
            .into_iter()
            .next()
            .expect("Ja-Knopf unter dem Speichersymbol fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, ja);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.dirty, "nach dem Speichern ist nichts mehr ungespeichert");
        assert!(
            basis.join("test.dat.bak").exists(),
            "Sicherungskopie wurde vor dem Schreiben angelegt"
        );
        let back = std::fs::read(&ziel).expect("Datei zuruecklesen");
        assert_eq!(
            parse_file(&back).expect("Ruecklesen der Datei").len(),
            6,
            "alle Karten stehen in der geschriebenen Datei"
        );
        assert!(
            app.status.iter().any(|s| s.starts_with("GESPEICHERT:")),
            "die Speicher-Meldung fehlt in der Statuszeile"
        );
        let _ = std::fs::remove_dir_all(&basis);
    }

    // ---------- Schliess-Rueckfrage bei ungespeicherten Aenderungen ----------
    // eframe meldet das X des Fensters (und ebenso den Menuepunkt "Beenden",
    // der ViewportCommand::Close schickt) als ViewportEvent::Close im
    // Rohinput des naechsten Bildaufbaus. Die App faengt ihn bei dirty==true
    // mit CancelClose ab und fragt nach; ohne Aenderungen schliesst sie direkt.

    // Simulierter Schliesswunsch als Roh-Eingabe.
    fn schliess_wunsch() -> egui::RawInput {
        let mut ri = eingabe();
        ri.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .expect("Wurzel-Viewport in den Roh-Eingaben")
            .events
            .push(egui::ViewportEvent::Close);
        ri
    }

    // Die Befehle, die egui an den Wurzel-Viewport zurueckgibt (Close, CancelClose).
    fn wurzel_befehle(out: &egui::FullOutput) -> Vec<egui::ViewportCommand> {
        out.viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.commands.clone())
            .unwrap_or_default()
    }

    #[test]
    fn schliessen_mit_ungespeicherten_aenderungen_liefert_rueckfrage() {
        let mut app = menue_app();
        app.dirty = true;
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let aus = zeichne_ganz(&ctx, &mut app, schliess_wunsch());
        assert!(app.confirm_quit, "die Schliess-Frage ist offen");
        assert!(app.dirty, "es ist nichts einfach verworfen");
        assert!(
            wurzel_befehle(&aus).contains(&egui::ViewportCommand::CancelClose),
            "das Schliessen wurde nicht abgefangen"
        );
        // Die Antworten der Frage erscheinen erst im naechsten Bild (Area).
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        for antwort in ["Speichern und beenden", "Verwerfen und beenden", "Abbrechen"] {
            assert!(
                !text_stellen(&mal, antwort).is_empty(),
                "die Antwort \"{antwort}\" fehlt in der Frage"
            );
        }
    }

    #[test]
    fn schliessen_ohne_ungespeicherte_aenderungen_schliesst_sofort() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let aus = zeichne_ganz(&ctx, &mut app, schliess_wunsch());
        assert!(!app.confirm_quit, "ohne Aenderungen bleibt die Frage aus");
        assert!(
            !wurzel_befehle(&aus).contains(&egui::ViewportCommand::CancelClose),
            "ohne Aenderungen darf das Schliessen nicht abgefangen werden"
        );
    }

    #[test]
    fn schliessfrage_verwerfen_gibt_schliessbefehl() {
        let mut app = menue_app();
        app.dirty = true;
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, schliess_wunsch());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let knopf = text_stellen(&mal, "Verwerfen und beenden")
            .into_iter()
            .next()
            .expect("der Knopf zum Verwerfen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let aus = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.confirm_quit, "die Frage ist vorbei");
        assert!(!app.dirty, "die Aenderungen sind verworfen");
        assert!(
            wurzel_befehle(&aus).contains(&egui::ViewportCommand::Close),
            "der Schliessbefehl fehlt"
        );
        assert!(
            app.status.iter().any(|s| s.contains("verworfen")),
            "die Statusmeldung ueber das Verwerfen fehlt"
        );
    }

    #[test]
    fn schliessfrage_abbrechen_laesst_alles_unveraendert() {
        let mut app = menue_app();
        app.dirty = true;
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, schliess_wunsch());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let knopf = text_stellen(&mal, "Abbrechen")
            .into_iter()
            .next()
            .expect("der Abbrechen-Knopf fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let aus = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.confirm_quit, "die Frage ist zu");
        assert!(app.dirty, "die Aenderungen bleiben erhalten");
        assert_eq!(app.cards.len(), 6, "an den Karten ist nichts geaendert");
        assert!(
            !wurzel_befehle(&aus).contains(&egui::ViewportCommand::Close),
            "es wurde kein Schliessbefehl gegeben"
        );
    }

    #[test]
    fn schliessfrage_speichern_und_beenden_schreibt_und_schliesst() {
        let basis = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-tmp-quit");
        let _ = std::fs::remove_dir_all(&basis);
        std::fs::create_dir_all(&basis).expect("Testordner anlegen");
        let ziel = basis.join("test.dat");
        let mut app = menue_app();
        std::fs::write(&ziel, encode(&app.cards)).expect("Ausgangsdatei schreiben");
        app.loaded = Some(ziel.clone());
        app.dirty = true;
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, schliess_wunsch());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let knopf = text_stellen(&mal, "Speichern und beenden")
            .into_iter()
            .next()
            .expect("der Knopf zum Speichern und Beenden fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let aus = zeichne_ganz(&ctx, &mut app, ri);
        assert!(
            !app.confirm_quit && !app.dirty,
            "nach dem Speichern ist die Frage vorbei und nichts mehr ungespeichert"
        );
        assert!(
            wurzel_befehle(&aus).contains(&egui::ViewportCommand::Close),
            "der Schliessbefehl fehlt"
        );
        assert!(
            basis.join("test.dat.bak").exists(),
            "die Sicherungskopie wurde vor dem Schliessen angelegt"
        );
        let _ = std::fs::remove_dir_all(&basis);
    }

    // Die Kind-Beziehungen (L1) sind im Formular bearbeitbar: Hinzufuegen und
    // Entfernen wirkt immer auf beide Seiten — L1-Paar der Elternkarte und
    // Elternfach (w2 bei einer Mutterkarte) der Kindkarte.
    #[test]
    fn kinderliste_hinzzufuegen_und_entfernen_wirkt_beidseitig() {
        let mut app = menue_app();
        app.selected = 3; // Mutter B4; ihre L1-Liste ist zunaechst leer
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // Kartenwechsel setzt kind_neu zurueck
        app.kind_neu = "5".to_string(); // Kind A
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "noch keine Kinder eingetragen").is_empty(),
            "ohne Kinder zeigt die Sektion den Hinweistext"
        );
        let knopf = text_stellen(&mal, "Kind hinzuuegen")
            .into_iter()
            .next()
            .expect("der Knopf Kind hinzuuegen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(app.cards[3].l1, vec![(5, 0)], "die Mutterkarte traegt Kind A in L1");
        assert_eq!(app.cards[4].w2, 4, "das Mutter-Fach von Kind A zeigt auf B4");
        assert_eq!(app.cards[4].w1, 3, "das Vater-Fach von Kind A bleibt unangetastet");
        assert!(app.dirty, "die Liste ist als geaendert markiert");
        assert!(
            app.status.iter().any(|s| s.starts_with("Kind B5")),
            "die Erfolgsmeldung fehlt in der Statuszeile"
        );
        // Kind A steht jetzt in der Liste und kann dort wieder ausgetragen werden.
        // Der Zweitwert ist aus der Anzeige herausgefallen; die Zeile heisst nur
        // noch "B5 Kind A".
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B5 Kind A").is_empty(),
            "die Kindzeile fehlt"
        );
        assert!(
            text_stellen(&mal, "Zweitwert der Originaldatei: 0").is_empty(),
            "der Zweitwert soll nicht mehr angezeigt werden"
        );
        assert!(
            !text_stellen(&mal, "entfernen").is_empty(),
            "der entfernen-Knopf in der Kindzeile fehlt"
        );
        let weg = text_stellen(&mal, "entfernen")
            .into_iter()
            .next()
            .expect("der Knopf entfernen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, weg);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.cards[3].l1.is_empty(), "der L1-Eintrag der Mutter ist fort");
        assert_eq!(app.cards[4].w2, 0, "das Mutter-Fach von Kind A ist geleert");
        assert_eq!(app.cards[4].w1, 3, "die Vaterseite bleibt auch jetzt heil");
    }

    // Ein Kind, dessen passendes Elternfach schon von einer anderen Person
    // belegt ist, laesst sich nicht dazuwaehlen; die Karten bleiben unveraendert.
    #[test]
    fn kinderliste_lehnt_belegtes_elternfach_ab() {
        let mut app = menue_app();
        app.selected = 1; // Grossmutter B2 (weiblich)
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // setzt kind_neu zurueck
        app.kind_neu = "5".to_string(); // Kind A hat die Mutter B4 schon
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let knopf = text_stellen(&mal, "Kind hinzuuegen")
            .into_iter()
            .next()
            .expect("der Knopf Kind hinzuuegen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.cards[1].l1.is_empty(), "nichts wurde eingetragen");
        assert_eq!(app.cards[4].w2, 4, "das Mutter-Fach von Kind A bleibt bei B4");
        assert!(
            app.status.iter().any(|s| s.starts_with("L1-Fehler:")),
            "die Ablehnung fehlt in der Statuszeile"
        );
    }

    // Haengerei aus einem entfernten Kind: die Elternkarte zeigt einen
    // L1-Eintrag auf eine Karte, die es im Datensatz nicht (mehr) gibt.
    // Nutzermeldung vom 30.10.: so ein Eintrag war nicht mehr zu entfernen.
    // Jetzt traegt "entfernen" die Zeile aus der Elternkarte aus.
    #[test]
    fn haengende_kinderzeile_laesst_sich_entfernen() {
        let mut app = menue_app();
        app.selected = 2; // Vater B3
        app.cards[2].l1.push((99, 0)); // Kind B99 gibt es in diesem Datensatz nicht
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // Kartenwechsel setzt Puffer zurueck
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B99 (fehlt in diesem Datensatz)").is_empty(),
            "die haengende Kinderzeile wird nicht angezeigt"
        );
        let weg = text_stellen(&mal, "entfernen")
            .into_iter()
            .next()
            .expect("der entfernen-Knopf in der haengenden Zeile fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, weg);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.cards[2].l1.is_empty(), "der haengende L1-Eintrag ist aus der Elternkarte fort");
        assert!(app.dirty, "die Liste ist als geaendert markiert");
        assert!(
            app.status
                .iter()
                .any(|s| s.contains("nur Elternseite ausgetragen")),
            "die Erfolgsmeldung ueber die einseitige Austragung fehlt"
        );
    }

    // Dasselbe fuer eine haengende Heiratszeile (L2 auf eine fehlende
    // Partnerkarte): die eigene Karte lasst sich damit sauber halten.
    #[test]
    fn haengende_ehezeile_laesst_sich_entfernen() {
        let mut app = menue_app();
        app.selected = 2; // Vater B3
        app.cards[2].l2.push(Ev {
            v: 98,
            raw: [0; 12],
            txt: Vec::new(),
            a: 0,
            b: 1,
        });
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B98 (fehlt in diesem Datensatz)").is_empty(),
            "die haengende Heiratszeile wird nicht angezeigt"
        );
        let weg = text_stellen(&mal, "entfernen")
            .into_iter()
            .next()
            .expect("der entfernen-Knopf in der haengenden Ehezeile fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, weg);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.cards[2].l2.is_empty(), "das haengende L2-Ereignis ist aus der Karte fort");
        assert!(
            app.status
                .iter()
                .any(|s| s.contains("nur eigene Seite ausgetragen")),
            "die Erfolgsmeldung ueber die einseitige Austragung fehlt"
        );
    }

    // Karte loeschen hinterlaesst keine Beziehungen mehr (Nutzerwunsch 30.10.):
    // Elternkarten verlieren den Kind-Eintrag, die Partnerkarte das
    // L2-Ereignis, Kinderkarten das Elternfach.
    #[test]
    fn karte_loeschen_bereinigt_kinder_ehen_und_elternfaecher() {
        let mut app = family_app();
        // Die echten Spiegelbilder wie in den Originaldateien:
        // Grosseltern-Karten mit L1 -> Vater B3; Vater/Mutter je ein L2-
        // Ereignis auf den anderen; beide Elternkarten mit L1 -> Kinder B5/B6.
        app.cards[0].l1.push((3, 0)); // Grossvater B1
        app.cards[1].l1.push((3, 0)); // Grossmutter B2
        app.cards[2].l2.push(Ev { v: 4, raw: [0; 12], txt: Vec::new(), a: 0, b: 1 });
        app.cards[3].l2.push(Ev { v: 3, raw: [0; 12], txt: Vec::new(), a: 0, b: 1 });
        for i in 2..4 {
            app.cards[i].l1.push((5, 0));
            app.cards[i].l1.push((6, 0));
        }
        app.selected = 2; // Vater B3 loeschen
        app.delete_card();
        assert_eq!(app.cards.len(), 5, "die Vaterkarte ist aus der Liste fort");
        assert!(
            app.cards[0].l1.is_empty() && app.cards[1].l1.is_empty(),
            "die Grosseltern-Karten tragen den geloeschten Sohn nicht mehr in L1"
        );
        // Nach dem Entfernen: Karten 0,1 = Grosseltern, 2 = Mutter B4, 3,4 = Kinder.
        assert!(
            app.cards[2].l2.is_empty(),
            "der Mutter ist das L2-Ereignis auf den geloeschten Mann fortgenommen"
        );
        assert_eq!(
            app.cards[2].l1,
            vec![(5, 0), (6, 0)],
            "die Kinder der Mutter bleiben unangetastet"
        );
        for k in &app.cards[3..] {
            assert_eq!(k.w1, 0, "Kind B{} hat kein Vater-Fach auf den Geloeschten mehr", k.b);
            assert_eq!(k.w2, 4, "Kind B{} haelt weiter die Mutter B4", k.b);
        }
        assert!(
            app.status.iter().any(|s| s.starts_with("Beziehungen zu B3")),
            "die Statusmeldungen ueber die Bereinigung fehlen"
        );
    }

    // Die Sektion "Heirat (L2)" zeigt jedes L2-Ereignis der Karte: das v-Feld ist
    // der B-Schluessel des Ehepartners, der Block traegt das Heiratsdatum.
    #[test]
    fn heirat_l2_sektion_zeigt_ehepartner_und_datum() {
        let mut app = menue_app();
        app.selected = 3; // Mutter B4; ohne L2-Ereignisse zuerst der Hinweistext
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "keine Heirat eingetragen").is_empty(),
            "ohne L2-Ereignisse fehlt der Hinweistext"
        );
        // Bekanntes Datum: Tag 12, Monat 5, Jahr 1850 (u16 little-endian je Teil;
        // q, x1 und x2 bleiben Null). v=3 zeigt auf den Vater.
        let mut raw = [0u8; 12];
        raw[2] = 12;
        raw[4] = 5;
        raw[6] = 0x3A; // 1850 = 0x073A little-endian
        raw[7] = 0x07;
        app.cards[3].l2.push(Ev {
            v: 3,
            raw,
            txt: b"Standesamt".to_vec(),
            a: 0,
            b: 1,
        });
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B3 Vater Text: \"Standesamt\"").is_empty(),
            "die Heiratszeile mit Ehepartner und Text fehlt"
        );
        assert!(
            !text_stellen(&mal, "12.05.1850").is_empty(),
            "das Datum steht nicht im Datumsfeld der Zeile"
        );
        // Lauter Nullen im Block (und v=0): Partner-Hinweistext und leerer
        // Datumsfeld-Platzhalter.
        app.cards[3].l2.push(Ev::default());
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B0 (fehlt in diesem Datensatz)").is_empty(),
            "die Zeile mit unbekanntem Partner fehlt"
        );
        assert!(
            !text_stellen(&mal, "(Datum unbekannt)").is_empty(),
            "der Platzhalter fuer das unbekannte Datum fehlt"
        );
    }

    // Analog zu den Kindern: Ehepartner lassen sich hinzuwaehlen und entfernen —
    // der L2-Eintrag steht in beiden Karten (jeweils v auf den anderen).
    #[test]
    fn heirat_hinzzufuegen_und_entfernen_wirkt_beidseitig() {
        let mut app = menue_app();
        app.selected = 3; // Mutter B4; der Vater B3 ist noch kein L2-Partner
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe()); // Kartenwechsel setzt heirat_neu zurueck
        app.heirat_neu = "3".to_string(); // Vater
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let knopf = text_stellen(&mal, "Heirat hinzuuegen")
            .into_iter()
            .next()
            .expect("der Knopf Heirat hinzuuegen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, knopf);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(
            app.cards[3].l2.iter().map(|e| e.v).collect::<Vec<_>>(),
            vec![3],
            "die Mutterkarte traegt den Vater in L2"
        );
        assert_eq!(
            app.cards[2].l2.iter().map(|e| e.v).collect::<Vec<_>>(),
            vec![4],
            "die Vaterkarte traegt zurueck die Mutter"
        );
        assert!(app.dirty, "die Liste ist als geaendert markiert");
        assert!(
            app.status.iter().any(|s| s.starts_with("B4 Mutter und B3 Vater")),
            "die Erfolgsmeldung fehlt in der Statuszeile"
        );
        // Die Heiratszeile hat nun Datumsfeld und einen entfernen-Knopf; der
        // Knopf loest die Paarbindung in beiden Karten auf.
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "B3 Vater").is_empty(),
            "die Heiratszeile fehlt"
        );
        assert!(
            !text_stellen(&mal, "(Datum unbekannt)").is_empty(),
            "das leere Datumsfeld zeigt den Platzhalter"
        );
        let weg = text_stellen(&mal, "entfernen")
            .into_iter()
            .next()
            .expect("der Knopf entfernen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, weg);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(app.cards[3].l2.is_empty(), "der Eintrag der Mutterkarte ist fort");
        assert!(app.cards[2].l2.is_empty(), "der Gegeneintrag der Vaterkarte ist fort");
    }

    // Das Heiratsdatum steht in der Zeile als Eingabefeld; "Datum setzen"
    // schreibt es in den 12-Byte-Block des Ereignisses (TT.MM.JJJJ, JJJJ oder
    // "-"/leer = unbekannt), unlesbare Eingaben werden abgelehnt.
    #[test]
    fn heiratsdatum_der_l2_zeile_aenderbar() {
        let mut app = menue_app();
        app.selected = 3; // Mutter B4
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        // Eine Heirat mit bekanntem Datum 12.05.1850 in die Karte legen.
        let mut raw = [0u8; 12];
        raw[2] = 12;
        raw[4] = 5;
        raw[6] = 0x3A;
        raw[7] = 0x07;
        app.cards[3].l2.push(Ev {
            v: 3,
            raw,
            txt: Vec::new(),
            a: 0,
            b: 1,
        });
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(
            !text_stellen(&mal, "12.05.1850").is_empty(),
            "das Datum steht nicht im Eingabefeld der Zeile"
        );
        // Neues Datum ins Feld schreiben und anwenden; das Feld nimmt auch
        // ohne fuehrende Nullen (3.4.1852) an.
        app.ehen_dat[0] = "3.4.1852".to_string();
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let kn = text_stellen(&mal, "Datum setzen")
            .into_iter()
            .next()
            .expect("der Knopf Datum setzen fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, kn);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(fmt_date(&app.cards[3].l2[0].raw), "03.04.1852");
        assert!(app.dirty, "die Liste ist als geaendert markiert");
        assert!(
            app.status.iter().any(|s| s.starts_with("Heiratsdatum in Karte B4")),
            "die Erfolgsmeldung fehlt in der Statuszeile"
        );
        // Unlesbares Datum (Monat 13): Meldung, der Block bleibt unveraendert.
        app.ehen_dat[0] = "31.13.1852".to_string();
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let kn = text_stellen(&mal, "Datum setzen")[0];
        let mut ri = eingabe();
        klick_stelle(&mut ri, kn);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(fmt_date(&app.cards[3].l2[0].raw), "03.04.1852", "kaputtes Datum aendert nichts");
        assert!(
            app.status.iter().any(|s| s.starts_with("L2-Fehler: Datum")),
            "die Fehlermeldung fuer das kaputte Datum fehlt"
        );
        // "-": Datum zurueck auf unbekannt setzen.
        app.ehen_dat[0] = "-".to_string();
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        let kn = text_stellen(&mal, "Datum setzen")[0];
        let mut ri = eingabe();
        klick_stelle(&mut ri, kn);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(fmt_date(&app.cards[3].l2[0].raw), "", "'-' setzt das Datum auf unbekannt");
    }

    // Datei -> "Drucken...": das Druckfenster zeigt Seitenzahl, Druckerwahl,
    // Druck- und Abbruchknopf; Abbrechen schliesst ohne jeden Druckauftrag.
    #[test]
    fn datei_menue_drucken_oeffnet_den_druckdialog() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let _ = zeichne_ganz(&ctx, &mut app, eingabe());
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Datei");
        let punkt = text_stellen(&offen, "Drucken...")
            .into_iter()
            .next()
            .expect("Menuepunkt 'Drucken...' fehlt im Datei-Menue");
        let mut ri = eingabe();
        klick_stelle(&mut ri, punkt);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        // Ein neues Fenster malt im ersten Frame noch nichts (egui-Caching).
        let mal = zeichne_ganz(&ctx, &mut app, eingabe());
        assert!(app.print_open, "das Druckfenster hat sich nicht geoeffnet");
        assert!(
            !text_stellen(&mal, "Drucken — alle Mitglieder").is_empty(),
            "der Titel des Druckfensters fehlt"
        );
        assert!(
            !text_stellen(
                &mal,
                "6 Mitglieder — 1 Seite(n) zu je 8 Personen (DIN A4, 4 Reihen zu 2 Spalten)."
            )
            .is_empty(),
            "die Seitenzahl fehlt (6 Karten = 1 Seite)"
        );
        assert!(
            !text_stellen(&mal, "Auf Drucker drucken").is_empty(),
            "der Druckknopf fehlt"
        );
        let weg = text_stellen(&mal, "Abbrechen")
            .into_iter()
            .next()
            .expect("der Abbruchknopf fehlt");
        let mut ri = eingabe();
        klick_stelle(&mut ri, weg);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert!(!app.print_open, "das Druckfenster hat sich nicht geschlossen");
    }

    // Die druckfertigen Datensaetze uebernehmen Name, Schluessel, Geschlecht,
    // Geburt/Tod und die Eltern mit Namen aus dem geladenen Datensatz.
    #[test]
    fn print_personen_uebernehmen_karten_mit_eltern_ehe_und_kindern() {
        let mut app = menue_app();
        // Beziehungen nachtraeglich anlegen, damit beide Elternkarten (L1) und
        // beide Eheleisten (L2) gefuellt sind — genau so pflegt die GUI sie.
        l1_add(&mut app.cards, 3, 5).expect("Vater B3 erhaelt Kind A B5");
        l1_add(&mut app.cards, 3, 6).expect("Vater B3 erhaelt Kind B B6");
        l1_add(&mut app.cards, 4, 5).expect("Mutter B4 erhaelt Kind A B5");
        l1_add(&mut app.cards, 4, 6).expect("Mutter B4 erhaelt Kind B B6");
        l2_heirat_add(&mut app.cards, 3, 4).expect("Ehe B3 und B4");
        for karte in app.cards.iter_mut() {
            if karte.b == 3 || karte.b == 4 {
                for e in karte.l2.iter_mut() {
                    if e.v != 0 {
                        assert!(set_date(&mut e.raw, "04.05.1900"), "Heiratsdatum setzbar");
                    }
                }
            }
        }
        let personen = app.print_personen();
        assert_eq!(personen.len(), 6, "alle Mitglieder des Datensatzes");
        let kind = personen.iter().find(|p| p.b == 5).expect("Kind A (B5)");
        assert_eq!(kind.name, "Kind A");
        assert!(kind.maennlich, "Kind A ist ein Junge");
        assert_eq!(kind.vater, "Vater (B3)");
        assert_eq!(kind.mutter, "Mutter (B4)");
        assert!(kind.geburt.is_empty(), "ohne Datumsblock ist das Feld leer");
        assert!(kind.ehen.is_empty() && kind.kinder.is_empty(), "Kind A hat selbst keine Familie");
        // Elternseite: Kinder in Anzeigereihenfolge, Ehe mit Heiratsdatum
        let vati = personen.iter().find(|p| p.b == 3).expect("Vater (B3)");
        assert_eq!(vati.ehen, ["Mutter (B4) verh. 04.05.1900"]);
        assert_eq!(vati.kinder, ["Kind A (B5)", "Kind B (B6)"]);
        let mutti = personen.iter().find(|p| p.b == 4).expect("Mutter (B4)");
        assert_eq!(mutti.ehen, ["Vater (B3) verh. 04.05.1900"], "die Gegenkarte zeigt dieselbe Ehe");
        assert_eq!(mutti.kinder, ["Kind A (B5)", "Kind B (B6)"]);
        // Grosseltern: ohne eigene L1-Liste, aber die Kinderkarten zeigen auf sie
        let gross = personen.iter().find(|p| p.b == 1).expect("Grossvater (B1)");
        assert!(gross.vater.is_empty() && gross.mutter.is_empty(), "ohne Schluessel keine Eltern");
        assert_eq!(gross.kinder, ["Vater (B3)"], "Kind ueber die w1/w2-Spur gefunden");
        assert!(gross.ehen.is_empty(), "unverheiratet bleibt die Ehezeile fort");
    }

    #[test]
    fn stammbaum_menue_wechselt_auf_den_baumreiter() {
        let mut app = menue_app();
        app.selected = 4; // Kind A ist in der Mitgliederliste markiert
        let ctx = egui::Context::default();
        let offen = menuepunkt_oeffnen(&ctx, &mut app, "Stammbaum");
        assert!(
            !text_stellen(&offen, "Nachkommen (alle Generationen)").is_empty(),
            "Nachkommen-Punkt fehlt im Stammbaum-Menue"
        );
        let punkt = text_stellen(&offen, "Nachkommen (alle Generationen)")[0];
        let mut ri = eingabe();
        klick_stelle(&mut ri, punkt);
        let _ = zeichne_ganz(&ctx, &mut app, ri);
        assert_eq!(app.tab, app.ev_tabs.len() + 1, "Der Baumreiter ist gewaehlt");
        assert_eq!(app.tree_mode, Mode::Desc);
        assert_eq!(app.tree_gens, ALL_GENS);
        assert_eq!(app.tree_sel, 4, "markierte Person wurde Wurzel");
        assert_eq!(app.tree_view.nodes[app.tree_view.root].card, 4);
    }

    // Die Reiterzeile zeigt nur noch "Karten" und "Stammbaum"; die
    // Ereignistafel-Knoepfe (Geburt/Taufe/Heirat ...) sind auskommentiert.
    // Der Test tut so, als waeren drei Tafeln geladen — ihre Knopfe und der
    // Hinweistext duerfen trotzdem nicht gezeichnet werden.
    #[test]
    fn karten_und_stammbaum_sind_die_einzigen_reiter() {
        let mut app = menue_app();
        let tafel = |t: &'static str, k: EvKind| EvTable {
            kind: k,
            title: t,
            path: std::path::PathBuf::from("test-tmp\\keine-datei"),
            recsize: 332,
            capacity: 1,
            counter: 0,
            file_len: 0,
            model_fits: true,
            n: 0,
            recs: Vec::new(),
        };
        app.ev_tabs = vec![
            tafel("Geburt", EvKind::Nais),
            tafel("Taufe", EvKind::Bapt),
            tafel("Heirat", EvKind::Mari),
        ];
        let ctx = egui::Context::default();
        let zeichnung = zeichne_ganz(&ctx, &mut app, eingabe());
        for titel in ["Geburt (0 aktiv)", "Taufe (0 aktiv)", "Heirat (0 aktiv)"] {
            assert!(
                text_stellen(&zeichnung, titel).is_empty(),
                "der Reiter {} ist nicht mehr sichtbar",
                titel
            );
        }
        assert!(
            text_stellen(&zeichnung, "(keine Ereignistafeln in diesem Ordner gefunden)")
                .is_empty(),
            "auch der Hinweistext auf Ereignistafeln ist weg"
        );
        assert!(!text_stellen(&zeichnung, "Karten").is_empty(), "der Reiter Karten bleibt");
        assert!(
            text_stellen(&zeichnung, "Stammbaum").len() >= 2,
            "Stammbaum steht als Reiter und als Menuepunkt"
        );
    }

    // Die Originaldaten speichern ein unbekanntes Datum mit 0xFFFF in allen
    // drei Teilen. Das Formular zeigte daraus "65535.65535.65535"; jetzt bleibt
    // das Feld leer (Hinweistext) und nur ein bekanntes Jahr bleibt sichtbar.
    #[test]
    fn unbekanntes_datum_65535_taucht_nirgends_auf() {
        let mut app = menue_app();
        app.selected = 1;
        for i in 0..3 {
            app.cards[1].dt[i].raw[2..6].fill(0xFF); // Tag und Monat unbekannt
            app.cards[1].dt[i].raw[6] = 0xFF;
            app.cards[1].dt[i].raw[7] = 0xFF; // Jahr unbekannt
        }
        app.cards[1].dt[0].raw[6] = 0x07;
        app.cards[1].dt[0].raw[7] = 0x07; // 1799 little-endian, nur das Jahr bekannt
        app.form_for = usize::MAX; // Formular wird mit den neuen Werten geladen
        let ctx = egui::Context::default();
        // Der Formularinhalt im Scrollbereich wird — wie die Menue-Fenster —
        // erst ab dem zweiten Frame gemalt: erst ein neutraler Durchlauf.
        let erster = zeichne_ganz(&ctx, &mut app, eingabe());
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        let mut sieht_jahr = false;
        for frame in [&erster, &aus] {
            for shape in &frame.shapes {
                if let egui::Shape::Text(t) = &shape.shape {
                    let text: String = t
                        .galley
                        .rows
                        .iter()
                        .flat_map(|r| r.glyphs.iter().map(|g| g.chr))
                        .collect();
                    assert!(
                        !text.contains("65535"),
                        "unbekanntes Datum wird als 65535 dargestellt: {:?}",
                        text
                    );
                    if text.contains("1799") {
                        sieht_jahr = true;
                    }
                }
            }
        }
        assert!(sieht_jahr, "ein bekanntes Jahr bleibt sichtbar (1799)");
        // Zwei voellig unbekannte Datenteile bleiben leere Felder mit Hinweis.
        assert_eq!(
            text_stellen(&aus, "TT.MM.JJJJ").len(),
            2,
            "die zwei unbekannten Datumsfelder zeigen den Hinweistext"
        );
    }

    // Vor jedem der drei Datumsfelder steht "Datum:", vor jedem Ortsfeld
    // "Ort:" — je dreimal im Formular des Kartenreiters.
    #[test]
    fn datum_und_ort_stehen_vor_den_feldern() {
        let mut app = menue_app();
        let ctx = egui::Context::default();
        let erster = zeichne_ganz(&ctx, &mut app, eingabe());
        let aus = zeichne_ganz(&ctx, &mut app, eingabe());
        // Der Scrollbereich malt erst ab dem zweiten Frame; gezählt wird im zweiten.
        let _ = erster;
        assert_eq!(
            text_stellen(&aus, "Datum:").len(),
            3,
            "Geburt/Heirat/Tod tragen je ein 'Datum:'"
        );
        assert_eq!(
            text_stellen(&aus, "Ort:").len(),
            3,
            "unter jedem Datum steht ein 'Ort:'"
        );
    }
}
