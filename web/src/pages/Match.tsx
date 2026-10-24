import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, type MatchResult } from '../api'
import { Loading, SourceChip, hourUtc, kg, kwh, tonnes, usePoll, usd } from '../ui'

// A small office: base load overnight, ramps through the working day (kWh per UTC hour).
const SAMPLE = [6, 6, 6, 6, 6, 7, 10, 14, 18, 20, 21, 21, 20, 20, 19, 17, 14, 11, 9, 8, 7, 7, 6, 6]

function Clock({ m }: { m: MatchResult }) {
  const C = 180
  const seg = (h: number, r0: number, r1: number) => {
    const a0 = ((h * 15 + .8 - 90) * Math.PI) / 180, a1 = (((h + 1) * 15 - .8 - 90) * Math.PI) / 180
    const p = (r: number, a: number) => `${(C + r * Math.cos(a)).toFixed(2)} ${(C + r * Math.sin(a)).toFixed(2)}`
    return `M${p(r1, a0)} A${r1} ${r1} 0 0 1 ${p(r1, a1)} L${p(r0, a1)} A${r0} ${r0} 0 0 0 ${p(r0, a0)} Z`
  }
  return (
    <svg viewBox="0 0 360 360" role="img" aria-label={`24-hour match clock: ${m.match_pct}% of today's load matched hour by hour; red outer segments are unmatched hours`} style={{ width: '100%', height: 'auto', display: 'block' }}>
      <defs><pattern id="mh" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="6" height="6" fill="var(--accent-deep)" /><rect width="2" height="6" fill="var(--accent)" /></pattern></defs>
      {m.hours.map(h => {
        const f = h.demand_wh ? h.matched_wh / h.demand_wh : 0
        const outer = f >= .999 ? 'var(--line)' : f > 0 ? 'var(--amber)' : 'var(--red)'
        const inner = f <= 0 ? null : h.gen_wh > h.matched_wh / 2 ? 'url(#mh)' : 'var(--accent)'
        return (
          <g key={h.hour}>
            <path d={seg(h.hour, 146, 168)} fill={outer} opacity={f >= .999 ? 1 : .85} />
            {inner && <path d={seg(h.hour, 112, 112 + 28 * Math.max(.25, f))} fill={inner} stroke={inner.startsWith('url') ? 'var(--accent)' : 'none'} opacity={f >= .999 ? 1 : .55} />}
          </g>
        )
      })}
      <g fontFamily="IBM Plex Mono" fontSize="11" fill="var(--muted)" textAnchor="middle"><text x="180" y="12">00</text><text x="352" y="184">06</text><text x="180" y="356">12</text><text x="8" y="184">18</text></g>
    </svg>
  )
}

export default function Match() {
  const nav = useNavigate()
  const [load, setLoad] = useState(SAMPLE)
  const [m, setM] = useState<MatchResult | null>(null)
  const [unit, setUnit] = useState<'ton' | 'kwh'>('ton')
  const [who, setWho] = useState('ACME Ltd')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const { data: market, error } = usePoll(api.market, 20_000)

  useEffect(() => { api.match(load).then(setM).catch(e => setErr(e.message)) }, [load, market?.certs.length])

  const rows = useMemo(() => {
    const val = (c: { price_per_kwh: number; factor: number }) => (unit === 'ton' ? c.price_per_kwh / c.factor * 1e6 : c.price_per_kwh)
    return [...(market?.certs ?? [])].sort((a, b) => val(a) - val(b)).slice(0, 14).map(c => ({ c, price: val(c) }))
  }, [market, unit])

  const onCsv = async (f: File) => {
    const nums = (await f.text()).split(/[\s,;]+/).map(Number).filter(n => Number.isFinite(n))
    if (nums.length >= 24) setLoad(nums.slice(-24))
    else setErr('CSV needs 24 hourly kWh values (00:00–23:00 UTC).')
  }

  const retire = async () => {
    if (!m?.picks.length) return
    setBusy(true); setErr(null)
    try { nav(`/proof/${(await api.retire(m.picks, who)).receipt}`) } catch (e) { setErr((e as Error).message) } finally { setBusy(false) }
  }

  if (!market) return <Loading error={error} />
  const gaps = m ? m.hours.filter(h => h.matched_wh < h.demand_wh).length : 0
  return (
    <div className="page">
      <div className="row" style={{ alignItems: 'flex-start' }}>
        <section className="panel feature" style={{ flex: '1 1 420px', minWidth: 0 }} aria-labelledby="match">
          <p className="label" style={{ color: 'var(--accent)' }}>Match my load · last 24 h · {m ? kwh(m.demand_wh, 0) : '—'} kWh used</p>
          <h1 id="match" className="h3" style={{ fontSize: 34 }}>
            {m && m.match_pct >= 50 ? <>Over half your hours, <span className="dot">proven</span> clean.</> : <>Your hours, <span className="dot">proven</span> clean.</>}
          </h1>
          <div style={{ position: 'relative', maxWidth: 360, margin: '20px auto 0' }}>
            {m && <Clock m={m} />}
            <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', pointerEvents: 'none' }}>
              <p className="num glowtxt" style={{ margin: 0, fontSize: 64, lineHeight: 1, color: 'var(--accent)' }}>{m ? `${m.match_pct}%` : '…'}</p>
              <p className="label" style={{ marginTop: 6, fontSize: 10 }}>24/7 match</p>
            </div>
          </div>
          <div className="mono small muted" style={{ display: 'flex', gap: 14, flexWrap: 'wrap', justifyContent: 'center', marginTop: 12 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw" />Matched</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw gen" />By GEN hours</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw" style={{ background: 'var(--amber)', borderColor: 'var(--amber)' }} />Partly</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw gap" />Gap</span>
          </div>
          <div className="grid" style={{ ['--min' as string]: '140px', marginTop: 18, gap: 10 }}>
            <div className="item"><p className="num" style={{ margin: 0, fontSize: 26 }}>{m ? `${tonnes(m.g_co2)} t` : '—'}</p><p className="label" style={{ marginTop: 6, fontSize: 10 }}>CO₂ avoided if retired</p></div>
            <div className="item"><p className="num" style={{ margin: 0, fontSize: 26, color: 'var(--red-fg)' }}>{gaps} h</p><p className="label" style={{ marginTop: 6, fontSize: 10 }}>Gaps · night needs storage</p></div>
          </div>
          <label className="stack" style={{ marginTop: 16 }}>
            <span className="label">Retire on behalf of</span>
            <input value={who} onChange={e => setWho(e.target.value)} maxLength={64} />
          </label>
          <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 14 }}>
            <button type="button" className="btn primary lg" style={{ flex: '1 1 220px' }} disabled={busy || !m?.picks.length} onClick={retire}>
              {busy ? 'Retiring on-chain…' : `Retire ${m?.certificates ?? 0} certificates · ${m ? usd(m.cost) : '—'} tUSDC`}
            </button>
            <label className="btn lg" style={{ flex: '1 1 160px' }}>Upload load (CSV)<input type="file" accept=".csv,text/csv,text/plain" className="sr" onChange={e => e.target.files?.[0] && onCsv(e.target.files[0])} /></label>
          </div>
          {err && <p className="mono small" style={{ color: 'var(--red-fg)', margin: '10px 0 0' }}>{err}</p>}
          <p className="muted small" style={{ margin: '14px 0 0' }}>Solar can't cover nights. Wind and battery-discharge certificates are on the roadmap; until then gaps stay red rather than being papered over with annual credits.</p>
        </section>

        <section className="panel grow" aria-labelledby="mkt">
          <div className="between">
            <div><h2 id="mkt" className="h4" style={{ fontSize: 28, margin: 0 }}>Certificates for sale</h2><p className="label" style={{ marginTop: 6 }}>Sorted by price per {unit === 'ton' ? 'tonne CO₂ avoided' : 'kWh'}</p></div>
            <div className="seg" role="group" aria-label="Price unit">
              <button type="button" aria-pressed={unit === 'kwh'} onClick={() => setUnit('kwh')}>PER KWH</button>
              <button type="button" aria-pressed={unit === 'ton'} onClick={() => setUnit('ton')}>PER TONNE CO₂</button>
            </div>
          </div>
          {unit === 'ton' && <div className="item good" style={{ marginTop: 14, fontSize: 14 }}>Per tonne, <strong style={{ color: 'var(--accent)' }}>generator-displacing hours are the cheapest climate impact here</strong>. The same kWh avoids more than twice the CO₂.</div>}
          <div className="scroll" style={{ marginTop: 12 }}>
            <table>
              <thead><tr><th>SITE</th><th>HOUR (UTC)</th><th>SOURCE</th><th>KWH LEFT</th><th>CO₂</th><th style={{ textAlign: 'right' }}>tUSDC / {unit === 'ton' ? 'TONNE' : 'KWH'}</th></tr></thead>
              <tbody>
                {rows.map(({ c, price }, i) => (
                  <tr key={c.id}>
                    <td style={{ fontFamily: 'Geist', fontWeight: 500 }}>{c.site}</td>
                    <td style={{ color: 'var(--fg-2)' }}>{hourUtc(c.hour)}</td>
                    <td><SourceChip src={c.source} /></td>
                    <td><span className="num" style={{ color: 'var(--accent)' }}>{kwh(c.available_wh, 2)}</span></td>
                    <td><span className="num">{kg(c.g_co2 * c.available_wh / c.wh)} kg</span></td>
                    <td style={{ textAlign: 'right' }}><span className="num" style={{ fontSize: 18, color: i === 0 ? 'var(--accent)' : undefined }}>{unit === 'ton' ? usd(price, 0) : usd(price, 3)}</span></td>
                  </tr>
                ))}
                {!rows.length && <tr><td colSpan={6} className="muted">Nothing listed yet.</td></tr>}
              </tbody>
            </table>
          </div>
          <p className="mono muted" style={{ fontSize: 11, margin: '12px 0 0' }}>Testnet prices in tUSDC. Retiring buys and burns in one transaction through SunmintMarket.buyAndRetire.</p>
        </section>
      </div>
    </div>
  )
}
