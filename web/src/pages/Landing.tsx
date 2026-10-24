import { Link } from 'react-router-dom'
import { api } from '../api'
import { Mark, ThemeToggle, kwh, kg, usePoll } from '../ui'

function HeroArt() {
  const blocks = [['08:00', '1.9'], ['09:00', '3.4'], ['10:00', '5.1', 1], ['11:00', '6.3', 1], ['12:00', '6.8'], ['13:00', '6.5']] as const
  return (
    <svg viewBox="0 0 600 540" role="img" aria-label="The sun's rays fall on rooftop panels while the generator stays off; each hour of output drops into a chain of minted certificates, generator-displacing hours hatched" style={{ width: '100%', height: 'auto', display: 'block' }}>
      <defs>
        <pattern id="hatch" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
          <rect width="5" height="5" fill="var(--accent-deep)" /><rect width="1.6" height="5" fill="var(--accent)" opacity=".45" />
        </pattern>
      </defs>
      <path d="M40 300 A260 260 0 0 1 560 300" fill="none" stroke="var(--line-2)" strokeWidth="1.5" strokeDasharray="4 6" />
      <circle cx="300" cy="96" r="70" fill="var(--accent)" opacity=".08" />
      <circle cx="300" cy="96" r="36" fill="none" stroke="var(--accent)" strokeWidth="8" strokeDasharray="3.1 2.55" />
      <circle cx="300" cy="96" r="24" fill="var(--accent)" />
      {[[290, 232], [300, 300], [310, 368]].map(([x1, x2]) => <line key={x2} className="ray" x1={x1} y1="134" x2={x2} y2="276" stroke="var(--accent)" strokeWidth="2" strokeDasharray="6 6" />)}
      {[196, 246, 296, 346].map(x => <polygon key={x} points={`${x},300 ${x + 14},276 ${x + 58},276 ${x + 44},300`} fill="var(--panel)" stroke="var(--accent)" strokeWidth="1.5" />)}
      <rect x="186" y="300" width="228" height="128" rx="4" fill="var(--panel)" stroke="var(--line-2)" strokeWidth="1.5" />
      <rect x="282" y="372" width="36" height="56" rx="2" fill="none" stroke="var(--line-2)" strokeWidth="1.5" />
      <rect x="210" y="324" width="40" height="28" rx="2" fill="none" stroke="var(--line-2)" strokeWidth="1.5" />
      <rect x="350" y="324" width="40" height="28" rx="2" fill="none" stroke="var(--line-2)" strokeWidth="1.5" />
      <g>
        <rect x="432" y="398" width="54" height="30" rx="3" fill="var(--panel)" stroke="var(--line-2)" strokeWidth="1.5" />
        <rect x="440" y="406" width="14" height="14" rx="2" fill="none" stroke="var(--line-2)" />
        <line x1="478" y1="398" x2="478" y2="384" stroke="var(--line-2)" strokeWidth="3" />
        <circle cx="482" cy="374" r="5" fill="none" stroke="var(--line-2)" strokeDasharray="2 3" />
        <circle cx="490" cy="362" r="7" fill="none" stroke="var(--line-2)" strokeDasharray="2 3" opacity=".6" />
        <line x1="470" y1="370" x2="494" y2="346" stroke="var(--red)" strokeWidth="2" />
        <text x="436" y="444" fontFamily="IBM Plex Mono, monospace" fontSize="9" fill="var(--muted)">GEN · OFF</text>
      </g>
      <rect x="392" y="392" width="16" height="22" rx="2" fill="var(--bg)" stroke="var(--accent)" strokeWidth="1.2" />
      <path className="ray" d="M400 414 V452 H300 V470" fill="none" stroke="var(--accent)" strokeWidth="2" strokeDasharray="6 6" />
      <g fontFamily="IBM Plex Mono, monospace" fontSize="9" fill="var(--muted)">
        {blocks.map(([t, v, g], i) => {
          const x = 20 + i * 72
          return (
            <g key={t}>
              <rect x={x} y="470" width="62" height="54" rx="6" fill={g ? 'url(#hatch)' : 'var(--panel)'} stroke={g ? 'var(--accent-line)' : 'var(--line)'} />
              <text x={x + 10} y="487" fill={g ? 'var(--fg)' : undefined}>{t}</text>
              <text x={x + 10} y="512" fontFamily="Doto, monospace" fontSize="16" fontWeight="900" fill="var(--accent)">{v}</text>
              {g ? <text x={x + 42} y="512" fontSize="7" fill="var(--accent)">GEN</text> : null}
              <line x1={x + 62} y1="497" x2={x + 72} y2="497" stroke="var(--line-2)" />
            </g>
          )
        })}
        <g className="minting">
          <rect x="452" y="470" width="62" height="54" rx="6" fill="var(--accent-deep)" stroke="var(--accent)" strokeWidth="1.5" />
          <text x="462" y="487" fill="var(--accent)">14:00</text>
          <text x="462" y="512" fill="var(--accent)" fontSize="10">MINTING</text>
        </g>
        <rect x="524" y="470" width="62" height="54" rx="6" fill="none" stroke="var(--line)" strokeDasharray="3 4" />
        <text x="534" y="487">15:00</text>
      </g>
    </svg>
  )
}

const STEPS = [
  ['Meter signs', 'Each meter holds its own key and signs every reading it sends, solar output and grid state together.'],
  ['Verified', "Signature, freshness and physics checks. No solar at midnight, no output above the panels' capacity."],
  ['Weighed', "Grid down and a registered generator? The hour counts at the diesel factor. Otherwise, the local grid's."],
  ['Minted', 'Each verified hour becomes exactly one certificate on-chain, so it can never be counted twice.'],
  ['Retired', 'Companies retire certificates for the hours they used power and get a public receipt.'],
]

const ATTACKS = [['Forged signature', '✗ SIGNATURE'], ['Replayed reading', '✗ FRESHNESS'], ['Solar in the dark', '✗ PHYSICS'], ['Fake blackout', '↓ GRID FACTOR'], ['Same hour, minted twice', '✗ ON-CHAIN']]

export default function Landing() {
  const { data } = usePoll(api.network, 30_000)
  const t = data?.totals
  return (
    <div className="landing">
      <div className="ambient" style={{ width: 560, height: 560, left: -280, top: -200 }} />
      <div className="ambient" style={{ width: 480, height: 480, right: -240, top: 900, opacity: .06 }} />

      <nav className="floatnav" aria-label="Main">
        <Link to="/" className="logo"><Mark size={30} />SUNMINT</Link>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
          <span className="links">
            <Link to="/daylight">DAYLIGHT</Link>
            <Link to="/match">MATCH</Link>
            <Link to="/trace">TRACE</Link>
            <Link to="/break-it" style={{ color: 'var(--accent)' }}>BREAK IT</Link>
          </span>
          <ThemeToggle />
          <Link to="/daylight" className="btn primary" style={{ minHeight: 40 }}>Open the app</Link>
        </div>
      </nav>

      <section className="hero">
        <div className="wrap row" style={{ alignItems: 'center', gap: 48 }}>
          <div style={{ flex: '1 1 440px', minWidth: 0 }}>
            <p className="label">Verified clean energy · hour by hour</p>
            <h1 className="h1">Every hour of sunlight, <span className="dot glowtxt">minted</span>.</h1>
            <p className="lead">SUNMINT turns signed meter readings from small solar sites into hourly clean-energy certificates on-chain. Producers earn from every hour. Companies get proof they can match to the hours they actually used power.</p>
            <p style={{ margin: '16px 0 0', maxWidth: 520, fontSize: 16 }}>Each hour is also weighed by what it replaced. Sunlight that switches off a diesel generator avoids far more CO₂ than sunlight on a clean grid, and the certificate says which.</p>
            <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 14 }}>
              <span className="chip"><i className="sw" />GRID DISPLACED</span>
              <span className="chip"><i className="sw gen" />GEN DISPLACED</span>
            </div>
            <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 30 }}>
              <Link to="/daylight" className="btn primary lg">Open the app</Link>
              <Link to="/trace" className="btn lg">Trace a certificate</Link>
            </div>
          </div>
          <div style={{ flex: '1 1 500px', minWidth: 0 }}><HeroArt /></div>
        </div>
      </section>

      <section className="wrap" style={{ paddingBottom: 72 }} aria-label="Network totals">
        <p className="label" style={{ marginBottom: 14 }}>Demo network · 6 sites in 6 countries · live</p>
        <div className="grid" style={{ ['--min' as string]: '240px' }}>
          <div className="panel stat"><p className="num" style={{ color: 'var(--accent)', fontSize: 56 }}>{t ? kwh(t.wh, 0) : '—'}</p><p className="label">kWh verified</p></div>
          <div className="panel stat"><p className="num" style={{ fontSize: 56 }}>{t ? kg(t.g_co2, 0) : '—'}</p><p className="label">kg CO₂ avoided</p></div>
          <div className="panel stat"><p className="num" style={{ fontSize: 56 }}>{t ? t.certs : '—'}</p><p className="label">Certificates minted</p></div>
        </div>
      </section>

      <section className="wrap" style={{ paddingBottom: 80 }} aria-labelledby="how">
        <p className="label">How it works</p>
        <h2 id="how" className="h2">From photon to <span className="dot">proof</span>.</h2>
        <div className="grid" style={{ ['--min' as string]: '200px' }}>
          {STEPS.map(([h, p], i) => (
            <article key={h} className="panel"><p className="num" style={{ margin: 0, fontSize: 28, color: 'var(--accent)' }}>0{i + 1}</p><h3 className="h4">{h}</h3><p className="muted" style={{ margin: 0 }}>{p}</p></article>
          ))}
        </div>
      </section>

      <section className="wrap" style={{ paddingBottom: 80 }} aria-label="Who it is for">
        <div className="grid" style={{ ['--min' as string]: '320px' }}>
          <article className="panel" style={{ padding: 32 }}>
            <p className="label">For producers</p>
            <h3 className="h3" style={{ margin: '14px 0 12px' }}>Your rooftop earns every hour.</h3>
            <p className="muted" style={{ margin: '0 0 20px' }}>Homes, schools, clinics and mini-grids that were too small for certificate markets. Plug in a meter, keep your key, get paid per verified hour.</p>
            <Link to="/rooftop" className="btn">See a rooftop</Link>
          </article>
          <article className="panel" style={{ padding: 32 }}>
            <p className="label">For companies</p>
            <h3 className="h3" style={{ margin: '14px 0 12px' }}>Clean power for the hours you actually use.</h3>
            <p className="muted" style={{ margin: '0 0 20px' }}>Upload your hourly load, see your 24/7 match, retire certificates for the gaps. Every claim links back to a signed meter reading.</p>
            <Link to="/match" className="btn">Match my load</Link>
          </article>
        </div>
      </section>

      <section className="wrap" style={{ paddingBottom: 80 }} aria-labelledby="cheat">
        <div className="panel feature row" style={{ padding: 40, gap: 40, alignItems: 'center', background: 'var(--accent-deep)' }}>
          <div style={{ flex: '1 1 400px', minWidth: 0 }}>
            <p className="label" style={{ color: 'var(--accent)' }}>Live against the real verifier</p>
            <h2 id="cheat" className="h2" style={{ margin: '14px 0' }}>Go on. Try to <span className="dot glowtxt">cheat</span>.</h2>
            <p style={{ margin: '0 0 24px', maxWidth: 460, color: 'var(--fg-2)' }}>Forge a signature, replay yesterday's reading, claim solar in the dark, fake a blackout. Each attack runs through the same pipeline as honest meters. Watch where it gets stopped.</p>
            <Link to="/break-it" className="btn primary lg">Break it</Link>
          </div>
          <ol className="list mono" style={{ flex: '1 1 360px', minWidth: 0, fontSize: 13 }}>
            {ATTACKS.map(([a, s]) => (
              <li key={a} className="item between" style={{ borderColor: 'var(--accent-line)' }}><span>{a}</span><span style={{ color: s.startsWith('↓') ? 'var(--amber)' : 'var(--red-fg)' }}>{s}</span></li>
            ))}
          </ol>
        </div>
      </section>

      <section className="wrap" style={{ paddingBottom: 96 }} aria-label="Open source">
        <div className="panel row" style={{ padding: 32, gap: 32, alignItems: 'center' }}>
          <div style={{ flex: '1 1 360px', minWidth: 0 }}>
            <p className="label">Open source · MIT</p>
            <h3 className="h3" style={{ margin: '14px 0 12px' }}>Don't trust us. <span className="dot">Check</span> the chain.</h3>
            <p className="muted" style={{ margin: 0 }}>The contract, the verifier and the meter code are public. Every certificate resolves to its readings, its checks and its mint transaction.</p>
          </div>
          <pre className="code" style={{ flex: '1 1 380px', minWidth: 0, padding: 18 }}>{`contract   SunmintCertificates
network    Base Sepolia
standard   1 token id = 1 device-hour, balance in Wh
weight     g CO₂ by displaced source (GRID | GEN)
mint()     needs device sig + verifier co-sig
retire()   burns + emits public receipt`}</pre>
        </div>
      </section>

      <footer className="mono" style={{ borderTop: '1px solid var(--line)', padding: '28px 24px 40px', textAlign: 'center', fontSize: 11, letterSpacing: '.08em', color: 'var(--muted)' }}>
        MIT · Built for the IEEE ClimateChain Global Hackathon 2026 · Testnet demo, demo emission factors, not financial advice
      </footer>
    </div>
  )
}
