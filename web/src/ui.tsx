import { useEffect, useState, type ReactNode } from 'react'
import { Link, NavLink, useLocation } from 'react-router-dom'
import { api, type Config, type Source } from './api'

// ── formatting ───────────────────────────────────────────────────────────
export const kwh = (wh: number, d = 1) => (wh / 1000).toFixed(d)
export const kg = (g: number, d = 2) => (g / 1000).toFixed(d)
export const tonnes = (g: number) => (g / 1e6).toFixed(g < 1e5 ? 3 : 2)
export const usd = (micro: number, d = 2) => (micro / 1e6).toFixed(d)
export const short = (s: string, a = 6, b = 4) => (s && s.length > a + b + 1 ? `${s.slice(0, a)}…${s.slice(-b)}` : s)
export const pad = (n: number) => String(n).padStart(2, '0')
export const hourUtc = (h: number) => `${pad(h % 24)}:00–${pad((h + 1) % 24)}:00`
export const ago = (ts: number, now = Date.now() / 1000) => {
  const s = Math.max(0, Math.round(now - ts))
  return s < 60 ? `${s}s ago` : s < 3600 ? `${Math.round(s / 60)}m ago` : s < 86400 ? `${Math.round(s / 3600)}h ago` : `${Math.round(s / 86400)}d ago`
}
export const utcClock = (ts: number) => new Date(ts * 1000).toISOString().slice(11, 16)
/** Local wall clock at a site. */
export const siteClock = (ts: number, utcOffset: number) => utcClock(ts + utcOffset * 3600)
export const attackLabel = (kind: string) => kind.startsWith('attack:') ? `attack · ${kind.slice(7)}` : kind

// ── data ─────────────────────────────────────────────────────────────────
export function usePoll<T>(fn: () => Promise<T>, ms: number, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let live = true
    const tick = () => fn().then(d => live && (setData(d), setError(null))).catch(e => live && setError(String(e.message ?? e)))
    tick()
    const t = setInterval(tick, ms)
    return () => { live = false; clearInterval(t) }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)
  return { data, error }
}

let cfgPromise: Promise<Config> | null = null
export function useConfig() {
  const [cfg, setCfg] = useState<Config | null>(null)
  useEffect(() => { (cfgPromise ??= api.config()).then(setCfg).catch(() => { cfgPromise = null }) }, [])
  return cfg
}
export const txUrl = (cfg: Config | null, tx: string) => (cfg?.live ? `${cfg.explorer}/tx/${tx}` : undefined)
export const addrUrl = (cfg: Config | null, a: string) => (cfg?.live ? `${cfg.explorer}/address/${a}` : undefined)

export function Tx({ hash, cfg }: { hash: string; cfg: Config | null }) {
  const href = txUrl(cfg, hash)
  return href ? <a href={href} target="_blank" rel="noreferrer">{short(hash)}</a> : <span>{short(hash)}</span>
}

// ── theme ────────────────────────────────────────────────────────────────
const getTheme = () => { try { return localStorage.getItem('sunmint-theme') ?? 'dark' } catch { return 'dark' } }
export function applyTheme(t = getTheme()) { document.documentElement.dataset.theme = t }
export function ThemeToggle() {
  const [t, setT] = useState(getTheme)
  const flip = () => {
    const n = t === 'dark' ? 'light' : 'dark'
    setT(n); applyTheme(n)
    try { localStorage.setItem('sunmint-theme', n) } catch { /* private mode */ }
  }
  return (
    <button type="button" className="iconbtn" onClick={flip} aria-label={`Switch to ${t === 'dark' ? 'light' : 'dark'} mode`}>
      {t === 'dark'
        ? <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M21 13.5A8.5 8.5 0 1 1 10.5 3a7 7 0 0 0 10.5 10.5z" /></svg>
        : <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><circle cx="12" cy="12" r="4.5" /><path d="M12 2v2.5M12 19.5V22M2 12h2.5M19.5 12H22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" /></svg>}
    </button>
  )
}

// ── brand ────────────────────────────────────────────────────────────────
export function Mark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 40 40" aria-hidden="true">
      <circle cx="20" cy="20" r="15" fill="none" stroke="var(--accent)" strokeWidth="5" strokeDasharray="2.2 1.73" />
      <circle cx="20" cy="20" r="9" fill="var(--accent)" />
    </svg>
  )
}
export const Logo = () => <Link to="/" className="logo"><Mark />SUNMINT</Link>

export const SourceChip = ({ src, label = true }: { src: Source; label?: boolean }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
    <i className={`sw ${src === 'GEN' ? 'gen' : ''}`} />{label && src}
  </span>
)

export const Stat = ({ value, label, accent }: { value: ReactNode; label: string; accent?: boolean }) => (
  <div className="panel stat">
    <p className="num" style={{ color: accent ? 'var(--accent)' : undefined }}>{value}</p>
    <p className="label">{label}</p>
  </div>
)

// ── app chrome ───────────────────────────────────────────────────────────
const PRODUCER = [['/daylight', 'Daylight'], ['/rooftop', 'Rooftop'], ['/trace', 'Trace']] as const
const COMPANY = [['/daylight', 'Daylight'], ['/match', 'Match'], ['/trace', 'Trace']] as const

export function AppShell({ children }: { children: ReactNode }) {
  const loc = useLocation()
  const cfg = useConfig()
  const company = loc.pathname.startsWith('/match') || loc.pathname.startsWith('/proof')
  const [role, setRole] = useState<'producer' | 'company'>(company ? 'company' : 'producer')
  useEffect(() => { if (company) setRole('company'); else if (loc.pathname.startsWith('/rooftop')) setRole('producer') }, [company, loc.pathname])
  const tabs = role === 'company' ? COMPANY : PRODUCER
  return (
    <>
      <header className="apphead">
        <div style={{ display: 'flex', alignItems: 'center', gap: 18, flexWrap: 'wrap' }}>
          <Logo />
          <div className="seg" role="group" aria-label="Role">
            <Link to="/rooftop" className={role === 'producer' ? 'active' : ''} aria-current={role === 'producer' ? 'true' : undefined}>PRODUCER</Link>
            <Link to="/match" className={role === 'company' ? 'active' : ''} aria-current={role === 'company' ? 'true' : undefined}>COMPANY</Link>
          </div>
          <nav aria-label="App">
            {tabs.map(([to, label]) => <NavLink key={to} to={to} className={({ isActive }) => `tab${isActive ? ' active' : ''}`}>{label}</NavLink>)}
            <NavLink to="/break-it" className={({ isActive }) => `tab hot${isActive ? ' active' : ''}`}>Break it</NavLink>
          </nav>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span className="chip"><span className="livedot" style={{ background: cfg?.live ? 'var(--accent)' : 'var(--amber)' }} />{cfg ? (cfg.live ? 'BASE SEPOLIA' : 'DRY RUN') : '…'}</span>
          <ThemeToggle />
        </div>
      </header>
      <main>{children}</main>
    </>
  )
}

export function Loading({ error }: { error?: string | null }) {
  return (
    <div className="page">
      <div className="panel" style={{ textAlign: 'center', padding: 48 }}>
        <p className="label">{error ? 'Verifier unreachable' : 'Loading'}</p>
        {error && <p className="muted small mono" style={{ marginTop: 10 }}>{error}</p>}
      </div>
    </div>
  )
}
