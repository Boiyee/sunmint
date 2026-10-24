import { useState } from 'react'
import { api, type Attack, type Stage } from '../api'

const ATTACKS = [
  ['forge', 'Forge a signature', 'Edit a signed reading and resend it with the old signature.'],
  ['replay', 'Replay a reading', 'Resend a valid reading, untouched, signature and all.'],
  ['night', 'Solar in the dark', 'Use a stolen meter key to sign output while the sun is down.'],
  ['capacity', 'Exceed capacity', 'Claim more in five minutes than the panels can physically produce.'],
  ['blackout', 'Fake a blackout', 'Claim the generator was displaced while the grid was up.'],
  ['double', 'Mint the same hour twice', 'Send a second honest claim for an hour already minted.'],
] as const

const tone = (s: Stage['status']) =>
  s === 'pass' ? { c: 'var(--accent)', b: 'var(--accent-line)', bg: 'var(--accent-deep)', m: '✓' }
  : s === 'reject' ? { c: 'var(--red-fg)', b: 'var(--red)', bg: 'color-mix(in srgb, var(--red) 8%, transparent)', m: '✗' }
  : s === 'downgrade' ? { c: 'var(--amber)', b: 'var(--amber)', bg: 'color-mix(in srgb, var(--amber) 8%, transparent)', m: '↓' }
  : { c: 'var(--faint)', b: 'var(--line)', bg: 'var(--sunk)', m: '–' }

export default function BreakIt() {
  const [sel, setSel] = useState<string>('forge')
  const [res, setRes] = useState<Record<string, Attack>>({})
  const [busy, setBusy] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const run = async (k: string) => {
    setSel(k); setBusy(k); setErr(null)
    try { const a = await api.attack(k); setRes(r => ({ ...r, [k]: a })) } catch (e) { setErr((e as Error).message) } finally { setBusy(null) }
  }
  const runAll = async () => { for (const [k] of ATTACKS) await run(k) }

  const a = res[sel]
  const ran = Object.values(res)
  const stopped = ran.filter(x => x.report.verdict !== 'pass').length
  const v = a ? tone(a.report.verdict) : null
  return (
    <div className="page" style={{ maxWidth: 1260 }}>
      <div className="between" style={{ alignItems: 'flex-end' }}>
        <div style={{ minWidth: 0 }}>
          <p className="label">Real verifier · real contract rules · stolen demo meter keys</p>
          <h1 className="h1" style={{ fontSize: 'clamp(40px, 6vw, 72px)', margin: '10px 0 0' }}>Go on. Try to <span className="dot glowtxt">cheat</span>.</h1>
          <p className="muted" style={{ margin: '12px 0 0', maxWidth: 580 }}>Each attack is built from live data and sent through the same pipeline as honest meters, exactly as a dishonest meter would send it.</p>
        </div>
        <div style={{ display: 'flex', gap: 12 }}>
          <div className="panel" style={{ padding: '14px 18px' }}><p className="num" style={{ margin: 0, fontSize: 36, lineHeight: 1 }}>{ran.length}</p><p className="label" style={{ fontSize: 10, marginTop: 6 }}>Attacks run</p></div>
          <div className="panel" style={{ padding: '14px 18px', background: 'var(--accent-deep)', borderColor: 'var(--accent-line)' }}><p className="num" style={{ margin: 0, fontSize: 36, lineHeight: 1, color: 'var(--accent)' }}>{stopped}</p><p className="label" style={{ fontSize: 10, marginTop: 6, color: 'var(--accent)' }}>Stopped</p></div>
        </div>
      </div>

      <div className="row" style={{ alignItems: 'flex-start' }}>
        <section className="panel side" aria-labelledby="attacks" style={{ padding: 18 }}>
          <div className="between" style={{ padding: '4px 4px 12px' }}>
            <h2 id="attacks" className="label">Pick an attack</h2>
            <button type="button" className="btn sm" onClick={runAll} disabled={!!busy}>RUN ALL 6</button>
          </div>
          <div className="stack">
            {ATTACKS.map(([k, title, how], i) => {
              const r = res[k], on = k === sel
              const tag = busy === k ? ['RUNNING…', 'var(--fg-2)'] : r ? (r.report.verdict === 'downgrade' ? ['↓ DOWNGRADED', 'var(--amber)'] : r.report.verdict === 'reject' ? ['✗ STOPPED', 'var(--red-fg)'] : ['PASSED?!', 'var(--red-fg)']) : ['RUN', 'var(--faint)']
              return (
                <button key={k} type="button" onClick={() => run(k)} aria-pressed={on} disabled={!!busy}
                  style={{ textAlign: 'left', display: 'flex', gap: 14, alignItems: 'flex-start', padding: 14, borderRadius: 12, border: `1px solid ${on ? 'var(--accent)' : 'var(--line)'}`, background: on ? 'var(--accent-deep)' : 'var(--sunk)' }}>
                  <span className="num" style={{ width: 30, fontSize: 18, color: on ? 'var(--accent)' : 'var(--muted)' }}>0{i + 1}</span>
                  <span style={{ flex: 1, minWidth: 0 }}>
                    <span style={{ display: 'block', font: "600 16px 'Inter Tight', sans-serif" }}>{title}</span>
                    <span className="muted small" style={{ display: 'block', marginTop: 4 }}>{r?.how ?? how}</span>
                  </span>
                  <span className="mono" style={{ fontSize: 10, letterSpacing: '.1em', color: tag[1] }}>{tag[0]}</span>
                </button>
              )
            })}
          </div>
          {err && <p className="mono small" style={{ color: 'var(--red-fg)', margin: '12px 4px 0' }}>{err}</p>}
        </section>

        <section className="grow stack" style={{ gap: 16 }} aria-live="polite">
          {!a ? (
            <div className="panel" style={{ padding: 40, textAlign: 'center' }}>
              <p className="label">Pick an attack on the left</p>
              <p className="muted" style={{ margin: '10px 0 0' }}>Or run all six. Each takes about a second.</p>
            </div>
          ) : (
            <>
              <div className="panel">
                <div className="between"><h2 className="label">Signed message · as sent</h2><span className="mono small" style={{ color: 'var(--red-fg)' }}>{a.note}</span></div>
                <pre className="code" style={{ marginTop: 14 }}>
                  <span>{'{'}</span>
                  {a.payload.map((f, i) => <span key={f.k} className={f.edited ? 'ed' : ''}>{`  "${f.k}": ${/^\d+$|^true$|^false$/.test(f.v) ? f.v : `"${f.v}"`}${i < a.payload.length - 1 ? ',' : ''}${f.edited ? (a.kind === 'replay' ? '   ← already used' : '   ← edited') : ''}`}</span>)}
                  <span>{'}'}</span>
                </pre>
              </div>
              <div className="panel">
                <h2 className="label" style={{ marginBottom: 14 }}>Verifier pipeline</h2>
                <ol className="list grid" style={{ ['--min' as string]: '130px', gap: 8 }}>
                  {a.report.stages.map((s, i) => {
                    const t = tone(s.status)
                    return (
                      <li key={s.name} style={{ padding: 12, borderRadius: 10, border: `1px solid ${t.b}`, background: t.bg }} title={s.detail}>
                        <p className="mono muted" style={{ margin: 0, fontSize: 10, letterSpacing: '.12em' }}>0{i + 1} · {s.name.replace('_', '-')}</p>
                        <p className="mono" style={{ margin: '8px 0 0', fontWeight: 600, color: t.c }}>{t.m} {s.status === 'downgrade' ? 'grid factor' : s.status}</p>
                      </li>
                    )
                  })}
                </ol>
              </div>
              <div key={sel + a.report.reason} className="panel stamp row" style={{ gap: 24, alignItems: 'center', border: `1px solid ${v!.b}`, background: v!.bg, boxShadow: `0 0 70px -18px ${v!.c}` }}>
                <p className="num" style={{ margin: 0, fontSize: 'clamp(40px, 5vw, 60px)', lineHeight: 1, color: v!.c }}>{a.report.verdict === 'downgrade' ? 'DOWNGRADED' : a.report.verdict === 'reject' ? 'REJECTED' : 'ACCEPTED'}</p>
                <div style={{ flex: '1 1 280px', minWidth: 0 }}>
                  <p style={{ margin: 0, font: "600 17px 'Inter Tight', sans-serif" }}>{a.report.reason}</p>
                  <p className="mono small" style={{ margin: '6px 0 0', color: 'var(--fg-2)' }}>{a.outcome}</p>
                </div>
              </div>
            </>
          )}
        </section>
      </div>
    </div>
  )
}
