# Fiche Microsoft Store - Français (fr-FR)
# Win2Pod Bridge - Version 0.1.16

## 0. Nouveautés de cette version (What's new in this version - À coller dans Partner Center)
```text
[Mise à jour majeure de Win2Pod Bridge 0.1.16]
• 🔔 Infobulle de la zone de notification repensée sur 2 lignes épurées : L'infobulle de l'icône dans la barre des tâches adopte désormais une présentation dédiée sur deux lignes. La 1re ligne indique l'état de diffusion HomePod / AirPlay et le volume (ex. « 🔊 Lecture : Salon (9%) » ou Muet) ; la 2e ligne affiche l'autonomie et le port des AirPods (ex. « 🎧 AirPods 3 : ● L 100% »). Fini les textes tronqués et encombrés.
• 🔄 Reflet dynamique de l'état AirPods ↔ HomePod : Dès que la priorité automatique bascule le son vers les écouteurs, l'infobulle affiche le mode d'amélioration locale, puis revient automatiquement à la diffusion HomePod dès le rangement dans le boîtier.
• 🛡️ Auto Pre-Amp & Gestion dynamique du Headroom : L'égaliseur 10 bandes intègre une gestion intelligente du headroom. L'amplification de n'importe quelle bande entraîne automatiquement une atténuation équivalente du préamplificateur, éliminant toute distorsion numérique grâce à un limiteur de crête transparent et ultra-linéaire.
• 🔄 Basculement automatique des périphériques audio (Fallback) : Surveillance proactive des sorties audio Windows. Si vos haut-parleurs par défaut ou le périphérique proxy sont déconnectés pendant l'utilisation ou avant la fermeture, Win2Pod bascule instantanément vers un autre périphérique physique actif, garantissant que l'audio Windows ne reste jamais bloqué.
• 🎧 Isolation d'état et synchronisation du volume des AirPods : Résolution dynamique des GUID matériels afin de synchroniser fidèlement le volume réel des AirPods lors de la transition depuis le HomePod, avec prise en charge du contrôle tactile sur la tige.
• ⚡ Reprise de lecture intelligente événementielle (Event-Driven Handoff) : Architecture optimisée pour la détection du boîtier, reprenant la lecture du lecteur multimédia de manière fiable et sans décalage temporel lors du rangement dans les 30 secondes.
• 🛡️ Protection Anti-Feedback Shield : Empêche tout effet Larsen en environnement à périphérique unique (AirPods seuls) grâce à l'analyse des identifiants matériels, tout en maintenant l'analyseur de spectre actif avec mise en veille écoénergétique.
• 🚀 Optimisations structurelles et performances : Allègement des routines de routage et stabilité renforcée des boucles de messages pour un streaming longue durée sans faille.
```

## 1. Nom du produit (Product Name)
```text
Win2Pod Bridge
```

---

## 2. Description courte (Short Description - Max. 270 caractères)
*(Affiché dans les résultats de recherche du Microsoft Store et les cartes promotionnelles)*

```text
Diffusez l'audio de Windows 10 & 11 vers Apple HomePod et enceintes AirPlay 2 en qualité ALAC sans perte. Nouveautés : visualiseur 3D Win2Pod Motion, ouverture du lecteur au clic sur la pochette, balance stéréo à pointeur dynamique, gestes HomePod et EQ 10 bandes.
```
*(Nombre de caractères : environ 262 caractères, inférieur à la limite de 270)*

---

## 3. Description complète (Full Description)
*(Copiez et collez ce texte dans le champ « Description » de l'Espace partenaires Microsoft)*

```text
Transformez votre PC Windows en un puissant émetteur audio haute-fidélité sans fil AirPlay 2 !

Win2Pod Bridge est un pont de streaming audio moderne, ultra-léger et haute performance, conçu spécifiquement pour Windows 10 et Windows 11. Grâce à la technologie native WASAPI Loopback à très faible latence, il capture le son de votre système ou d'une application spécifique pour le diffuser au format ALAC sans perte (Apple Lossless Audio Codec, 44,1 kHz / 16 bits stéréo) directement vers Apple HomePod, HomePod mini, Apple TV et toute enceinte compatible AirPlay 2.

Windows ne prend pas en charge nativement AirPlay 2, et les HomePod d'Apple exigent un jumelage transitoire chiffré HomeKit. Win2Pod Bridge résout ce problème grâce à une implémentation logicielle complète en pur Rust. Aucune modification matérielle, aucune puce MFi et aucun service tiers encombrant requis !

============================================================
🌟 FONCTIONNALITÉS CLÉS & AVANTAGES (KEY FEATURES)
============================================================

• 🛡️ Auto Pre-Amp & Égaliseur professionnel 10 bandes :
  - L'égaliseur 10 bandes intègre une gestion intelligente du headroom. L'amplification de n'importe quelle bande entraîne automatiquement une atténuation équivalente du préamplificateur, éliminant toute distorsion numérique grâce à un limiteur de crête transparent et ultra-linéaire.
• 🔄 Basculement automatique des périphériques audio (Self-Healing Audio Routing) :
  - Surveillance proactive des sorties audio Windows. Si vos haut-parleurs par défaut ou le périphérique proxy sont déconnectés pendant l'utilisation ou avant la fermeture, Win2Pod bascule instantanément vers un autre périphérique physique actif, garantissant que l'audio Windows ne reste jamais bloqué.
• 🔔 Infobulle de la zone de notification sur 2 lignes épurées (Compact Two-Line Tooltip) :
  - Présentation claire sur deux lignes : la 1re ligne indique l'état de diffusion HomePod / AirPlay et le volume (ex. « 🔊 Lecture : Salon (9%) » ou Muet) ; la 2e ligne affiche l'autonomie et le port des AirPods (ex. « 🎧 AirPods 3 : ● L 100% »), évitant tout texte tronqué.
• 🎧 Handoff intelligent bidirectionnel AirPods ↔ HomePod (Smart Handoff Routing) :
  - Dès que vous mettez vos écouteurs, le son bascule instantanément sur les AirPods, et revient fidèlement sur le HomePod dès que vous les rangez dans leur boîtier !
• Nouvelle balance stéréo avec indicateur de direction dynamique (◄ Centre ►) :
  - Repère directionnel intuitif : Affiche « ◄ Centre ► » en position centrale et bascule dynamiquement sur « ◄ G 30% » ou « 30% D ► » lors des réglages.
  - Recentrage instantané : Cliquez sur le libellé texte ou double-cliquez sur le curseur pour rétablir immédiatement le centre exact.
  - Traitement DSP 32 bits virgule flottante purement logiciel : Évite les conflits de volume matériel des pilotes Windows et préserve 100 % de la dynamique sans baisse involontaire ni distorsion numérique.
• Ouverture instantanée du lecteur source en un clic : Cliquez sur la pochette vinyle ou sur le titre du morceau dans Win2Pod Player pour afficher et placer immédiatement au premier plan l'application de lecture (Spotify, Apple Music, Tidal, Chrome, Edge, etc.).
• Prise en charge intégrale de la balance et du mode mono pour casque et enceintes locales (Mode AO) : Que vous diffusiez vers HomePod ou écoutiez sur le casque connecté à votre PC, profitez d'une balance stéréo fluide et d'un basculement instantané en mono (MONO).
• Restitution symétrique du cycle de vie audio & Protection Anti-Feedback Shield :
  - Enregistre fidèlement l'état initial des périphériques Windows au lancement et le rétablit à 100 % à la fermeture.
  - Empêche tout effet Larsen en environnement à périphérique unique (AirPods seuls) grâce à l'analyse des identifiants matériels, tout en maintenant l'analyseur de spectre actif avec mise en veille écoénergétique.

🎨 WIN2POD MOTION - VISUALISEUR AUDIO 3D IMPRESSIONNISTE
• Mouvement audio dynamique (Paint Pulse) : Réagit en temps réel à l'énergie du spectre audio – des pulsations graves rouge profond aux lueurs violettes des aigus, la sphère se disperse et s'illumine au rythme de la musique.
• Rendu de touches impressionnistes (PainterlyPass) : Touches de pinceau 3D ancrées sur la géométrie. Taille de pinceau et rotation de la caméra ajustables.
• Plein écran dédié & Épinglage au premier plan : Basculez en plein écran (touche F) ou épinglez la fenêtre au-dessus des autres applications (📌). Empêche automatiquement la mise en veille de l'écran en mode plein écran.
• Fenêtre d'aide pour raccourcis clavier (touche ?).
• 100 % hors ligne & local : Moteur Three.js totalement intégré, aucune requête réseau externe, démarrage instantané.
• Changement instantané parmi 11 langues en temps réel.

🎛️ TÉLÉCOMMANDE MULTIMÉDIA BIDIRECTIONNELLE INTELLIGENTE
• Prise en charge des gestes tactiles sur le dessus du HomePod :
  - 1 toucher : Lecture / Pause
  - 2 touchers : Piste suivante
  - 3 touchers : Piste précédente
• Contrôle à distance depuis l'application « Maison » (Apple Home) et le Centre de contrôle sur iPhone et iPad !
• Déclaration standard AirPlay 2 updateMRSupportedCommands et canal d'événements inversé chiffré, parfaitement synchronisé avec les commandes Windows GSMTC.
• Envoi en temps réel du code temporel et des métadonnées DMAP vers le HomePod.

✨ HARMONIC EXCITER (HE) - RESTAURATION DES HARMONIQUES AIGUS
• Présence & Clarté : Compense la perte de hautes fréquences des flux compressés et Bluetooth. Régénère les harmoniques de 8 kHz à 20 kHz.
• 3 préréglages : Naturel (Natural), Dynamique (Dynamic), Brillant (Brilliant).

📻 WARMTH ENHANCER (WE) - CHALEUR ANALOGIQUE VINYLE
• Chaleur vintage : Élimine la froideur du son numérique en recréant l'acoustique d'une platine vinyle classique.
• Filtre de résonance Low-Shelf (50Hz–90Hz) pour des basses rondes et chaleureuses.
• Saturation analogique (2,5 % à 5,0 %) avec limiteur doux anti-écrêtage.
• 3 ambiances : Subtil (Subtle), Chaleureux (Warm), Rétro (Vintage).

🎛️ CHAÎNE DE TRAITEMENT AUDIO PROFESSIONNELLE (HE ➔ EQ ➔ WE)
• Traitement audiophile : Restauration des harmoniques (HE), égalisation 10 bandes (EQ), résonance analogique et protection des crêtes (WE).
• Mode veille silencieux : En l'absence de son, le DSP passe en veille à 0 % de processeur et se réveille en 0 ms dès que la musique reprend !

📊 ANALYSEUR DE SPECTRE EN SORTIE (POST-DSP)
• Surveillance 16 bandes : Visualise l'onde sonore finale envoyée aux enceintes après application des effets.

⚡ FRÉQUENCE D'IMAGES ADAPTATIVE & ÉCONOMIE D'ÉNERGIE
• Fluidité 30 FPS en lecture musicale ; réduction automatique à 2 FPS en silence, abaissant l'utilisation du processeur à ~0,5 %.
• 0,0 % de processeur dans la barre des tâches : Arrêt complet des rendus lorsque l'application est réduite (8 à 15 Mo de RAM).

📻 LECTEUR AUDIO VINTAGE HI-FI WIN2POD PLAYER
• Design rétro intégrant pochette, écran fluorescent VFD, analyseur de spectre 16 bandes et égaliseur 10 bandes.
• Métadonnées GSMTC et code temporel en direct (mm:ss / mm:ss).
• Barre de progression fluide sans saccades.

📌 ÉPINGLAGE AU PREMIER PLAN (ALWAYS ON TOP 📌)
• Reste visible au-dessus de vos jeux et fenêtres de travail.

🎯 ROUTAGE AUDIO PAR APPLICATION
• Isolez Spotify ou votre navigateur pour les envoyer vers le HomePod, pendant que vos appels Discord et jeux restent dans votre casque.

🎵 CONÇU SPÉCIFIQUEMENT POUR APPLE HOMEPOD & AIRPLAY 2
• Jumelage chiffré HomeKit Transient avec les HomePod et HomePod mini.
• Encodage ALAC sans perte haute fidélité (44,1 kHz, 16 bits stéréo).
• Synchronisation d'horloge gPTP IEEE 802.1AS de précision éliminant toute gigue.

🎚️ ÉGALISEUR GRAPHIQUE 10 BANDES & PRÉRÉGLAGES
• Réglages précis de 31 Hz à 16 kHz (±12 dB) avec 8 préréglages (Bass Boost, Vocal, Rock, etc.).

🔊 PAIRES STÉRÉO HOMEPOD & DIFFUSION MULTIROOM
• Détection automatique des paires stéréo HomePod pour une scène sonore immersive.
• Diffusion simultanée vers plusieurs enceintes AirPlay 2 dans toute la maison.
• Synchronisation matérielle bidirectionnelle avec les boutons de volume du HomePod.

============================================================
CONFIGURATION REQUISE
============================================================
• Système : Windows 10 (version 1809 ou supérieure) ou Windows 11 (64 bits).
• Réseau : Le PC et les enceintes doivent être sur le même réseau local (Wi-Fi ou Ethernet).
• Pare-feu Windows : Autoriser les connexions entrantes lors de la première invite.
```

---

## 4. Caractéristiques principales (Product Features - Max. 120 caractères par point)
*(Collez chaque point dans le champ « Fonctionnalités / Features » de l'Espace partenaires)*

1. `Ouverture du lecteur au clic sur pochette ou titre : Spotify, Apple Music et Tidal au 1er plan`
2. `Balance stéréo à pointeur dynamique (◄ Centre ►) : DSP 32 bits, clic pour recentrer immédiatement`
3. `Win2Pod Motion : visualiseur audio 3D impressionniste avec plein écran et épinglage au 1er plan`
4. `Diffusion audio sans perte ALAC 44,1 kHz / 16 bits stéréo en temps réel vers AirPlay 2`
5. `Jumelage chiffré HomeKit Transient : 100 % Rust, sans puce matérielle Apple MFi requise`
6. `Harmonic Exciter (HE) : restaure la présence aérienne et la clarté des harmoniques aigus`
7. `Warmth Enhancer (WE) : résonance analogique vinyle, saturation douce et limiteur anti-crête`
8. `Chaîne DSP professionnelle (HE ➔ EQ ➔ WE) avec égaliseur 10 bandes et veille silencieuse 0% CPU`
9. `Analyseur de spectre Post-DSP en direct : visualise la dynamique acoustique exacte en sortie`
10. `Fréquence d'images adaptative : réduit à 2 FPS en silence, abaissant le CPU à ~0,5 %`
11. `Win2Pod Player : châssis Hi-Fi rétro avec pochette d'album, métadonnées GSMTC et barre de saut`
12. `Épinglage au premier plan (📌) : conserve le panneau principal et le lecteur au-dessus des fenêtres`
13. `Routage audio par application : diffusez la musique sur HomePod tout en gardant Discord au casque`
14. `Égaliseur graphique 10 bandes de précision avec 8 préréglages musicaux dédiés`
15. `Reconnaissance automatique des paires stéréo HomePod pour un champ sonore enveloppant`
16. `Diffusion multiroom : envoyez l'audio du PC vers plusieurs enceintes AirPlay 2 simultanément`
17. `Gestion dynamique des groupes : ajoutez ou retirez des enceintes en cours de lecture sans coupure`
18. `Synchronisation matérielle du volume : les boutons du HomePod contrôlent les curseurs Windows`
19. `0,0 % de processeur en barre des tâches : suspend les rendus et n'occupe que 8 à 15 Mo de RAM`
20. `Mise en sourdine intelligente du PC : évite l'écho en streaming et rétablit le volume à la fin`

---

## 5. Mots-clés de recherche (Search Terms / Keywords - Max. 7 termes, 30 caractères chacun)
*(Remplissez le champ « Termes de recherche / Search terms »)*

1. `AirPlay`
2. `HomePod`
3. `Audio`
4. `Égaliseur`
5. `Streaming`
6. `Musique`
7. `Lecteur`
