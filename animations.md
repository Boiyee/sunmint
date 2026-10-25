# SUNMINT · animations.md

Motion is functional: it says **live**, **pending** or **verdict**. Nothing loops just to decorate except the ambient glow, which is slow enough to read as light. All keyframes live in `web/src/styles.css`.

## Global rule

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; }
}
```
Every state is still readable with motion off: pending blocks stay outlined amber, verdicts still show their colour and word.

---

## 1. Ambient breathe

Where: landing background glows (`.ambient`), Proof receipt glow.

```css
@keyframes breathe { 50% { opacity: .04; transform: scale(1.12); } }
.ambient { position: absolute; border-radius: 50%; background: var(--accent);
           filter: blur(130px); opacity: .1; pointer-events: none;
           animation: breathe 14s ease-in-out infinite; }
```
- Landing: 560 px circle at `left:-280px; top:-200px` (opacity .1) and 480 px at `right:-240px; top:900px` (opacity .06).
- Proof: 420 px at `right:-200px; top:-180px`, opacity .14.
- 14 s is deliberate: slow enough that it never pulls attention.

## 2. Ray flow

Where: hero sun rays and the meter-to-blocks trace.

```css
@keyframes flow { to { stroke-dashoffset: -24; } }
.ray { animation: flow 1.4s linear infinite; }
```
Applied to SVG lines/paths with `stroke-dasharray="6 6"`. The −24 offset is two full dash periods, so the loop is seamless. Direction reads as energy travelling from the sun to the panels and down into the chain.

## 3. Minting pulse

Where: the "MINTING" hour block in the hero, the in-progress hour marker on the Rooftop curve.

```css
@keyframes mint { 0%, 100% { opacity: .5; } 50% { opacity: 1; } }
.minting { animation: mint 2.4s ease-in-out infinite; }
```
Only one element on screen pulses at a time. Stops (class removed) once the hour is minted.

## 4. Site ping

Where: lit sites on the Daylight map.

```css
@keyframes ping {
  0%   { transform: translate(-50%, -50%) scale(.6); opacity: .7; }
  100% { transform: translate(-50%, -50%) scale(2.6); opacity: 0; }
}
.pin .pulse { animation: ping 2.6s ease-out infinite; opacity: .5; }
```
Ring diameter = `18 + 44·√(liveW / maxLiveW)` px, so bigger output means a bigger ping. Night sites have no ring. The translate stays inside the keyframes so the ring scales from the pin's centre.

## 5. Verdict stamp

Where: the Break it verdict banner (REJECTED / DOWNGRADED / ACCEPTED).

```css
@keyframes stamp { from { transform: scale(1.08); opacity: 0; } to { transform: none; opacity: 1; } }
.stamp { animation: stamp .35s cubic-bezier(.2, .8, .2, 1) both; }
```
The banner gets a React `key` of `attack + reason`, so it remounts and re-stamps on every new run. Glow: `box-shadow: 0 0 70px -18px <verdict colour>`.

## 6. Feed rise

Where: new certificates in the Daylight mint feed.

```css
@keyframes rise { from { transform: translateY(8px); opacity: 0; } to { transform: none; opacity: 1; } }
.rise { animation: rise .4s ease both; }
```
Items are keyed by certificate id, so only newly arrived ones animate when the feed refreshes every 15 s.

---

## Live data rhythm

Not CSS, but part of the feel:

| Screen | Refresh |
|---|---|
| Daylight | 15 s |
| Rooftop | 15 s |
| Match market | 20 s; re-matches when the market size changes |
| Landing totals | 30 s |
| Simulator meters | a signed reading per site every 5 min, a claim per closed hour |

## Navigation

Route changes use React Router's `v7_startTransition`: the current screen stays up until the next one is ready, so there is no loader flash. Page chunks are prefetched 1.2 s after first load. On route change the window scrolls to the top instantly (no smooth scroll).

## Interaction states

| Element | Change |
|---|---|
| Links | colour → `--accent`, no transition |
| Primary button | `filter: brightness(1.06)` on hover |
| Focus | 2 px accent outline, 2 px offset |
| Busy button | opacity .5, label changes ("Retiring on-chain…", "RUNNING…") |

---

## Demo video overlays (`docs/sunmint-demo-60s.mp4`)

Rebuildable with the recorder script; values for reference:

| Element | Motion |
|---|---|
| Title / problem / end cards | Full-screen `#0A0C10`, opacity 0→1 over .45 s; cursor hidden while shown |
| Step caption | Bottom-left pill, accent border; enter `opacity 0→1, translateY(14px)→0` over .35 s; swap = .25 s out, then in |
| Cursor | 22 px arrow; moves on an ease-in-out quad curve over 650–800 ms; on click scales to .8 for 160 ms and fires a 44 px accent ring (`scale .4→1.6`, opacity .9→0, .5 s) |
| Scroll | Ease-out cubic over 1.2–1.6 s |
| Edit | Recorded at 1600×900 @1.2×, sped to exactly 60 s, 30 fps, 0.6 s fade in, 0.8 s fade out |
