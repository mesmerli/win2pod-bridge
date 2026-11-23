# Scheda di Microsoft Store - Italiano (it-IT)
# Win2Pod Bridge - Versione 0.1.21

## 0. Novità di questa versione (What's new in this version - Per Partner Center)
```text
[Win2Pod Bridge 0.1.21 Aggiornamento - Effetto Genio fluido e Traslucenza intelligente]
• 🧞 Effetto di riduzione a icona Genio stile macOS (Muelle Genie Effect):
  - Integrazione dell'effetto S-Curve per riga di scansione ispirato al progetto open source muelle. La finestra viene aspirata elegantemente nella barra delle applicazioni durante la riduzione a icona e ripristinata istantaneamente.
• 🪟 Traslucenza nativa di Windows (WS_EX_LAYERED) e Hover Peek intelligente:
  - La finestra fluttuante del visualizzatore supporta l'opacità regolabile (20% ~ 100%), fondendosi delicatamente con lo sfondo.
  - Funzione esclusiva al passaggio del mouse: la finestra torna istantaneamente opaca al 100% appena il cursore si avvicina, per un controllo comodo e intuitivo.
• 🧲 Calibrazione e aggancio magnetico su configurazioni multi-monitor:
  - Risolti i disallineamenti con monitor multipli: all'apertura simultanea, la finestra principale e il visualizzatore si attraggono e si uniscono perfettamente.
• 💡 Cerchio pulsante e suggerimenti visivi per i nuovi utenti:
  - Introdotte guide visive interattive per scoprire rapidamente come bloccare in primo piano o rendere trasparente la finestra.
```

## 1. Nome del prodotto (Product Name)
```text
Win2Pod Bridge
```

---

## 2. Descrizione breve (Short Description - Max. 270 caratteri)
*(Visualizzato nei risultati di ricerca di Microsoft Store, nelle categorie e nei banner promozionali)*

```text
Unisci Windows e Apple! Trasmetti l'audio del PC su HomePod in ALAC lossless o ricevi l'audio AirPlay da iPhone/iPad con gamma dinamica massima Unity Gain! Include passaggio AirPods, visualizzatore 3D, telecomando multimediale ed EQ a 10 bande.
```
*(Conteggio caratteri: circa 248 caratteri, ampiamente sotto il limite di 270)*

---

## 3. Descrizione completa (Full Description)
*(Copia e incolla questo testo nel campo «Descrizione / Description» in Microsoft Partner Center)*

```text
Trasforma il tuo PC Windows in un potente centro di trasmissione audio wireless ad alta fedeltà AirPlay 2!

Win2Pod Bridge è un bridge di streaming audio moderno, leggero e ad alte prestazioni progettato appositamente per Windows 10 e Windows 11. Grazie alla tecnologia nativa WASAPI Loopback a bassissima latenza, cattura l'audio di sistema o di singole applicazioni e lo trasmette in formato ALAC senza perdita (Apple Lossless Audio Codec, 44,1 kHz / 16 bit stereo) direttamente ad Apple HomePod, HomePod mini, Apple TV e a qualsiasi diffusore compatibile con AirPlay 2.

Windows non supporta nativamente AirPlay 2 e gli altoparlanti Apple HomePod richiedono un abbinamento crittografato transitorio HomeKit. Win2Pod Bridge risolve completamente questo problema con un'implementazione software pura in Rust. Nessuna modifica hardware, nessun chip MFi e nessun pesante servizio in background richiesto!

============================================================
🌟 FUNZIONALITÀ CHIAVE & VANTAGGI (KEY FEATURES)
============================================================

• 🛡️ Auto Pre-Amp ed Equalizzatore professionale a 10 bande:
  - L'equalizzatore a 10 bande gestisce automaticamente il margine di dinamica. L'incremento di qualsiasi banda applica una corrispondente attenuazione al pre-amplificatore, eliminando la distorsione digitale con un limitatore di picco trasparente e lineare.
• 🔄 Ripristino automatico dell'endpoint audio (Self-Healing Audio Routing):
  - Monitoraggio continuo dei dispositivi audio di Windows. Se gli altoparlanti predefiniti o il dispositivo proxy vengono scollegati, Win2Pod passa istantaneamente a un altro dispositivo attivo, garantendo la continuità dell'audio di sistema.
• 🔔 Tooltip della barra delle applicazioni su 2 righe compatte (Compact Two-Line Tooltip):
  - Design chiaro a due righe: Riga 1: stato di riproduzione HomePod / AirPlay e volume (es. "🔊 In riproduzione: Studio (9%)" o Muto); Riga 2: carica e stato di inserimento degli AirPods (es. "🎧 AirPods 3: ● L 100%"). Informazioni pulite e senza troncamenti.
• 🎧 Handoff intelligente bidirezionale AirPods ↔ HomePod (Smart Handoff Routing):
  - Quando indossi gli auricolari, l'audio passa automaticamente agli AirPods, per poi riprendere fedelmente su HomePod non appena vengono riposti nella custodia!
• Nuovo bilanciamento stereo con puntatore dinamico (◄ Centro ►):
  - Guida direzionale intuitiva: Mostra «◄ Centro ►» in posizione centrale e passa dinamicamente a «◄ S 30%» o «30% D ►» durante la regolazione.
  - Ricentraggio rapido con un clic: Fai clic sull'etichetta di testo o fai doppio clic sul cursore per ripristinare all'istante il centro esatto.
  - Elaborazione DSP a 32 bit float puramente software: Risolve i conflitti di volume dei driver Windows, garantendo il 100% della gamma dinamica senza cali di volume involontari né distorsione digitale.
• Apertura rapida dell'app sorgente con un clic: Fai clic sulla copertina in vinile o sul titolo del brano in Win2Pod Player per portare istantaneamente in primo piano l'applicazione sorgente (Spotify, Apple Music, Tidal, Chrome, Edge, ecc.).
• Supporto completo di bilanciamento e modalità mono per cuffie e altoparlanti locali (Modalità AO): Che tu stia trasmettendo su HomePod o ascoltando tramite cuffie collegate al PC, goditi un bilanciamento stereo fluido e il passaggio immediato a mono (MONO).
• Ripristino simmetrico del ciclo di vita audio e Protezione Anti-Feedback Shield:
  - Registra fedelmente lo stato iniziale dei dispositivi Windows all'avvio e lo ripristina al 100% alla chiusura.
  - Evita larsen e ritorni audio quando sono collegati solo gli auricolari AirPods, mantenendo attiva la visualizzazione dello spettro in tempo reale con modalità a basso consumo.

🎨 WIN2POD MOTION - VISUALIZZATORE AUDIO 3D IMPRESSIONISTA
• Dinamismo visivo vibrante (Paint Pulse): Reagisce in tempo reale all'energia dello spettro sonoro, passando dai bassi rosso intenso ai bagliori violetti degli acuti, disperdendosi a ritmo di musica.
• Tratti impressionisti (PainterlyPass): Pennellate tridimensionali ancorate alle superfici degli oggetti 3D con dimensione del pennello e angolazione visuale regolabili.
• Schermo intero e Finestra sempre in primo piano: Attiva lo schermo intero (tasto F) o fissa la finestra al di sopra delle altre app (📌). Inibisce automaticamente la sospensione dello schermo a schermo intero.
• Finestra di riepilogo delle scorciatoie da tastiera (tasto ?).
• 100% locale e offline: Motore Three.js interamente integrato, nessuna richiesta di rete esterna, avvio immediato.
• Cambio istantaneo tra 11 lingue in tempo reale.

🎛️ CONTROLLO REMOTO MULTIMEDIALE BIDIREZIONALE INTELLIGENTE
• Supporto dei gesti tattili sulla superficie superiore di HomePod:
  - 1 tocco: Riproduci / Pausa
  - 2 tocchi: Traccia successiva
  - 3 tocchi: Traccia precedente
• Controllo remoto dall'app «Casa» (Apple Home) e dal Centro di controllo su iPhone e iPad!
• Protocollo standard AirPlay 2 updateMRSupportedCommands e canale eventi inverso crittografato, perfettamente sincronizzato con i comandi Windows GSMTC.
• Invio in tempo reale del codice temporale e dei metadati DMAP a HomePod.

✨ HARMONIC EXCITER (HE) - RIPRISTINO DELLE ARMONICHE ACUTE
• Aria e brillantezza cristallina: Compensa il decadimento delle alte frequenze nei file compressi e Bluetooth. Rigenera armoniche naturali da 8 kHz a 20 kHz.
• 3 preset di mastering: Naturale (Natural), Dinamico (Dynamic) e Brillante (Brilliant).

📻 WARMTH ENHANCER (WE) - CALORE ANALOGICO DEL VINILE
• Suono vintage: Elimina la freddezza dell'audio digitale ricreando la fisica dei giradischi classici.
• Filtro Low-Shelf (50Hz–90Hz) per bassi rotondi e profondi.
• Saturazione analogica a valvole (2,5% - 5,0%) con limitatore morbido anti-clipping.
• 3 atmosfere: Delicata (Subtle), Calda (Warm) e Vintage.

🎛️ CATENA DI ELABORAZIONE DSP PROFESSIONALE (HE ➔ EQ ➔ WE)
• Ordine audiofilo: Ripristino delle armoniche (HE), equalizzazione a 10 bande (EQ), risonanza analogica e protezione dei picchi (WE).
• Sospensione automatica nel silenzio: Quando non c'è suono, il DSP entra in standby allo 0% di CPU e si risveglia in 0 ms alla ripresa del brano!

📊 ANALIZZATORE DI SPETTRO IN USCITA (POST-DSP)
• Monitoraggio a 16 bande: Visualizza la forma d'onda finale esatta inviata agli altoparlanti.

⚡ FREQUENZA DEI FOTOGRAMMI ADATTIVA E MASSIMO RISPARMIO ENERGETICO
• 30 FPS fluidi durante l'ascolto; riduzione automatica a 2 FPS nel silenzio, abbattendo l'uso di CPU a ~0,5%.
• 0,0% di CPU nella barra delle applicazioni: Sospende il rendering quando minimizzato (solo 8–15 MB di RAM).

📻 LETTORE VINTAGE HI-FI WIN2POD PLAYER
• Design classico con copertina dell'album, display fluorescente VFD, spettro a 16 bande ed equalizzatore a 10 bande.
• Metadati GSMTC e durata in tempo reale (mm:ss / mm:ss).
• Barra di avanzamento fluida senza scatti.

📌 FINESTRA SEMPRE IN PRIMO PIANO (ALWAYS ON TOP 📌)
• Mantieni il pannello o il lettore sempre visibile sopra giochi e finestre di lavoro.

🎯 INDIRIZZAMENTO AUDIO PER SINGOLA APPLICAZIONE
• Invia l'audio di Spotify o del browser a HomePod lasciando chiamate Discord e giochi sulle cuffie del PC.

🎵 PROGETTATO SPECIFICAMENTE PER APPLE HOMEPOD E AIRPLAY 2
• Abbinamento crittografato HomeKit Transient compatibile con HomePod e HomePod mini.
• Streaming ALAC senza perdita con qualità da studio (44,1 kHz, 16 bit stereo).
• Sincronizzazione dell'orologio gPTP IEEE 802.1AS ad alta precisione.

🎚️ EQUALIZZATORE GRAFICO A 10 BANDE E PRESET
• Regolazione precisa da 31 Hz a 16 kHz (±12 dB) con 8 preset dedicati (Bass Boost, Vocal, Rock, ecc.).

🔊 COPPIE STEREO HOMEPOD E TRASMISSIONE MULTI-ROOM
• Riconoscimento automatico delle coppie stereo HomePod per un palcoscenico tridimensionale.
• Trasmissione simultanea a più diffusori AirPlay 2 in stanze diverse.
• Sincronizzazione bidirezionale con i pulsanti fisici del volume su HomePod.

============================================================
REQUISITI DI SISTEMA
============================================================
• SO: Windows 10 (versione 1809 o successiva) o Windows 11 (64 bit).
• Rete: PC e diffusori devono trovarsi sulla stessa rete locale (Wi-Fi o Ethernet).
• Firewall di Windows: Consentire le connessioni in entrata alla prima richiesta.
```

---

## 4. Caratteristiche principali (Product Features - Max. 120 caratteri per punto)
*(Incolla ciascun punto nel campo «Funzionalità / Features» in Partner Center)*

1. `Apre il player con un clic su copertina o titolo: Spotify, Apple Music e Tidal in 1º piano`
2. `Bilanciamento stereo con puntatore dinamico (◄ Centro ►): DSP a 32 bit, un clic per ricentrare`
3. `Win2Pod Motion: visualizzatore audio 3D impressionista con schermo intero e blocco in primo piano`
4. `Streaming audio ALAC 44,1 kHz / 16 bit stereo in tempo reale e senza perdita verso AirPlay 2`
5. `Abbinamento crittografato HomeKit Transient: 100% Rust, senza bisogno di chip hardware MFi`
6. `Harmonic Exciter (HE): ripristina la presenza e la brillantezza delle armoniche acute`
7. `Warmth Enhancer (WE): risonanza analogica da vinile, saturazione morbida e limitatore di picco`
8. `Catena DSP professionale (HE ➔ EQ ➔ WE) con equalizzatore a 10 bande e standby allo 0% di CPU`
9. `Analizzatore di spettro Post-DSP in tempo reale: visualizza l'acustica effettiva in uscita`
10. `Frequenza dei fotogrammi adattiva: scende a 2 FPS in silenzio riducendo l'uso di CPU a ~0,5%`
11. `Win2Pod Player: telaio Hi-Fi vintage con copertina d'album, metadati GSMTC e barra fluida`
12. `Sempre in primo piano (📌): mantiene il pannello e il lettore al di sopra di tutte le finestre`
13. `Routing per applicazione: riproduci musica su HomePod lasciando chiamate e giochi nelle cuffie`
14. `Equalizzatore grafico di precisione a 10 bande con 8 preset musicali dedicati`
15. `Riconoscimento automatico delle coppie stereo HomePod per un suono avvolgente e immersivo`
16. `Trasmissione multi-room: invia l'audio del PC a più diffusori AirPlay 2 contemporaneamente`
17. `Gestione dinamica dei gruppi: aggiungi o rimuovi altoparlanti in corsa senza interrompere il brano`
18. `Sincronizzazione volume hardware: i pulsanti su HomePod regolano i cursori in Windows`
19. `0,0% di CPU nella barra delle applicazioni: arresta il rendering occupando solo 8-15 MB di RAM`
20. `Silenziamento automatico altoparlanti PC: evita echi in streaming e ripristina il volume all'uscita`

---

## 5. Termini di ricerca (Search Terms / Keywords - Max. 7 termini, 30 caratteri ciascuno)
*(Compila il campo «Termini di ricerca / Search terms»)*

1. `AirPlay`
2. `HomePod`
3. `Audio`
4. `Equalizzatore`
5. `Streaming`
6. `Musica`
7. `Lettore`
