import { Link, useNavigate, useParams } from 'react-router-dom'
import { api, type HourRow } from '../api'
import { Loading, SourceChip, Stat, Tx, ago, attackLabel, hourUtc, kg, kwh, pad, siteClock, useConfig, usePoll, usd } from '../ui'

const SLUGS = [['lagos', 'Lagos'], ['nairobi', 'Nairobi'], ['jaipur', 'Jaipur'], ['manila', 'Manila'], ['seville', 'Seville'], ['saopaulo', 'São Paulo']]

function Curve({ hours, capW, off, now }: { hours: HourRow[]; capW: number; off: number; now: number }) {
  const W = 1000, H = 330, x0 = 44, x1 = 970, top = 44, base = 290
  const n = hours.length
  const bw = (x1 - x0) / n
  const peak = Math.max(capW * 0.9, ...hours.map(h => h.wh))
  const y = (wh: number) => base - (wh / peak) * (base - top)
  const xm = (i: number) => x0 + i * bw + bw / 2
  const done = hours.map((h, i) => ({ h, i })).filter(({ h }) => h.readings > 0)
  const line = done.map(({ h, i }, k) => `${k ? 'L' : 'M'}${xm(i).toFixed(1)} ${y(h.wh).toFixed(1)}`).join(' ')
  const area = done.length ? `${line} L${xm(done[done.length - 1].i).toFixed(1)} ${base} L${xm(done[0].i).toFixed(1)} ${base} Z` : ''
  const nowX = x0 + ((now / 3600 - hours[0].hour) / n) * (x1 - x0)
  const ticks = [0, 1, 2, 3].map(k => peak * k / 3)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Generation over the last 24 hours, hourly certificates marked; generator-displacing hours hatched" style={{ width: '100%', height: 'auto', display: 'block', marginTop: 16 }}>
      <defs>
        <pattern id="sh" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="7" height="7" fill="var(--accent-deep)" /><rect width="2" height="7" fill="var(--accent)" opacity=".45" /></pattern>
        <linearGradient id="ar" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor="var(--accent)" stopOpacity=".28" /><stop offset="1" stopColor="var(--accent)" stopOpacity="0" /></linearGradient>
      </defs>
      {ticks.map(t => <g key={t}><line x1={x0} x2={x1} y1={y(t)} y2={y(t)} stroke="var(--line)" /><text x="4" y={y(t) + 4} fontFamily="IBM Plex Mono" fontSize="11" fill="var(--muted)">{kwh(t)}</text></g>)}
      {hours.map((h, i) => h.grid_down && h.wh > 0 ? <rect key={h.hour} x={x0 + i * bw + 1} y={top} width={bw - 2} height={base - top} fill="url(#sh)" stroke="var(--accent-line)" /> : null)}
      {hours.map((h, i) => h.grid_down && h.wh > 0 ? <text key={'g' + h.hour} x={xm(i)} y={top + 14} textAnchor="middle" fontFamily="IBM Plex Mono" fontSize="9" fill="var(--accent)">GEN</text> : null)}
      {area && <path d={area} fill="url(#ar)" />}
      {line && <path d={line} fill="none" stroke="var(--accent)" strokeWidth="3" strokeLinejoin="round" strokeLinecap="round" />}
      <line x1={nowX} x2={nowX} y1={top - 10} y2={base} stroke="var(--fg)" strokeDasharray="2 4" />
      {hours.map((h, i) => {
        const minting = !h.cert && h.wh > 0 && i === n - 1
        return h.cert ? <circle key={'c' + h.hour} cx={xm(i)} cy={26} r="5" fill="var(--accent)" /> : minting ? <circle key="m" className="minting" cx={xm(i)} cy={26} r="6" fill="none" stroke="var(--amber)" strokeWidth="2" /> : null
      })}
      {hours.map((h, i) => (i % 3 === 0 ? <text key={'t' + h.hour} x={xm(i)} y={H - 10} textAnchor="middle" fontFamily="IBM Plex Mono" fontSize="11" fill="var(--muted)">{pad((h.hour + off + 24) % 24)}</text> : null))}
    </svg>
  )
}

export default function Rooftop() {
  const { slug = 'lagos' } = useParams()
  const nav = useNavigate()
  const { data, error } = usePoll(() => api.site(slug), 15_000, [slug])
  const cfg = useConfig()
  if (!data || data.site.slug !== slug) return <Loading error={error} />
  const { site, summary: s, hours, stream, rejections, certs } = data
  const genShare = s.g_co2 ? Math.round((s.g_co2_gen / s.g_co2) * 100) : 0
  return (
    <div className="page">
      <div className="between" style={{ alignItems: 'flex-end' }}>
        <div style={{ minWidth: 0 }}>
          <p className="label">{site.name} · {kwh(site.capacity_w)} kWp rooftop · {site.has_generator ? 'generator registered' : 'no generator'} · {siteClock(data.now, site.utc_offset)} local</p>
          <h1 className="h3">
            {site.has_generator && s.gen_hours > 0
              ? <>This roof kept the generator off for <span className="dot">{s.gen_hours}</span> {s.gen_hours === 1 ? 'hour' : 'hours'}.</>
              : <>This roof minted <span className="dot">{s.certs}</span> verified hours.</>}
          </h1>
        </div>
        <label className="seg" style={{ padding: 0, border: 0 }}>
          <span className="sr">Site</span>
          <select value={slug} onChange={e => nav(`/rooftop/${e.target.value}`)} className="btn" style={{ background: 'var(--panel)', paddingRight: 14 }}>
            {SLUGS.map(([v, n]) => <option key={v} value={v}>{n}</option>)}
          </select>
        </label>
      </div>

      <div className="grid">
        <Stat value={kwh(s.wh)} label="kWh verified · 24 h" accent />
        <Stat value={kg(s.g_co2, 1)} label={`kg CO₂ avoided · ${genShare}% from gen hours`} />
        <Stat value={s.certs} label="Certificates minted" />
        <Stat value={usd(s.earned)} label="tUSDC earned" />
      </div>

      <div className="row">
        <section className="panel grow" aria-labelledby="curve">
          <div className="between">
            <h2 id="curve" className="label">Last 24 hours · one certificate per hour</h2>
            <div className="mono small muted" style={{ display: 'flex', gap: 14, flexWrap: 'wrap' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw part" />Grid displaced</span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><i className="sw gen" />Grid down · generator displaced</span>
            </div>
          </div>
          <Curve hours={hours} capW={site.capacity_w} off={site.utc_offset} now={data.now} />
          <p className="mono muted" style={{ margin: '8px 0 0', fontSize: 12 }}>● minted · ◌ this hour, minting when it closes · local time on the axis · demo factors: {site.country} grid {site.grid_factor} g/kWh, diesel {cfg?.genFactor ?? 860} g/kWh</p>
        </section>

        <aside className="panel side" aria-labelledby="stream">
          <div className="between"><h2 id="stream" className="label">Signed readings · every 5 min</h2><span className="mono small" style={{ color: 'var(--accent)' }}>● LIVE</span></div>
          <ol className="list" style={{ marginTop: 12, gap: 6 }}>
            {rejections.slice(0, 2).map((r, i) => (
              <li key={'x' + i} className="item bad mono small"><span style={{ color: 'var(--red-fg)' }}>✗ {attackLabel(r.kind)}</span> <span className="muted">{ago(r.at, data.now)}</span><div style={{ marginTop: 4, color: 'var(--fg-2)', whiteSpace: 'normal' }}>{r.reason}</div></li>
            ))}
            {stream.slice(0, 9).map(r => (
              <li key={r.nonce} className="item mono small" style={{ display: 'grid', gridTemplateColumns: '48px 1fr auto', gap: 10, padding: '9px 12px' }}>
                <span className="muted">{siteClock(r.ts, site.utc_offset)}</span>
                <span>{r.wh} Wh <span className="muted">· grid {r.grid_v} V · n{r.nonce}</span></span>
                <span style={{ color: 'var(--accent)' }}>✓</span>
              </li>
            ))}
          </ol>
          <Link to="/break-it" className="mono small muted" style={{ display: 'block', marginTop: 12 }}>Try sending a bad one →</Link>
        </aside>
      </div>

      <section className="panel" aria-labelledby="certs">
        <div className="between"><h2 id="certs" className="label">Certificates · last 24 h</h2><span className="mono small muted">device {site.device.slice(0, 10)}…</span></div>
        <div className="scroll" style={{ marginTop: 12 }}>
          <table>
            <thead><tr><th>HOUR (UTC)</th><th>KWH</th><th>DISPLACED</th><th>FACTOR</th><th>CO₂ AVOIDED</th><th>CERT</th><th>TX</th></tr></thead>
            <tbody>
              {[...certs].reverse().map(c => (
                <tr key={c.id}>
                  <td>{hourUtc(c.hour)}</td>
                  <td><span className="num" style={{ color: 'var(--accent)' }}>{kwh(c.wh, 2)}</span></td>
                  <td><SourceChip src={c.source} /></td>
                  <td style={{ color: 'var(--fg-2)' }}>{c.factor} g/kWh</td>
                  <td><span className="num">{kg(c.g_co2)} kg</span></td>
                  <td><Link to={`/trace/${c.id}`}>{c.short}</Link></td>
                  <td style={{ color: 'var(--fg-2)' }}><Tx hash={c.tx} cfg={cfg} /></td>
                </tr>
              ))}
              {!certs.length && <tr><td colSpan={7} className="muted">No certificates yet. Night hours aren't certified.</td></tr>}
            </tbody>
          </table>
        </div>
      </section>

      <section className="panel row" style={{ gap: 24, alignItems: 'center' }} aria-labelledby="addh">
        <div style={{ flex: '1 1 360px', minWidth: 0 }}>
          <p className="label">Add a meter</p>
          <h2 id="addh" className="h4" style={{ fontSize: 28, margin: '10px 0 8px' }}>The key never leaves the meter.</h2>
          <p className="muted" style={{ margin: 0 }}>The meter generates its own key in a secure element on first boot and signs its registration, so the contract knows the key really lives in that device. Wire the grid sensor to the changeover switch so blackout hours can be proven.</p>
        </div>
        <ol className="list mono small" style={{ flex: '1 1 360px', minWidth: 0 }}>
          <li className="item good" style={{ color: 'var(--accent)' }}>✓ 01 · Meter key {site.device.slice(0, 6)}…{site.device.slice(-4)} registered</li>
          <li className="item good" style={{ color: 'var(--accent)' }}>✓ 02 · Array {kwh(site.capacity_w)} kWp · {site.lat.toFixed(1)}°, {site.lon.toFixed(1)}°</li>
          <li className="item good" style={{ color: 'var(--accent)' }}>✓ 03 · Grid sensor on changeover{site.has_generator ? ' · generator registered' : ''}</li>
          <li className="item muted" style={{ borderStyle: 'dashed' }}>04 · Add another meter (hardware kit on the roadmap)</li>
        </ol>
      </section>
    </div>
  )
}
