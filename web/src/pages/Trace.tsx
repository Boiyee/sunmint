import { useEffect, useState, type FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { api, type Trace as T } from '../api'
import { SourceChip, Tx, addrUrl, hourUtc, kg, kwh, short, useConfig } from '../ui'

export default function Trace() {
  const { id } = useParams()
  const nav = useNavigate()
  const cfg = useConfig()
  const [q, setQ] = useState(id ?? '')
  const [t, setT] = useState<T | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    if (id) { setQ(id); api.trace(id).then(x => (setT(x), setErr(null))).catch(e => (setT(null), setErr(e.message))) }
    else api.market().then(({ certs }) => { const c = certs.find(c => c.source === 'GEN') ?? certs[0]; if (c) nav(`/trace/${c.id}`, { replace: true }) }).catch(() => {})
  }, [id, nav])

  const submit = (e: FormEvent) => { e.preventDefault(); if (q.trim()) nav(`/trace/${q.trim()}`) }

  const c = t?.cert
  const gridV = t?.readings.map(r => r.grid_v) ?? []
  const steps = t && c ? [
    { title: 'Device', badge: '✓ KEY ON-CHAIN', body: `A meter with its own key. It signed its own registration, so the contract knows the key lives in that device.`,
      facts: [`device ${short(t.site?.device ?? '')}`, `${kwh(t.site?.capacity_w ?? 0)} kWp`, t.site?.has_generator ? 'generator registered' : 'no generator'] },
    { title: 'Signed readings', badge: `✓ ${t.readings.length} OF ${t.readings.length}`, body: 'Five-minute readings, each signed over solar output, grid voltage, timestamp and a nonce.',
      facts: [`${hourUtc(c.hour)} UTC`, `nonces ${t.readings[0]?.nonce}–${t.readings.at(-1)?.nonce}`, `root ${short(t.root)}`] },
    { title: 'Verifier checks', badge: t.root_matches ? '✓ ROOT MATCHES' : '✗ ROOT MISMATCH', body: 'Every signature matched the registered key, every nonce was new, and the output fit the sun angle and stayed under the array’s rating. Recomputing the Merkle root from the readings above gives the root stored on-chain.',
      facts: ['sig ✓', 'freshness ✓', `peak ${Math.max(...t.readings.map(r => r.wh))} Wh / 5 min ✓`] },
    { title: 'Displacement evidence', badge: c.source === 'GEN' ? '✓ GEN HOUR' : '✓ GRID HOUR', body: c.source === 'GEN'
        ? `The signed grid channel read 0 V for the whole hour and a generator is registered at this site, so this hour displaced diesel.`
        : `The grid was up for at least part of the hour${t.site?.has_generator ? '' : ', or no generator is registered'}, so the conservative grid factor applies.`,
      facts: [`grid ${Math.min(...gridV)}–${Math.max(...gridV)} V`, `factor ${c.factor} g/kWh`] },
    { title: 'Minted', badge: '✓ ONE PER DEVICE-HOUR', body: 'The contract checked the device signature and the verifier co-signature, then minted exactly one certificate for this device and hour.',
      facts: [`tx ${short(c.tx)}`, `block ${c.block.toLocaleString()}`, `co-sig ${short(t.attest_sig)}`] },
    ...(t.retirements.length ? [{ title: 'Retired', badge: '✓ BURNED', body: `Retired for ${t.retirements.map(r => r.beneficiary).join(', ')}. Those Wh are burned and can never be claimed again.`,
      facts: t.retirements.map(r => `receipt #${r.receipt}`) }] : []),
  ] : []

  return (
    <div className="page" style={{ maxWidth: 1200 }}>
      <div>
        <p className="label">Public · no account needed</p>
        <h1 className="h2" style={{ margin: '10px 0 0' }}>Don't trust it. <span className="dot">Trace</span> it.</h1>
        <form onSubmit={submit} style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 20, maxWidth: 760 }}>
          <label className="sr" htmlFor="cert">Certificate ID, short code or transaction hash</label>
          <input id="cert" value={q} onChange={e => setQ(e.target.value)} placeholder="Certificate ID, #CODE or tx hash" style={{ flex: '1 1 320px', minWidth: 0 }} />
          <button className="btn primary lg">Trace certificate</button>
        </form>
        {err && <p className="mono small" style={{ color: 'var(--red-fg)' }}>{err}</p>}
      </div>

      {t && c && (
        <div className="row" style={{ alignItems: 'flex-start' }}>
          <section className="panel grow" aria-labelledby="chain" style={{ padding: 28 }}>
            <h2 id="chain" className="label" style={{ marginBottom: 22 }}>Chain of custody · {steps.length} steps</h2>
            <ol className="tl">
              {steps.map((s, i) => (
                <li key={s.title}>
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                    <span className="node" style={i === steps.length - 1 ? { background: 'var(--accent)', color: 'var(--accent-ink)' } : undefined}>0{i + 1}</span>
                    {i < steps.length - 1 && <span className="stem" />}
                  </div>
                  <div style={{ paddingBottom: 26, minWidth: 0 }}>
                    <div className="between" style={{ alignItems: 'baseline' }}><h3 className="h4" style={{ margin: '4px 0 0' }}>{s.title}</h3><span className="mono small" style={{ color: s.badge.startsWith('✗') ? 'var(--red-fg)' : 'var(--accent)' }}>{s.badge}</span></div>
                    <p style={{ margin: '6px 0 0', color: 'var(--fg-2)' }}>{s.body}</p>
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 10 }}>{s.facts.map(f => <span key={f} className="fact">{f}</span>)}</div>
                  </div>
                </li>
              ))}
            </ol>
            <details style={{ marginTop: 4 }}>
              <summary className="label" style={{ cursor: 'pointer' }}>Raw signed readings</summary>
              <div className="scroll" style={{ marginTop: 10 }}>
                <table>
                  <thead><tr><th>TS</th><th>WH</th><th>GRID V</th><th>NONCE</th><th>SIGNATURE</th><th>LEAF</th></tr></thead>
                  <tbody>{t.readings.map(r => <tr key={r.nonce}><td>{new Date(r.ts * 1000).toISOString().slice(11, 16)}</td><td>{r.wh}</td><td>{r.grid_v}</td><td>{r.nonce}</td><td>{short(r.sig, 10, 6)}</td><td>{short(r.leaf, 10, 6)}</td></tr>)}</tbody>
                </table>
              </div>
            </details>
          </section>

          <aside className="side stack" style={{ gap: 16 }}>
            <div className="panel feature" style={{ padding: 24 }}>
              <div className="between"><h2 className="label">Certificate {c.short}</h2><span className="chip" style={t.retirements.length ? { background: 'var(--fg)', color: 'var(--bg)', borderColor: 'var(--fg)' } : undefined}>{t.retirements.length ? 'RETIRED' : 'ACTIVE'}</span></div>
              <p className="num" style={{ margin: '16px 0 0', fontSize: 56, lineHeight: 1, color: 'var(--accent)' }}>{kwh(c.wh, 2)}<span style={{ fontSize: 22 }}> kWh</span></p>
              <p style={{ margin: '4px 0 0', color: 'var(--fg-2)' }}>{c.site} · {hourUtc(c.hour)} UTC</p>
              <div className="grid" style={{ ['--min' as string]: '120px', gap: 10, marginTop: 18 }}>
                <div className="item"><p className="num" style={{ margin: 0, fontSize: 24 }}>{kg(c.g_co2)}</p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>kg CO₂ avoided</p></div>
                <div className="item"><p className="num" style={{ margin: 0, fontSize: 24 }}><SourceChip src={c.source} /></p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>{c.factor} g/kWh</p></div>
              </div>
              <div className="stack" style={{ marginTop: 18 }}>
                {t.retirements[0] && <Link to={`/proof/${t.retirements[0].receipt}`} className="btn primary">View retirement receipt</Link>}
                <a className="btn" download={`sunmint-${c.short.slice(1)}.json`} href={`data:application/json,${encodeURIComponent(JSON.stringify(t, null, 2))}`}>Download proof bundle (JSON)</a>
              </div>
            </div>
            <Link to="/break-it" className="panel" style={{ display: 'block', textDecoration: 'none', background: 'var(--accent-deep)', borderColor: 'var(--accent-line)' }}>
              <p className="label" style={{ color: 'var(--accent)' }}>Think it's fake?</p>
              <p className="h4" style={{ fontSize: 26, margin: '8px 0 0' }}>Try to forge one like it →</p>
              <p className="muted small" style={{ margin: '8px 0 0' }}>Six attacks against the same verifier and contract.</p>
            </Link>
            <div className="panel">
              <p className="label">How CO₂ was counted</p>
              <p className="mono small" style={{ margin: '10px 0 0', lineHeight: 1.7, color: 'var(--fg-2)' }}>
                {kwh(c.wh, 3)} kWh × {c.factor} g/kWh = {kg(c.g_co2)} kg<br />
                {c.source === 'GEN' ? 'Grid 0 V all hour and a registered generator, so the diesel factor applies.' : 'Grid present or no generator, so the grid factor applies.'} Any doubt defaults to the grid factor. Rule enforced in the contract.
              </p>
              <p className="mono small muted" style={{ margin: '10px 0 0' }}>Verifier {addrUrl(cfg, t.verifier) ? <a href={addrUrl(cfg, t.verifier)} target="_blank" rel="noreferrer">{short(t.verifier)}</a> : short(t.verifier)} · tx <Tx hash={c.tx} cfg={cfg} /></p>
            </div>
          </aside>
        </div>
      )}
    </div>
  )
}
