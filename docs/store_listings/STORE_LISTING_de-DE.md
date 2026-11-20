# Microsoft Store Produktbeschreibung - Deutsch (de-DE)
# Win2Pod Bridge - Version 0.1.18

## 0. Neuheiten in dieser Version (What's new in this version - Für Partner Center)
```text
[Win2Pod Bridge 0.1.18 Großes Update - AirPlay Audio-Empfänger ist da!]
• 📲 Brandneuer „AirPlay Audio-Empfänger (Audio Receiver)“:
  - Verwandeln Sie Ihren Windows-PC in einen echten kabellosen AirPlay-Lautsprecher! Streamen Sie Audio von iPhone, iPad oder Mac („AirPlay“ im Kontrollzentrum) direkt auf Ihren PC.
  - Native 44,1 kHz / 16-Bit verlustfreie ALAC-Echtzeit-Dekodierung für glasklaren Hi-Fi-Sound.
• ✨ Maximaler Dynamikbereich (Unity Gain Passthrough):
  - Umgeht Lautstärkebeschränkungen des iOS-Kontrollzentrums. Der Empfänger dekodiert direkt mit 1,0 (0 dB Unity Gain) digitalem Maximalpegel – volle Dynamik und Details bleiben auch bei leiserem Handy erhalten.
  - Neues „✨ Max Dynamikbereich“-Badge mit Lautstärkeanzeige beim Darüberfahren.
• ⏯️ Intelligente Standby-Reaktivierung (DACP-Target-Caching):
  - Bei Wiedergabepause auf dem iPhone behält die Standby-Karte die ▶-Wiedergabetaste. Ein Klick weckt die Wiedergabe auf dem iPhone drahtlos auf.
• 🎛️ Bidirektionale Mediensteuerung & Lip-Sync-Synchronisation:
  - Steuern Sie Wiedergabe/Pause/Titelwechsel direkt vom PC aus, inklusive Albumcover-Synchronisation.
  - Integrierte Dual-Latenz-Modi: Standard (1,5 s) und Tief (2,0 s) für lippensynchrones YouTube- & Videostreaming.
```

## 1. Produktname (Product Name)
```text
Win2Pod Bridge
```

---

## 2. Kurzbeschreibung (Short Description - Max. 270 Zeichen)
*(Wird in Microsoft Store-Suchergebnissen, Kategoriekarten und Bannern angezeigt)*

```text
Windows & Apple Audio perfekt vereint! PC-Audio verlustfrei via ALAC an HomePod streamen oder Musik von iPhone & iPad via AirPlay empfangen mit Unity Gain Dynamikbereich! Inkl. AirPods-Handoff, 3D-Visualizer, Medienfernbedienung & 10-Band-EQ.
```
*(Zeichenanzahl: ca. 240 Zeichen, innerhalb des Limits von 270 Zeichen)*

---

## 3. Ausführliche Beschreibung (Full Description)
*(Kopieren Sie diesen Text direkt in das Feld „Beschreibung / Description“ im Microsoft Partner Center)*

```text
Verwandeln Sie Ihren Windows-PC in eine leistungsstarke AirPlay 2 Hi-Fi-Audiostation!

Win2Pod Bridge ist eine moderne, schlanke und hochleistungsfähige Audio-Streaming-Bridge, die speziell für Windows 10 und Windows 11 entwickelt wurde. Mithilfe der nativen Windows WASAPI-Loopback-Technologie mit extrem geringer Latenz wird Ihr PC-Systemsound oder der Ton bestimmter Anwendungen im verlustfreien ALAC-Format (Apple Lossless Audio Codec, 44,1 kHz / 16-Bit Stereo) verzögerungsfrei an Apple HomePod, HomePod mini, Apple TV und AirPlay 2-kompatible Lautsprecher übertragen.

Windows unterstützt das AirPlay 2-Protokoll von Haus aus nicht, und Apple HomePods setzen zwingend eine verschlüsselte HomeKit-Transient-Kopplung voraus. Win2Pod Bridge löst dies vollständig durch eine reine Software-Implementierung in Rust. Keine Hardware-Modifikationen, keine MFi-Chips und keine überladenen Hintergrunddienste erforderlich!

============================================================
🌟 WICHTIGSTE FUNKTIONEN & HIGHLIGHTS (KEY FEATURES)
============================================================

• 🛡️ Auto Pre-Amp & Professioneller 10-Band-Equalizer:
  - Der 10-Band-Equalizer passt die Vorverstärkung bei Bandanhebungen automatisch an, um digitale Übersteuerungen (Clipping) vollständig zu eliminieren. In Kombination mit einem transparenten Peak-Limiter bleibt der Klang stets unverfälscht und kristallklar.
• 🔄 Globales Audio-Fallback & Selbstheilung (Self-Healing Audio Routing):
  - Systemweite Überwachung der Audio-Endpunkte. Wird das Standard-Wiedergabegerät oder das Proxy-Gerät getrennt, schaltet Win2Pod nahtlos auf ein anderes aktives Audiogerät um, sodass der Windows-Ton niemals blockiert wird.
• 🔔 Kompakter 2-Zeilen-Tooltip im Infobereich (Compact Two-Line Tooltip):
  - Klares zweizeiliges Format: Zeile 1 zeigt den HomePod / AirPlay Streaming-Status und die Lautstärke (z. B. „🔊 Streaming: Wohnzimmer (9%)“ oder Stumm); Zeile 2 zeigt den Tragezustand und Akkustand der AirPods (z. B. „🎧 AirPods 3: ● L 100%“). Verhindert abgeschnittenen Text.
• 🎧 AirPods ↔ HomePod Intelligentes Handoff (Smart Handoff Routing):
  - Schaltet beim Aufsetzen der Kopfhörer automatisch nahtlos auf die AirPods um und übergibt die Wiedergabe beim Einlegen ins Ladecase präzise wieder an den HomePod!
• Neuer dynamischer Zeiger-Stereobalance-Regler (◄ Mitte ►):
  - Intuitive Richtungsanzeige: Zeigt in Mittelstellung „◄ Mitte ►“ und wechselt beim Verschieben dynamisch zu „◄ L 30%“ bzw. „30% R ►“.
  - Schnelles Zentrieren mit einem Klick: Ein Klick auf das Textlabel oder ein Doppelklick auf den Schieberegler stellt die Mitte sofort wieder her.
  - Reine 32-Bit-Float Software-DSP-Berechnung: Verhindert Windows-Treiber-Lautstärkefallen, garantiert 100% Dynamikumfang ohne Lautstärkeabfall und ohne digitales Clipping.
• Medienplayer mit einem Klick aufrufen: Klicken Sie in Win2Pod Player einfach auf das Vinyl-Cover oder den Songtitel, um die Wiedergabe-App (Spotify, Apple Music, Tidal, Chrome, Edge usw.) sofort in den Vordergrund zu holen.
• Volle Balance- und Mono-Unterstützung für lokale Kopfhörer/Lautsprecher (AO-Modus): Ob beim Streamen auf den HomePod oder beim Hören über am PC angeschlossene Kopfhörer – genießen Sie stufenlose Stereobalance und sofortiges Umschalten auf Mono (MONO).
• Symmetrische Wiederherstellung des Audio-Lebenszyklus & Anti-Feedback Shield:
  - Speichert das ursprüngliche Windows-Standardgerät beim Start und stellt Audiogeräte, Lautstärke und Stummschaltung beim Schließen zu 100 % originalgetreu wieder her.
  - Erkennt Rückkopplungsrisiken bei alleiniger AirPods-Nutzung und unterdrückt die Wiedergabeschleife zuverlässig, während die Visualisierung ressourcenschonend aktiv bleibt.

🎨 WIN2POD MOTION - 3D-IMPRESSIONISTISCHER AUDIO-VISUALIZER
• Lebendige Klangbewegung (Paint Pulse): Reagiert in Echtzeit auf das Audiospektrum – von tiefroten Basswellen bis zu violettem Hochtonglanz breiten sich Partikel im Rhythmus der Musik aus.
• Impressionistischer Pinselstrich-Shader (PainterlyPass): Dreidimensional verankerte Pinselstriche, die sich an die Modelloberfläche anpassen. Pinselgröße und Kameraperspektive frei anpassbar.
• Eigenständiger Vollbildmodus & Immer im Vordergrund: Vollbildmodus (Taste F) und Fensternadel (📌). Verhindert im Vollbildmodus automatisch den Bildschirmschoner und Energiesparmodus.
• Tastenkürzel-Übersicht (Taste ?): Übersichtliche Anzeige aller Navigations- und Effektsteuerungstasten.
• 100% Offline & Lokal: Vollständig integrierte Three.js-Engine ohne externe Internetverbindungen, blitzschneller Start.
• Nahtlose Umschaltung von 11 Sprachen in Echtzeit.

🎛️ ZWEISEITIGE SMARTE MEDIENSTEUERUNG
• Volle Unterstützung für Touch-Gesten auf der HomePod-Oberseite:
  - 1x Tippen: Wiedergabe / Pause (Play / Pause)
  - 2x Tippen: Nächster Titel (Next Track)
  - 3x Tippen: Vorheriger Titel (Previous Track)
• Volle Fernsteuerung über die Apple Home App und das iOS-Kontrollzentrum auf iPhone und iPad!
• Unterstützt AirPlay 2 updateMRSupportedCommands und verschlüsselte Rückkanal-Ereignisse, synchronisiert mit Windows GSMTC und Medientasten.
• Überträgt Echtzeit-Fortschritt und DMAP-Metadaten an den HomePod.

✨ HARMONIC EXCITER (HE) - HOCHTON-OBERTONRESTAURATION
• Frische Luftigkeit & Brillanz: Speziell entwickelt, um Höhenverluste bei komprimierten Streaming- und Bluetooth-Inhalten auszugleichen. Regeneriert Obertöne von 8 kHz bis 20 kHz.
• Drei Voreinstellungen: Natural (sanft, ideal für Klassik/Podcast), Dynamic (ausgewogen, ideal für Pop/Rock), Brilliant (glanzvoll, ideal für ältere Aufnahmen).

📻 WARMTH ENHANCER (WE) - ANALOGE SCHALLPLATTENWÄRME
• Authentischer Vinyl-Klang: Beseitigt digitale Härte durch harmonische Wärme klassischer Plattenspieler.
• Low-Shelf-Resonanzfilter (50Hz–90Hz) für tiefen, warmen Bass.
• Analoge Röhren- und Bandsättigung (2,5%–5,0%) mit integriertem Soft-Limiter gegen Übersteuern.
• Drei Vinyl-Stufen: Subtle (dezent), Warm (warm), Vintage (klassisch).

🎛️ PROFESSIONELLE DSP-SIGNALKETTE (HE ➔ EQ ➔ WE)
• Audiophile Verarbeitungsreihenfolge: Erst Obertonrestauration (HE), dann präzise Frequenzabstimmung (10-Band-EQ), gefolgt von analoger Sättigung und Pegelschutz (WE).
• Stille-Bypass (DSP-Schlafmodus): Bei Stille schaltet der DSP auf 0% CPU-Bypass und wacht bei Musikwiedergabe in 0 ms wieder auf!

📊 POST-DSP ECHTZEIT-SPEKTRUMANALYSATOR
• 16-Band-Ausgangsüberwachung: Zeigt die Frequenzverteilung exakt nach Durchlaufen der Effekte an.

⚡ ADAPTIVE BILDWIEDERHOLRATE & ENERGIEEFFIZIENZ
• Dynamische Anpassung: Läuft bei Musikwiedergabe mit flüssigen 30 FPS und drosselt bei Stille auf 2 FPS (~0,5% CPU-Last).
• 0,0% CPU im Infobereich: Stoppt das Rendern vollständig, wenn minimiert (nur ca. 8–15 MB RAM).

📻 WIN2POD PLAYER VINTAGE HI-FI-CHASSIS
• Retro-Design mit Albumcover, VFD-Leuchtschirm, 16-Band-Spektrum und 10-Band-Equalizer.
• GSMTC-Echtzeit-Metadaten: Songtitel, Interpret und Spielzeit (mm:ss / mm:ss).
• Ruckelfreier interaktiver Fortschrittsbalken.

📌 FENSTER IMMER IM VORDERGRUND (📌)
• Bleibt über allen Arbeits- und Gaming-Fenstern sichtbar.

🎯 ANWENDUNGSSPEZIFISCHES AUDIO-ROUTING
• Gesamtsystem oder einzelne Anwendung (z. B. Spotify oder Browser) isoliert an HomePod senden, während Spiele und Discord-Anrufe auf den PC-Kopfhörern verbleiben.

🎵 MASSGESCHNEIDERT FÜR APPLE HOMEPOD & AIRPLAY 2
• HomeKit Transient Pairing: Vollständig kompatibel mit HomePod und HomePod mini.
• Verlustfreies ALAC in Studioqualität (44,1 kHz, 16-Bit Stereo).
• IEEE 802.1AS / gPTP hochpräzise Taktsynchronisation ohne Jitter oder Zeitversatz.

🎚️ 10-BAND GRAFIK-EQUALIZER & KLANGPRESETS
• Präzise Steuerung von 31 Hz bis 16 kHz (±12 dB) mit 8 Presets (Bass Boost, Vocal, Rock usw.).

🔊 HOMEPOD STEREO-PAARE & MULTIROOM-STREAMING
• Automatische Erkennung gekoppelter HomePod-Stereopaare.
• Gleichzeitiges Streaming an mehrere AirPlay 2-Lautsprecher im ganzen Haus.
• Dynamische Gruppenverwaltung während der Wiedergabe.
• Zwei-Wege-Lautstärkesynchronisation mit den Tasten am HomePod.

============================================================
SYSTEMANFORDERUNGEN & HINWEISE
============================================================
• Betriebssystem: Windows 10 (Version 1809 oder neuer) oder Windows 11 (64-Bit).
• Netzwerk: PC und Lautsprecher müssen im selben lokalen Netzwerk (WLAN oder LAN) verbunden sein.
• Windows-Firewall: Eingehende Verbindungen bei der ersten Abfrage bitte zulassen.
```

---

## 4. Wichtigste Produktmerkmale (Product Features - Max. 120 Zeichen pro Punkt)
*(Fügen Sie die Punkte in das Feld „Merkmale / Features“ im Partner Center ein)*

1. `Player per Klick auf Cover oder Titel aufrufen: Spotify, Apple Music & Tidal sofort im Fokus`
2. `Dynamischer Zeiger-Stereobalance (◄ Mitte ►): 32-Bit-DSP, Klick stellt Mitte sofort wieder her`
3. `Win2Pod Motion: 3D-Impressionismus-Audio-Visualizer mit Vollbild & Immer-im-Vordergrund`
4. `Verlustfreies 44,1 kHz / 16-Bit 2-Kanal ALAC-Audio-Streaming an AirPlay 2-Lautsprecher`
5. `HomeKit Transient Verschlüsselung: 100% reines Rust, kein Apple MFi-Chip erforderlich`
6. `Harmonic Exciter (HE): stellt verloren gegangene Höhen und Obertonbrillanz wieder her`
7. `Warmth Enhancer (WE): authentische Vinyl-Resonanz, Röhrensättigung & Soft-Limiter`
8. `Audiophile DSP-Signalkette (HE ➔ EQ ➔ WE) mit 10-Band-EQ und 0% CPU-Stille-Bypass`
9. `Post-DSP Echtzeit-Spektrumanalysator: visualisiert die tatsächliche Klangausgabe`
10. `Adaptive Bildwiederholrate: drosselt bei Stille auf 2 FPS und senkt CPU-Last auf ~0,5%`
11. `Win2Pod Player: Retro Hi-Fi-Design mit Albumcover, GSMTC-Metadaten & sanftem Suchbalken`
12. `Immer im Vordergrund (📌): hält Hauptpanel und Player über allen Fenstern schwebend`
13. `App-spezifisches Routing: Musik auf HomePod streamen, während Discord am Kopfhörer bleibt`
14. `10-Band-Präzisions-Equalizer mit 8 kuratierten Klangpresets (Bass Boost, Vocal usw.)`
15. `Automatische Erkennung von HomePod-Stereopaaren für eine räumliche Stereobühne`
16. `Multiroom-Streaming: PC-Audio gleichzeitig an mehrere AirPlay-Lautsprecher senden`
17. `Dynamische Gruppenverwaltung: Lautsprecher flexibel hinzufügen oder entfernen`
18. `Zwei-Wege-Lautstärkesynchronisation: Tasten am HomePod steuern Windows-Schieberegler`
19. `0,0% CPU im Infobereich: pausiert alle Renderdurchläufe und belegt nur ca. 8–15 MB RAM`
20. `PC-Lautsprecher stummschalten: verhindert Echos beim Streaming und stellt Lautstärke wieder her`

---

## 5. Suchbegriffe (Search Terms / Keywords - Max. 7 Begriffe, je max. 30 Zeichen)
*(Fügen Sie diese in das Feld „Suchbegriffe / Search terms“ im Partner Center ein)*

1. `AirPlay`
2. `HomePod`
3. `Audio`
4. `Equalizer`
5. `Streaming`
6. `Musik`
7. `Player`
