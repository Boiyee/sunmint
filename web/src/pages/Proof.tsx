import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api, type Cert, type Retirement } from '../api'
import { Loading, Mark, Tx, kwh, short, tonnes, useConfig, usd } from '../ui'

/** Draws the receipt card to a canvas so it can be saved as a PNG. */
function savePng(r: Retirement, certs: Cert[]) {
  const W = 1200, H = 630, c = document.createElement('canvas')
  c.width = W; c.height = H
  const g = c.getContext('2d')!
  const css = getComputedStyle(document.documentElement)
  const v = (n: string) => css.getPropertyValue(n).trim()
  g.fillStyle = '#0A0C10'; g.fillRect(0, 0, W, H)
  const grd = g.createRadialGradient(W, 0, 0, W, 0, 520); grd.addColorStop(0, 'rgba(75,227,139,.22)'); grd.addColorStop(1, 'rgba(75,227,139,0)')
  g.fillStyle = grd; g.fillRect(0, 0, W, H)
  g.strokeStyle = '#4BE38B'; g.lineWidth = 2; g.beginPath(); g.roundRect(30, 30, W - 60, H - 60, 28); g.stroke()
  g.fillStyle = '#8A93A3'; g.font = '500 20px "IBM Plex Mono"'
  g.fillText(`SUNMINT · RECEIPT #${r.receipt} · ${r.beneficiary.toUpperCase()}`, 80, 100)
  g.fillStyle = '#4BE38B'; g.font = '900 150px Doto'; g.fillText(tonnes(r.g_co2), 72, 270)
  g.fillStyle = '#E9ECF1'; g.font = '900 44px "Inter Tight"'; g.fillText('tonnes of CO₂ avoided', 80, 335)
  const wh = r.whs.reduce((a, b) => a + b, 0)
  g.fillStyle = '#B7BFCC'; g.font = '500 26px "IBM Plex Mono"'
  g.fillText(`${kwh(wh, 1)} kWh · ${r.cert_ids.length} certificates · ${new Set(certs.map(x => x.slug)).size} sites`, 80, 400)
  g.fillStyle = '#8A93A3'; g.font = '400 20px "IBM Plex Mono"'
  g.fillText(`tx ${short(r.tx, 10, 8)} · burned on-chain, never claimable again`, 80, 540)
  void v
  const a = document.createElement('a'); a.download = `sunmint-receipt-${r.receipt}.png`; a.href = c.toDataURL('image/png'); a.click()
}

export default function Proof() {
  const { n = '1' } = useParams()
  const cfg = useConfig()
  const [d, setD] = useState<{ receipt: Retirement; certs: Cert[] } | null>(null)
  const [err, setErr] = useState<string | null>(null)
  useEffect(() => { api.receipt(Number(n)).then(setD).catch(e => setErr(e.message)) }, [n])
  if (!d) return <Loading error={err} />
  const { receipt: r, certs } = d
  const wh = r.whs.reduce((a, b) => a + b, 0)
  const hours = new Map<number, 'gen' | 'grid'>()
  certs.forEach(c => { const h = c.hour % 24; if (c.source === 'GEN' || !hours.has(h)) hours.set(h, c.source === 'GEN' ? 'gen' : 'grid') })
  const countries = new Set(certs.map(c => c.country)).size
  const url = `${location.origin}/proof/${r.receipt}`
  return (
    <div className="page" style={{ maxWidth: 760 }}>
      <article className="panel feature" style={{ padding: 32, borderRadius: 22, position: 'relative', overflow: 'hidden' }}>
        <div className="ambient" style={{ width: 420, height: 420, right: -200, top: -180, opacity: .14 }} />
        <div className="between" style={{ position: 'relative' }}>
          <span className="logo"><Mark />SUNMINT</span>
          <span className="chip" style={{ borderColor: 'var(--accent)', color: 'var(--accent)' }}>RETIRED · BURNED</span>
        </div>
        <p className="label" style={{ marginTop: 28 }}>{r.beneficiary} · {new Date(r.at * 1000).toUTCString().slice(5, 16)} · Receipt #{r.receipt}</p>
        <p className="num glowtxt" style={{ margin: '14px 0 0', fontSize: 'clamp(72px, 14vw, 108px)', lineHeight: .9, color: 'var(--accent)' }}>{tonnes(r.g_co2)}</p>
        <p className="h4" style={{ fontSize: 30, margin: '8px 0 0', fontWeight: 900 }}>tonnes of CO₂ avoided</p>
        <div className="grid" style={{ ['--min' as string]: '140px', gap: 10, marginTop: 24 }}>
          <div className="item"><p className="num" style={{ margin: 0, fontSize: 26 }}>{kwh(wh, 1)}</p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>kWh verified</p></div>
          <div className="item"><p className="num" style={{ margin: 0, fontSize: 26 }}>{r.cert_ids.length}</p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>Certificates</p></div>
          <div className="item"><p className="num" style={{ margin: 0, fontSize: 26 }}>{new Set(certs.map(c => c.slug)).size}</p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>Sites · {countries} countries</p></div>
        </div>
        <p className="label" style={{ margin: '24px 0 10px' }}>Hours covered (UTC) · hatched = generator displaced</p>
        <div role="img" aria-label={`${hours.size} of 24 hours covered`} style={{ display: 'grid', gridTemplateColumns: 'repeat(24, minmax(0, 1fr))', gap: 3 }}>
          {Array.from({ length: 24 }, (_, h) => {
            const s = hours.get(h)
            return <i key={h} style={{ height: 34, borderRadius: 4, border: `1px solid ${s ? 'var(--accent)' : 'var(--line-2)'}`, background: s === 'gen' ? 'var(--hatch)' : s ? 'var(--accent)' : 'transparent' }} />
          })}
        </div>
        <div className="between mono muted" style={{ fontSize: 10, marginTop: 6 }}><span>00</span><span>06</span><span>12</span><span>18</span><span>24</span></div>
        <div style={{ marginTop: 24, paddingTop: 20, borderTop: '1px solid var(--line)' }}>
          <p className="h4" style={{ fontSize: 18, margin: 0 }}>Every certificate traces to its meter.</p>
          <p className="mono small muted" style={{ margin: '8px 0 0', lineHeight: 1.6 }}>tx <Tx hash={r.tx} cfg={cfg} /> · paid {usd(r.cost)} tUSDC · burned on-chain, these hours can never be claimed again.</p>
        </div>
      </article>
      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
        <button type="button" className="btn primary" onClick={() => savePng(r, certs)}>Download PNG</button>
        <button type="button" className="btn" onClick={() => navigator.clipboard?.writeText(url)}>Copy link</button>
        {certs[0] && <Link to={`/trace/${(certs.find(c => c.source === 'GEN') ?? certs[0]).id}`} className="btn">Trace a certificate</Link>}
      </div>
      <section className="panel">
        <h2 className="label">Certificates in this receipt</h2>
        <div className="scroll" style={{ marginTop: 10, maxHeight: 360 }}>
          <table>
            <thead><tr><th>CERT</th><th>SITE</th><th>WH RETIRED</th><th>SOURCE</th></tr></thead>
            <tbody>{certs.map((c, i) => <tr key={c.id}><td><Link to={`/trace/${c.id}`}>{c.short}</Link></td><td>{c.site}</td><td>{r.whs[i]}</td><td>{c.source}</td></tr>)}</tbody>
          </table>
        </div>
      </section>
    </div>
  )
}
