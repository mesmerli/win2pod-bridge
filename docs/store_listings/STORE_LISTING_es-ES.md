# Ficha de Microsoft Store - Español (es-ES)
# Win2Pod Bridge - Versión 0.1.21

## 0. Novedades de esta versión (What's new in this version - Para Partner Center)
```text
[Win2Pod Bridge 0.1.21 Actualización - Efecto Genio fluido y Translucidez inteligente]
• 🧞 Efecto de minimización Genio estilo macOS (Muelle Genie Effect):
  - Integración del efecto de compresión S-Curve por línea de escaneo inspirado en el proyecto de código abierto muelle. La ventana se aspira suavemente hacia la barra de tareas al minimizar y se restaura al instante sin tirones.
• 🪟 Translucidez nativa de Windows (WS_EX_LAYERED) e interacción inteligente al pasar el cursor:
  - La ventana flotante del visualizador admite opacidad ajustable (20% ~ 100%) sin interferir con su flujo de trabajo.
  - Función exclusiva «Hover Peek»: al pasar el ratón por encima, la ventana recupera el 100% de opacidad de inmediato para un control táctil y cómodo.
• 🧲 Acoplamiento y atracción magnética de ventanas multimonitor:
  - Corrección de desajustes en configuraciones con múltiples monitores. Al abrir ambas ventanas, se alinean y acoplan magnéticamente de forma natural.
• 💡 Anillo de respiración y guía intuitiva para nuevos usuarios:
  - Nuevas indicaciones visuales elegantes que enseñan a fijar la ventana y ajustar la transparencia en el primer uso.
```

## 1. Nombre del producto (Product Name)
```text
Win2Pod Bridge
```

---

## 2. Descripción breve (Short Description - Máx. 270 caracteres)
*(Se muestra en los resultados de búsqueda de Microsoft Store, categorías y tarjetas de promoción)*

```text
¡Integración perfecta de Windows y Apple! Transmita audio de PC a HomePod en ALAC sin pérdidas o reciba audio AirPlay desde iPhone/iPad con rango dinámico máximo Unity Gain. Incluye relevo AirPods, visualizador 3D, control multimedia remoto y EQ de 10 bandas.
```
*(Longitud: aprox. 252 caracteres, dentro del límite de 270 caracteres)*

---

## 3. Descripción completa (Full Description)
*(Copie y pegue este texto en el campo «Descripción / Description» de Microsoft Partner Center)*

```text
¡Convierta su PC con Windows en un potente centro emisor de audio inalámbrico de alta fidelidad AirPlay 2!

Win2Pod Bridge es un puente de transmisión de audio moderno, ligero y de alto rendimiento diseñado específicamente para Windows 10 y Windows 11. Mediante la tecnología nativa WASAPI Loopback de latencia ultrabaja, captura el audio del sistema o de aplicaciones individuales y lo transmite en formato ALAC sin pérdidas (Apple Lossless Audio Codec, 44,1 kHz / 16 bits estéreo) directamente a Apple HomePod, HomePod mini, Apple TV y cualquier altavoz compatible con AirPlay 2.

Windows no admite AirPlay 2 de forma nativa y los altavoces Apple HomePod exigen un emparejamiento cifrado transitorio HomeKit. Win2Pod Bridge soluciona esto por completo mediante una implementación de software puro en Rust. ¡Sin modificaciones de hardware, sin chips MFi y sin pesados servicios en segundo plano!

============================================================
🌟 CARACTERÍSTICAS PRINCIPALES (KEY FEATURES)
============================================================

• 🛡️ Auto Pre-Amp y Ecualizador profesional de 10 bandas:
  - El ecualizador de 10 bandas incorpora compensación automática de ganancia. Al aumentar cualquier banda, atenúa dinámicamente el preamplificador para evitar saturación y distorsión digital, junto con un limitador de picos transparente y lineal.
• 🔄 Protección y Fallback global de puntos finales de audio (Self-Healing Audio Routing):
  - Supervisión del estado de los dispositivos de Windows. Si los altavoces predeterminados o el dispositivo proxy se desconectan, Win2Pod conmuta automáticamente a otro dispositivo activo, evitando bloqueos en el sonido del sistema.
• 🔔 Notificación de la bandeja del sistema en dos líneas compactas (Compact Two-Line Tooltip):
  - Diseño limpio en dos líneas: la línea 1 muestra el estado de transmisión de HomePod / AirPlay y el volumen (ej.: «🔊 Reproduciendo: Salón (9%)» o Silenciado); la línea 2 muestra la batería y colocación de los AirPods (ej.: «🎧 AirPods 3: ● L 100%»), eliminando textos cortados.
• 🎧 Handoff inteligente bidireccional AirPods ↔ HomePod (Smart Handoff Routing):
  - Al colocarse los auriculares, el sonido conmuta automáticamente a los AirPods, y se reanuda con precisión en el HomePod al guardarlos en el estuche.
• Nuevo balance estéreo con puntero dinámico (◄ Centro ►):
  - Guía direccional intuitiva: Muestra «◄ Centro ►» en el centro y cambia dinámicamente a «◄ I 30%» o «30% D ►» al ajustarlo.
  - Centrado rápido con un clic: Haga clic en la etiqueta de texto o doble clic en el deslizador para volver instantáneamente al centro.
  - Procesamiento DSP de 32 bits flotantes puro por software: Evita fallos de volumen de los controladores de Windows y garantiza el 100% del rango dinámico sin caídas de volumen ni distorsión digital.
• Apertura rápida del reproductor fuente con un clic: Haga clic en la carátula de vinilo o en el título de la canción en Win2Pod Player para traer inmediatamente al frente la aplicación de origen (Spotify, Apple Music, Tidal, Chrome, Edge, etc.).
• Soporte completo de balance y modo mono para auriculares locales (Modo AO): Disfrute de balance estéreo continuo y cambio instantáneo a mono (MONO) tanto si transmite a HomePod como si escucha por auriculares en su PC.
• Restitución simétrica del ciclo de vida de audio y Protección Anti-Feedback Shield:
  - Registra el estado inicial del dispositivo predeterminado de Windows al iniciar y lo restablece fielmente al 100% al cerrar.
  - Detecta riesgos de bucle de retroalimentación en entornos con un solo dispositivo mediante GUID, suprimiendo la reemisión a los auriculares mientras el visualizador continúa funcionando de forma eficiente.

🎨 WIN2POD MOTION - VISUALIZADOR DE AUDIO 3D IMPRESIONISTA
• Movimiento acústico vibrante (Paint Pulse): Reacciona en tiempo real a la energía del espectro sonoro, desde pulsos graves en rojo intenso hasta destellos agudos violetas, dispersándose al compás de la música.
• Pinceladas impresionistas 3D (PainterlyPass): Trazos anclados a la geometría tridimensional con tamaño de pincel y ángulo de visión ajustables.
• Pantalla completa y Fijación siempre visible: Alterne pantalla completa (tecla F) o fije la ventana en primer plano (📌). Evita automáticamente la suspensión del equipo en pantalla completa.
• Superposición de atajos de teclado (tecla ?).
• 100% local y sin conexión: Motor Three.js integrado, sin llamadas de red externas y con arranque instantáneo.
• Cambio inmediato entre 11 idiomas en tiempo real.

🎛️ CONTROL REMOTO MULTIMEDIA BIDIRECCIONAL INTELIGENTE
• Soporte de gestos táctiles en la superficie superior del HomePod:
  - 1 toque: Reproducir / Pausar
  - 2 toques: Pista siguiente
  - 3 toques: Pista anterior
• Control remoto desde la app «Casa» (Apple Home) y el Centro de control en iPhone y iPad.
• Integración estándar AirPlay 2 updateMRSupportedCommands y canal de eventos inversos cifrados, sincronizado con controles GSMTC de Windows.
• Envío en tiempo real del código de tiempo y metadatos DMAP al HomePod.

✨ HARMONIC EXCITER (HE) - RESTAURACIÓN DE ARMÓNICOS AGUDOS
• Aire y presencia cristalina: Compensa la atenuación de altas frecuencias de audios comprimidos y Bluetooth. Regenera armónicos naturales de 8 kHz a 20 kHz.
• 3 ajustes maestros: Natural, Dinámico (Dynamic) y Brillante (Brilliant).

📻 WARMTH ENHANCER (WE) - CALIDEZ ANALÓGICA DE VINILO
• Sonido vintage: Elimina la aspereza digital recreando la calidez física de los tocadiscos analógicos.
• Filtro Low-Shelf (50Hz–90Hz) para graves profundos y envolventes.
• Saturación analógica (2,5% a 5,0%) con limitador suave para evitar distorsiones.
• 3 modos: Sutil (Subtle), Cálido (Warm) y Vintage.

🎛️ CADENA DE PROCESAMIENTO DSP PROFESIONAL (HE ➔ EQ ➔ WE)
• Procesamiento audiófilo: Restauración de armónicos (HE), ecualización de 10 bandas (EQ), resonancia analógica y limitador de picos (WE).
• Modo de suspensión silenciosa: Sin sonido, el DSP entra en reposo al 0% de CPU y se despierta en 0 ms cuando la música se reanuda.

📊 ANALIZADOR DE ESPECTRO POST-DSP EN TIEMPO REAL
• Monitoreo de salida de 16 bandas: Muestra la forma de onda final exacta emitida a los altavoces tras los efectos.

⚡ FRECUENCIA DE CUADROS ADAPTATIVA Y MÁXIMO AHORRO ENERGÉTICO
• 30 FPS fluidos en reproducción; reducción automática a 2 FPS en silencio, bajando la CPU a ~0,5%.
• 0,0% de CPU en bandeja del sistema: Detiene por completo el renderizado al minimizar (solo 8 a 15 MB de RAM).

📻 REPRODUCTOR WIN2POD PLAYER ESTILO RETRO HI-FI
• Diseño vintage con carátula de álbum, pantalla fluorescente VFD, espectro de 16 bandas y ecualizador de 10 bandas.
• Metadatos GSMTC y código de tiempo en vivo (mm:ss / mm:ss).
• Barra de búsqueda fluida y precisa sin saltos ni tirones.

📌 VENTANA SIEMPRE EN PRIMER PLANO (ALWAYS ON TOP 📌)
• Mantenga el panel principal o el reproductor flotando sobre sus juegos y ventanas de trabajo.

🎯 ENCAMINAMIENTO DE AUDIO POR APLICACIÓN
• Envíe la música de Spotify o del navegador al HomePod mientras sus llamadas de Discord y videojuegos permanecen en los auriculares del PC.

🎵 DISEÑADO ESPECÍFICAMENTE PARA APPLE HOMEPOD Y AIRPLAY 2
• Emparejamiento cifrado HomeKit Transient compatible con HomePod y HomePod mini.
• Transmisión ALAC sin pérdidas con calidad de estudio (44,1 kHz, 16 bits estéreo).
• Sincronización de reloj gPTP IEEE 802.1AS de alta precisión sin fluctuaciones ni demoras.

🎚️ ECUALIZADOR GRÁFICO DE 10 BANDAS Y PREAJUSTES
• Ajuste preciso de 31 Hz a 16 kHz (±12 dB) con 8 preajustes (Bass Boost, Vocal, Rock, etc.).

🔊 PARES ESTÉREO HOMEPOD Y DIFUSIÓN MULTISALA
• Reconocimiento automático de pares estéreo de HomePod.
• Transmisión simultánea a múltiples altavoces AirPlay 2 en diferentes habitaciones.
• Sincronización bidireccional con los botones físicos de volumen del HomePod.

============================================================
REQUISITOS DEL SISTEMA
============================================================
• SO: Windows 10 (versión 1809 o superior) o Windows 11 (64 bits).
• Red: El PC y los altavoces deben estar conectados a la misma red local (Wi-Fi o cable Ethernet).
• Firewall de Windows: Permita las conexiones entrantes la primera vez que se solicite.
```

---

## 4. Características principales (Product Features - Máx. 120 caracteres por punto)
*(Pegue cada elemento en el campo «Funciones / Features» de Partner Center)*

1. `Abre el reproductor al pulsar carátula o título: Spotify, Apple Music y Tidal en 1º plano`
2. `Balance estéreo con puntero dinámico (◄ Centro ►): DSP de 32 bits, un clic para centrar`
3. `Win2Pod Motion: visualizador de audio 3D impresionista con pantalla completa y fijación superior`
4. `Transmisión de audio sin pérdidas ALAC 44,1 kHz / 16 bits estéreo en tiempo real a AirPlay 2`
5. `Emparejamiento cifrado HomeKit Transient: 100% Rust, sin requerir chips de hardware Apple MFi`
6. `Harmonic Exciter (HE): restaura la presencia de aire y el brillo de los armónicos agudos`
7. `Warmth Enhancer (WE): resonancia analógica de vinilo, saturación suave y limitador de picos`
8. `Cadena DSP profesional (HE ➔ EQ ➔ WE) con ecualizador de 10 bandas y reposo con 0% de CPU`
9. `Analizador de espectro Post-DSP en vivo: visualiza la acústica exacta de salida a los altavoces`
10. `Frecuencia de cuadros adaptativa: baja a 2 FPS en silencio reduciendo el uso de CPU a ~0,5%`
11. `Win2Pod Player: chasis Hi-Fi retro con carátula de álbum, metadatos GSMTC y barra de salto suave`
12. `Ventana siempre visible (📌): mantiene el panel y el reproductor flotando sobre otras ventanas`
13. `Enrutamiento por aplicación: reproduzca música en HomePod mientras mantiene llamadas en auriculares`
14. `Ecualizador gráfico de precisión de 10 bandas con 8 preajustes dedicados (Bass Boost, Vocal, etc.)`
15. `Detección automática de pares estéreo de HomePod para un escenario sonoro envolvente`
16. `Difusión multisala: envíe el sonido del PC a múltiples altavoces AirPlay 2 simultáneamente`
17. `Gestión dinámica de grupos: agregue o quite altavoces sobre la marcha sin detener la música`
18. `Sincronización de volumen por hardware: los botones del HomePod controlan el volumen en Windows`
19. `0,0% de CPU en la bandeja del sistema: detiene el renderizado y ocupa solo de 8 a 15 MB de RAM`
20. `Silencio automático de altavoces del PC: previene ecos al emitir y restaura el volumen al salir`

---

## 5. Términos de búsqueda (Search Terms / Keywords - Máx. 7 términos, hasta 30 caracteres)
*(Rellene el campo «Términos de búsqueda / Search terms»)*

1. `AirPlay`
2. `HomePod`
3. `Audio`
4. `Ecualizador`
5. `Streaming`
6. `Música`
7. `Reproductor`
