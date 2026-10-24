import { Link } from 'react-router-dom'
import { api } from '../api'
import { MASK } from '../worldmask'
import { Loading, SourceChip, Tx, ago, attackLabel, hourUtc, kg, kwh, useConfig, usePoll, utcClock } from '../ui'

const DOTS = MASK.flatMap((row, r) => [...row].flatMap((ch, c) => (ch === '#' ? [{ l: ((c + .5) / 96) * 100, t: ((r + .5) / 40) * 100 }] : [])))
const pos = (lat: number, lon: number) => ({ left: `${((lon + 180) / 360) * 100}%`, top: `${((78 - lat) / 134) * 100}%` })

export default function Daylight() {
  const { data, error } = usePoll(api.network, 15_000)
  const cfg = useConfig()
  if (!data) return <Loading error={error} />
  const lit = data.sites.filter(s => (s.live_w ?? 0) > 0)
  const max = Math.max(1, ...data.sites.map(s => s.live_w ?? 0))
  return (
    <div className="page">
      <div className="row">
        <section className="panel grow" aria-labelledby="net">
          <div className="between" style={{ alignItems: 'flex-end' }}>
            <div>
              <p className="label">Daylight · {data.sites.length} sites · UTC {utcClock(data.now)}</p>
              <h1 id="net" className="h3">The sun is up over <span className="dot">{lit.length}</span> {lit.length === 1 ? 'site' : 'sites'}.</h1>
            </div>
            <div style={{ display: 'flex', gap: 28 }}>
              <div><p className="num" style={{ margin: 0, fontSize: 34, color: 'var(--accent)' }}>{kwh(data.totals.live_w)}</p><p className="label" style={{ fontSize: 10 }}>kW now</p></div>
              <div><p className="num" style={{ margin: 0, fontSize: 34 }}>{kwh(data.totals.wh, 0)}</p><p className="label" style={{ fontSize: 10 }}>kWh minted</p></div>
              <div><p className="num" style={{ margin: 0, fontSize: 34 }}>{kg(data.totals.g_co2, 0)}</p><p className="label" style={{ fontSize: 10 }}>kg CO₂</p></div>
            </div>
          </div>

          <div className="map" role="img" aria-label={`World map with ${data.sites.length} solar sites: ${data.sites.map(s => `${s.name} ${s.live_w ? `${kwh(s.live_w)} kW` : 'night'}`).join(', ')}`}>
            {DOTS.map((d, i) => <i key={i} className="d" style={{ left: `${d.l}%`, top: `${d.t}%` }} />)}
            {data.sites.map(s => {
              const on = (s.live_w ?? 0) > 0
              const c = on ? 'var(--accent)' : 'var(--line-2)'
              const r = on ? 18 + 44 * Math.sqrt((s.live_w ?? 0) / max) : 14
              return (
                <div key={s.slug} className="pin" style={pos(s.lat, s.lon)}>
                  {on && <i className="pulse" style={{ width: r, height: r, background: c }} />}
                  <i style={{ width: 12, height: 12, background: c, boxShadow: on ? `0 0 18px ${c}` : 'none' }} />
                  <Link to={`/rooftop/${s.slug}`} className="tag">
                    {s.name} <span style={{ color: on ? 'var(--accent)' : 'var(--muted)' }}>{on ? `${kwh(s.live_w!)} kW` : 'night'}</span>
                    {s.grid_down && <span style={{ color: 'var(--accent)' }}> · GEN OFF</span>}
                  </Link>
                </div>
              )
            })}
          </div>

          <div style={{ display: 'flex', gap: 20, flexWrap: 'wrap', marginTop: 18 }} className="mono small muted">
            <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}><i className="sw" style={{ borderRadius: '50%' }} />Generating · size = live kW</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}><i className="sw gen" />Grid down · generator displaced</span>
            <span>Simulated meters on real sun positions</span>
          </div>
        </section>

        <aside className="panel side" aria-labelledby="feed">
          <div className="between"><h2 id="feed" className="label">Live mint feed</h2><span className="mono small" style={{ color: 'var(--accent)' }}>● LIVE</span></div>
          <ol className="list" style={{ marginTop: 14 }}>
            {data.feed.slice(0, 6).map(c => (
              <li key={c.id} className="item rise">
                <Link to={`/trace/${c.id}`} style={{ textDecoration: 'none', display: 'block' }}>
                  <div className="between mono small muted"><span>{c.short} · {hourUtc(c.hour)} UTC</span><span>{ago(c.minted_at, data.now)}</span></div>
                  <div className="between" style={{ marginTop: 6, alignItems: 'baseline' }}><span style={{ fontWeight: 500 }}>{c.site}</span><span className="num" style={{ fontSize: 22, color: c.source === 'GEN' ? 'var(--accent)' : undefined }}>{kwh(c.wh, 2)} kWh</span></div>
                </Link>
                <div className="mono small" style={{ marginTop: 4, color: 'var(--fg-2)', display: 'flex', gap: 8, flexWrap: 'wrap' }}><SourceChip src={c.source} /> · {kg(c.g_co2)} kg CO₂ · tx <Tx hash={c.tx} cfg={cfg} /></div>
              </li>
            ))}
            {!data.feed.length && <li className="item muted small">No certificates yet. Start the simulator.</li>}
          </ol>
          {data.rejections.length > 0 && (
            <>
              <h2 className="label" style={{ marginTop: 20 }}>Recently rejected</h2>
              <ol className="list" style={{ marginTop: 10 }}>
                {data.rejections.slice(0, 2).map((r, i) => (
                  <li key={i} className="item bad mono small"><span style={{ color: 'var(--red-fg)' }}>✗ {attackLabel(r.kind)}</span> <span className="muted">{ago(r.at, data.now)}</span><div style={{ color: 'var(--fg-2)', marginTop: 4, whiteSpace: 'normal' }}>{r.reason}</div></li>
                ))}
              </ol>
            </>
          )}
        </aside>
      </div>
    </div>
  )
}
