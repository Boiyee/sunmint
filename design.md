# SUNMINT · design.md

Design language: **rain-design-skill-2**, SUNMINT edition. Everything below matches the shipped code in `web/src/styles.css` and `web/src/pages/*`. Use these values exactly to rebuild or extend a screen.

---

## 1. Idea

SUNMINT turns hours of sunlight into certificates. The interface does two things at once:

- **Feels alive.** Sites glow by live output, certificates appear in a feed, a pending hour pulses while it waits to be minted.
- **Reads like evidence.** Every number is set in a dot-matrix face, like a meter display. Hashes, nonces and signatures stay visible in mono, and every certificate links to its chain of custody.

One accent, leaf green, carries three meanings: brand, "verified", and "clean energy". Hours that displaced a generator use the **same green, hatched**, so the carbon story never needs a second hue.

---

## 2. Colour tokens

Set on `:root`; light mode overrides under `:root[data-theme="light"]`. Theme is stored in `localStorage['sunmint-theme']`, default dark.

| Token | Dark | Light | Use |
|---|---|---|---|
| `--bg` | `#0A0C10` | `#F4F5F7` | Page ground (cool ink) |
| `--panel` | `#11141A` | `#FFFFFF` | Panels, cards |
| `--sunk` | `#0A0C10` | `#F4F5F7` | Items inside panels, code blocks |
| `--line` | `#1E232C` | `#DDE1E7` | Panel borders, table rules |
| `--line-2` | `#2A303B` | `#C9CED6` | Button borders, inactive strokes |
| `--fg` | `#E9ECF1` | `#0A0C10` | Primary text |
| `--fg-2` | `#B7BFCC` | `#3D4452` | Secondary text, code |
| `--muted` | `#8A93A3` | `#5A6271` | Labels, captions |
| `--faint` | `#5A6271` | `#8A93A3` | Skipped/disabled states |
| `--dot` | `#2A3240` | `#CDD3DB` | World-map dots |
| `--accent` | `#4BE38B` | `#1F9D55` | Brand, verified, matched, CTAs |
| `--accent-ink` | `#06120B` | `#FFFFFF` | Text on accent fills |
| `--accent-deep` | `#0E1A13` | `#E6F7EC` | Accent-tinted surfaces |
| `--accent-line` | `#1E3A2A` | `#BFE6CE` | Borders on accent-tinted surfaces |
| `--accent-glow` | `rgba(75,227,139,.4)` | `rgba(31,157,85,.22)` | Glows, text-shadow |
| `--amber` | `#FFB020` | `#B7791F` | Pending, partly matched, downgraded |
| `--red` | `#FF4D5E` | `#D6293B` | Rejected, gaps (fills, borders) |
| `--red-fg` | `#FF6B78` | `#C21F31` | Rejected (text) |
| `--hatch` | `repeating-linear-gradient(45deg, var(--accent) 0 2px, var(--accent-deep) 2px 5px)` | same | Generator-displaced hours |

**Status mapping.** green = pass / verified / minted / matched · amber = pending / partly / downgraded · red = rejected / gap · faint = skipped.

**GRID vs GEN.** Always a 14 px swatch (`.sw`): solid accent = GRID, `--hatch` = GEN. Charts use an SVG pattern of the same idea: 45° stripes, 2 px accent at .45 opacity on `--accent-deep`, 7 px period. Never rely on colour alone; GEN is always also labelled "GEN".

---

## 3. Typography

Self-hosted via `@fontsource` (latin subsets), so the PWA works offline.

| Role | Face | Weight | Notes |
|---|---|---|---|
| Headlines | Inter Tight | 900 (800 for h4) | Tracking −.045em on big sizes, −.02em on small |
| Accent word / all numbers | Doto (dot-matrix) | 900 | Colour `--accent` for accent words; tracking −.02em |
| Body | Geist | 400 / 500 / 600 | 15 px / 1.5 |
| Labels, codes, hashes | IBM Plex Mono | 400 / 500 / 600 | Labels uppercase, 11 px, tracking .16em |

**Rule:** exactly one Doto accent word per headline ("**minted**", "**proof**", "**cheat**", "**Trace**", "**proven**"). Every kWh, kg, %, price and count is Doto.

| Class | Size / line-height |
|---|---|
| `.h1` | `clamp(44px, 7vw, 96px)` / .95 |
| `.h2` | `clamp(32px, 5vw, 60px)` / .98 |
| `.h3` | `clamp(26px, 3.4vw, 40px)` / 1 |
| `.h4` | 20 px |
| `.lead` | 18 px, `--muted`, max 540 px |
| `.label` | 11 px Plex Mono 500, .16em, uppercase, `--muted` |
| `.stat .num` | 44 px Doto (36 px under 640 px) |
| Table body | 13 px Plex Mono; numbers 16 px Doto |

---

## 4. Layout

- **Landing:** `.wrap`, max 1160 px, 24 px gutters. Hero is full-bleed, `min-height: 80vh`, not a card.
- **App pages:** `.page`, max 1480 px, padding 24 px (16 px under 640 px), 16 px gaps.
- **Two-column rows:** `.row` (flex-wrap, gap 16, `align-items: flex-start` so panels never stretch). Main column `.grow` = `flex: 999 1 600px`, side column `.side` = `flex: 1 1 340px`. Side drops below main on narrow screens.
- **Stat grids:** `.grid` = `repeat(auto-fit, minmax(var(--min, 220px), 1fr))`.
- **Radii:** panels 16 px, items 12 px, code 10 px, swatches 3 px, buttons/chips/inputs 999 px.
- **No horizontal page scroll.** Wide tables sit in `.scroll` (`overflow-x: auto`).

---

## 5. Components

### Floating navbar (landing)
Sticky, `top: 16px`, max 1120 px, pill radius, `color-mix(panel 80%, transparent)` + `backdrop-filter: blur(12px)`, 1 px `--line`. Left: logo (30 px mark + "SUNMINT" in Plex Mono 600 12 px, .16em). Right: DAYLIGHT · MATCH · TRACE · **BREAK IT** (accent), theme toggle, primary "Open the app".

### App header
Sticky top bar, 1 px bottom rule. Logo · role switch (`.seg`: PRODUCER / COMPANY) · tabs · **Break it** tab in accent. Right: network chip (green dot "BASE SEPOLIA", amber dot "DRY RUN") and theme toggle.
- Producer tabs: Daylight · Rooftop · Trace · Break it
- Company tabs: Daylight · Match · Trace · Break it
- `.tab`: 9×14 px padding, pill; active = `--panel` fill + `--line` border.

### Panels
`.panel`: `--panel`, 1 px `--line`, radius 16, padding 22. **Featured panel** (`.panel.feature`): border `--accent` + `box-shadow: 0 0 70px -20px var(--accent-glow)`. One featured panel per screen at most: Match (match clock), Trace (certificate summary), Proof (receipt), landing "Try to cheat".

### Buttons
- `.btn`: min-height 44, padding 0 20, pill, 1 px `--line-2`, Geist 500 15 px.
- `.btn.primary`: accent fill, `--accent-ink` text, `box-shadow: 0 0 28px var(--accent-glow)`.
- `.btn.lg`: 50 px, 16 px text. `.btn.sm`: 34 px, Plex Mono 11 px .1em.
- Disabled: opacity .5, `cursor: progress`.

### Chips and segmented control
`.chip`: 7×12 px, pill, 1 px `--line`, Plex Mono 11 px .1em, `--muted`. `.seg`: 3 px padded pill; selected child = `--fg` fill, `--bg` text; uses `aria-pressed`.

### Swatches
`.sw` 14×14 px, radius 3: plain = GRID, `.gen` = hatched, `.part` = accent at 38%, `.gap` = red.

### Items and lists
`.item`: `--sunk`, 1 px `--line`, radius 12, 12×14 px. `.item.good`: `--accent-deep` + `--accent-line`. `.item.bad`: red at 7% fill, red at 55% border.

### Tables
Header 10 px Plex Mono .14em `--muted`; rows 11×12 px, 1 px top rule, `white-space: nowrap`. Numbers in Doto 16 px; kWh in accent.

### Code block
`pre.code`: `--sunk`, radius 10, 13 px Plex Mono / 1.75, each line a block span with 16 px side padding. Tampered lines `.ed`: red at 14% background, `--red-fg` text, suffix "← edited" (or "← already used" on replays).

### Chain-of-custody timeline (Trace)
Two-column grid (40 px / 1fr). Node: 34 px circle, 2 px accent ring, step number in Plex Mono 600. Stem: 2 px `--accent-line`. Last node filled accent. Each step: h4 title, status badge right (accent, or red-fg if failed), body, then `.fact` pills (radius 8, Plex Mono 12).

### Logo, the "sun-coin"
40-unit viewBox. Outer ring r 15, stroke 5, `stroke-dasharray: 2.2 1.73` (rays cut like a coin's milled edge). Inner disc r 9. Both accent. Favicon: same on a `#0A0C10` 9-unit rounded square.

---

## 6. Data visualisations

### Hero art, "Sun to block" (landing, SVG 600×540)
Dashed sun arc (`--line-2`, `4 6`), sun = glow disc r 70 at .08 + sun-coin, three dashed rays (`6 6`) onto four parallelogram panels over a house. Generator box at right with dashed smoke circles and a red strike, labelled "GEN · OFF". A dashed trace runs from the meter down to a row of hour blocks (62×54, radius 6): 08:00–13:00 with kWh in Doto, 10:00 and 11:00 hatched with a "GEN" tag, 14:00 is the pulsing "MINTING" block, 15:00 dashed and empty.

### Daylight world map
96×40 dot-matrix mask (`web/src/worldmask.ts`), equirectangular, lat 78°N–56°S, dots 3 px `--dot`. Pin position: `left = (lon+180)/360`, `top = (78−lat)/134`. Lit site: 12 px accent dot with 18 px glow, a pulse ring of diameter `18 + 44·√(live/max)` px, pill tag "City, CC 4.2 kW". Night site: `--line-2` dot, "night" in muted. Grid-down sites add "· GEN OFF".

### Rooftop 24-hour curve (SVG 1000×330)
Plot x 44–970, y 44–290. Four horizontal gridlines with kWh labels. Generator hours = full-height hatched bands with "GEN" above. Area gradient accent .28→0, line 3 px accent, round joins. Now-line dashed `2 4` in `--fg`. Above the plot: ● 5 px accent dot per minted hour; ◌ amber pulsing ring for the hour in progress. X labels every 3 hours in **site local time** (`utc_offset`).

### Match clock (SVG 360×360)
24 segments, 15° each, 0.8° gaps, 00 at top. Outer ring (r 146–168) = demand: `--line` if fully matched, amber if partly, red if a gap. Inner ring (from r 112, depth 28 × matched fraction, min 25%) = matched supply: accent, or hatched when generator hours supplied most of it. Centre: match % in Doto 64 px with glow, "24/7 MATCH" label.

### Proof coverage strip
24 cells in a row, 34 px tall, radius 4, 3 px gaps: accent = covered, hatched = covered by GEN hours, outlined = not covered.

---

## 7. Screens

| Route | Name | Composition |
|---|---|---|
| `/` | Landing | Floating nav · 80vh hero (copy left, Sun-to-block art right, GRID/GEN legend chips, "Open the app" + "Trace a certificate") · live totals (kWh, kg CO₂, certificates) · How it works (5 numbered panels: Meter signs, Verified, Weighed, Minted, Retired) · For producers / For companies · featured "Go on. Try to **cheat**." with the five attack outcomes · "Don't trust us. **Check** the chain." with contract facts in a code block · one-line mono footer |
| `/daylight` | Daylight | Map panel (headline "The sun is up over **N** sites.", kW now, kWh, kg CO₂) + side feed (6 newest certificates with source swatch, CO₂ and tx; 2 newest rejections below) |
| `/rooftop/:slug` | Rooftop | Site line (city, kWp, generator, local time) · headline "This roof kept the generator off for **N** hours." · site picker · 4 stats · 24 h curve + signed-readings stream (rejections on top) · certificates table · "The key never leaves the meter." panel |
| `/match` | Match | Featured match panel (clock, CO₂ if retired, gap hours, beneficiary input, "Retire N certificates · X tUSDC", CSV upload) + market table with PER KWH / PER TONNE CO₂ toggle and the cheapest-impact insight line |
| `/trace/:id` | Trace | "Don't trust it. **Trace** it." · search pill · timeline (Device, Signed readings, Verifier checks, Displacement evidence, Minted, Retired) with raw readings in a `<details>` · side: featured summary, "Try to forge one like it →", "How CO₂ was counted" |
| `/proof/:n` | Proof | Featured receipt (tonnes in Doto 72–108 px, kWh / certificates / sites, 24-cell strip, tx) · Download PNG · Copy link · Trace a certificate · certificate list |
| `/break-it` | Break it | "Go on. Try to **cheat**." · attacks-run / stopped counters · six attack buttons with result tags · signed message (tampered lines highlighted) · five-stage pipeline tiles · verdict banner (REJECTED red / DOWNGRADED amber) |

---

## 8. Copy voice

Short, concrete, slightly dry. Name the thing, then the proof. "Even a stolen meter key cannot beat physics." "The lie earns nothing." No exclamation marks, no "revolutionary". Numbers carry units. Testnet and demo factors are always disclosed on screen.

---

## 9. Accessibility

- Real `<button>`, `<a>`, `<input>` + `<label>`; `aria-pressed` on toggles; `aria-current` on the active role.
- Every chart and the map have `role="img"` and a sentence-long `aria-label` with the actual values.
- Focus ring: 2 px accent outline, 2 px offset.
- Touch targets ≥ 44 px (34 px only for dense segmented pills).
- GEN/GRID differ in pattern and label, not hue; red/green status always carries ✓ / ✗ / ↓ and a word.
- `prefers-reduced-motion`: all animation off (see animations.md).

---

## 10. Asset pack (`web/public/`)

| File | Spec |
|---|---|
| `favicon.svg` | Sun-coin on `#0A0C10` rounded square |
| `mark.svg` | Sun-coin, transparent |
| `icon-192.png`, `icon-512.png` | Sun-coin centred on `#0A0C10` |
| `icon-maskable-512.png` | Same with safe-zone padding |
| `apple-touch-icon.png` | 180×180 |
| `og.png` | 1200×630: ambient glow top-right, logo, "Every hour of sunlight, **minted**." (Inter Tight 900 104 px + Doto), six hour blocks (two hatched), mono tagline |
| `manifest.webmanifest` | standalone, start `/daylight`, theme/background `#0A0C10` |
| `sw.js` | App-shell cache; API always from network |
| `docs/sunmint-demo-60s.mp4` | 60 s demo video |
