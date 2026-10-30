# Racines in Rust — Familienkartei und Stammbaum

Eine moderne Neuauflage des alten Windows 95 -Genealogieprogramms **RACINES**
(Personal Soft, franzoesische Originalkartei aus den 1990ern). Deutsch unter dem 
Namen "Ahnengalerie" vertrieben. Das Nachfolgeprogramm liest und schreibt 
die **originalen Datensaetze** (FICHES, MARIAGES, NAISS, DECES ... in den 
Nummernschluessen .001 bis .005, Textseite CP1252) 
— wer also frueher mit RACINES gearbeitet hat, kann seine Datenkartei hier weiterpflegen.

Die Oberflaeche ist in **Rust** gebaut (Rahmen [eframe/egui](https://github.com/emilk/egui))
und laeuft als ein einziges Windows-Programm ohne Installierer.

## Was es kann

- **Karteikarten-Reiter**: Personen erfassen und bearbeiten (Name, Daten,
  Eltern, Ehe, Kinder) — Anderungen an Elternfeldern ziehen die Kinderlisten
  der Elternkarten automatisch mit.
- **Stammbaum-Reiter**: Baumansicht mit anklickbaren Familienkaesten
  (neue Wurzel setzen), Vorfahren- und Nachfahrenverzweigung.
- **Heiraten (L2)**: beidseitig gepflegt — Heirat hinzuuegen oder entfernen
  wirkt immer auf beide Partnerkarten; das Datumsfeld sitzt in der
  Ehe-Ereigniszeile.
- **Loeschen einer Karte bereinigt die Beziehungen**: Kinderlisten,
  Ehepartner-Ereignisse und Elternfaecher der betroffenen Karten werden
  mitgeaendert; auch haengende Zeile-Nachweise aus dem Altformat lassen sich
  noch entfernen.
- **Drucken**: Mitgliederblatt aller Karten, je 8 Personen auf einer
  A4-Seite, mit Drucker-Auswahl.
- **Sicheres Speichern**: beim Speichern wandert die alte Datei nach
  `.bak` (die vorige nach `.bak~1`), der DOSSIER-Zaehler wird passend
  gepatcht. Beim Schliessen mit ungespeicherten Aenderungen fragt die App:
  **speichern oder verwerfen?**
- **Datensatz-Auswahl** (.001 bis .006) und ein gemerktes DATEIEN-Verzeichnis
  in `config.ini` (wird bei "Ordner neu scannen" erneut eingelesen).

## Mitnehmen und Ausprobieren

Voraussetzung ist ein [Rust-Toolchain](https://rustup.rs/). Dann im Ordner
`racines-gui`:

```
cargo build --release
```

Das fertige Programm liegt danach unter
`racines-gui\target\release\racines-gui.exe` (Windows, 64 Bit).
`build_rel.bat` macht dasselbe per Doppelklick.

Fuenf **Beispieldatensaetze** liegen schon im Repo unter
`racines-gui\DATEIEN`: die Original-Demos von RACINES — .001 *Die Genesis*,
.002 *Les Bourbons*, .003 *Die Hohenzollern*, .004 *Royal Families of
England*, .005 *Familles royales francaises*. Eigene (private) Familien-Daten
sind hier bewusst keine enthalten.

## Tests

```
cargo test --release -- --include-ignored
```

Aktuell **54 Tests** — sie pruefen das Dateiformat (Roundtrip bytegenau,
Backup-Rotation, CP1252), die Kartenlogik (Eltern/Kind/Ehe beidseitig),
den Stammbaum und die Oberflaeche bis hin zum Ansprechen der Buttons.
Die Tests laufen ohne Internet.

Hinweis fuer frische Klone: Einige Format-Tests lesen zusaetzliche
Original-Kopien unter `../racines-port/data` (nicht Teil dieses Repos, weil
dort auch persönliche Daten liegen). Auf einem anderen Rechner laufen diese
wenigen Tests ins Leere, bis man entsprechende Beispiel-Datensaetze dorthin
legt; die uebrigen Tests und der Bau der EXE funktionieren unveraendert.

## Repo-Aufbau

```
racines-gui/        das Rust-Programm (src, Cargo, Bau-Skript, Beispieldaten)
.gitignore          Whitelist: nur das Programm geht ins Repo
```

Der Rest der Entwicklung (Analyse des Originalprogramms, Notizen,
Werkzeuge) bleibt aus privaten Gruenden lokal und ist nicht Teil dieses
Repositories.

