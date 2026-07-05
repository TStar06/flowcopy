# FlowCopy installieren — Anleitung für Freunde

FlowCopy ist ein kostenloses Diktier-Tool: Taste gedrückt halten, sprechen, loslassen — der Text landet fertig formatiert in jeder App (WhatsApp, Mail, Word, Browser …). Keine Kosten, kein Abo, keine Cloud-Pflicht.

## Installation (2 Minuten)

1. **`FlowCopy_x64-setup.exe` doppelklicken.**
2. Windows zeigt eine blaue Warnung „Der Computer wurde durch Windows geschützt" — das ist normal bei Apps ohne teures Firmen-Zertifikat (~400 €/Jahr, sparen wir uns).
   → Klicke **„Weitere Informationen"** und dann **„Trotzdem ausführen"**.
3. Dem Installer folgen (Weiter → Fertig).

## Ersteinrichtung (3 Minuten)

1. Beim ersten Start lädt FlowCopy ein Sprachmodell herunter — nimm eine der **empfohlenen** Karten oben (gut für Deutsch + Englisch).
2. **Optional, aber empfohlen — beste Qualität gratis:** Auf der nächsten Seite kannst du einen kostenlosen **Groq-API-Key** hinterlegen:
   - Klicke „Kostenlosen API-Key holen" (öffnet console.groq.com — Anmeldung nur mit E-Mail, keine Kreditkarte)
   - Dort „Create API Key" → kopieren → in FlowCopy einfügen → „Aktivieren"
   - Damit bekommt jedes Diktat: bessere Erkennung, Füllwörter raus („ähm", „ja also"), Selbstkorrekturen angewendet
   - Ohne Key läuft alles lokal auf deinem Rechner — funktioniert auch, nur etwas langsamer/einfacher
3. Fertig. FlowCopy sitzt jetzt unten rechts im Tray.

## Benutzung

- **Diktieren:** `Strg + Leertaste` gedrückt **halten**, sprechen, loslassen → Text erscheint dort, wo dein Cursor steht.
- **Sprachbefehle:** „neue Zeile", „neuer Absatz", „Punkt", „Komma", „Fragezeichen" werden zu echter Formatierung.
- **Einstellungen:** Tray-Icon anklicken. Dort gibt es u. a.:
  - *Erweitert → Wörterbuch*: eigene Ersetzungen („vs code" → „VS Code") und Snippets („meine Signatur" → ganzer Textblock)
  - *Erweitert → Per-App-Profile*: pro App anderen Ton (WhatsApp locker, Outlook formell)
  - *Verlauf*: alle Diktate mit Suche und Statistik

## Häufige Fragen

**Kostet das was?** Nein. Das Programm ist Open Source (Basis: Handy, MIT-Lizenz), die lokalen Modelle laufen auf deinem Rechner, und der optionale Groq-Dienst hat einen dauerhaft kostenlosen Tarif, der für persönliches Diktieren mehr als reicht.

**Gehen meine Daten in die Cloud?** Nur wenn du den Groq-Key aktivierst — dann wird das Audio zur Transkription an Groq geschickt. Ohne Key bleibt alles zu 100 % auf deinem Rechner.

**Der Hotkey stört mich.** Unter Einstellungen → Allgemein kannst du jede beliebige Taste(nkombination) festlegen.
