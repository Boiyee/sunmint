export const API = (import.meta.env.VITE_API_URL ?? 'http://localhost:8080').replace(/\/$/, '')

export type Source = 'GRID' | 'GEN'

export interface Cert {
  id: string
  short: string
  slug: string
  site: string
  country: string
  hour: number
  wh: number
  source: Source
  factor: number
  g_co2: number
  tx: string
  block: number
  minted_at: number
  listing: number | null
  price_per_kwh: number
  available_wh: number
}

export interface Site {
  slug: string
  name: string
  country: string
  lat: number
  lon: number
  device: string
  capacity_w: number
  has_generator: boolean
  grid_factor: number
  utc_offset: number
  live_w?: number
  grid_down?: boolean
  sun?: number
  today_wh?: number
}

export interface Rejection { device: string; kind: string; ts: number; verdict: string; reason: string; at: number }

export interface Stage { name: string; status: 'pass' | 'reject' | 'downgrade' | 'skipped'; detail: string }
export interface Report { stages: Stage[]; verdict: Stage['status']; reason: string }

export interface Network {
  now: number
  totals: { wh: number; g_co2: number; certs: number; g_retired: number; live_w: number }
  sites: Site[]
  feed: Cert[]
  rejections: Rejection[]
}

export interface HourRow { hour: number; wh: number; readings: number; grid_down: boolean; cert: Cert | null }
export interface SiteView {
  site: Site
  now: number
  summary: { wh: number; g_co2: number; g_co2_gen: number; certs: number; gen_hours: number; earned: number }
  hours: HourRow[]
  stream: { ts: number; wh: number; grid_v: number; nonce: number }[]
  rejections: Rejection[]
  certs: Cert[]
}

export interface Retirement { receipt: number; beneficiary: string; cert_ids: string[]; whs: number[]; g_co2: number; cost: number; tx: string; at: number }

export interface Trace {
  cert: Cert
  site: Site | null
  readings: { ts: number; wh: number; grid_v: number; nonce: number; sig: string; leaf: string }[]
  root: string
  root_matches: boolean
  claim_sig: string
  attest_sig: string
  verifier: string
  retirements: Retirement[]
}

export interface Pick { id: string; listing: number | null; wh: number }
export interface MatchResult {
  hours: { hour: number; demand_wh: number; matched_wh: number; gen_wh: number }[]
  match_pct: number
  matched_wh: number
  demand_wh: number
  g_co2: number
  cost: number
  picks: Pick[]
  certificates: number
}

export interface Attack {
  kind: string
  title: string
  how: string
  note: string
  payload: { k: string; v: string; edited: boolean }[]
  report: Report
  outcome: string
}

export interface Config { live: boolean; chainId: number; certs: string; market: string; usd: string; verifier: string; explorer: string; genFactor: number }

async function req<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(API + path, body === undefined ? undefined : { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  const j = await r.json()
  if (!r.ok) throw new Error(j.error ?? r.statusText)
  return j as T
}

export const api = {
  config: () => req<Config>('/v1/config'),
  network: () => req<Network>('/v1/network'),
  site: (slug: string) => req<SiteView>(`/v1/sites/${slug}`),
  market: () => req<{ certs: Cert[] }>('/v1/market'),
  trace: (id: string) => req<Trace>(`/v1/certs/${encodeURIComponent(id)}`),
  match: (load_kwh: number[]) => req<MatchResult>('/v1/match', { load_kwh }),
  retire: (picks: Pick[], beneficiary: string) => req<Retirement>('/v1/retire', { picks, beneficiary }),
  receipt: (n: number) => req<{ receipt: Retirement; certs: Cert[] }>(`/v1/receipts/${n}`),
  attack: (kind: string) => req<Attack>(`/v1/attacks/${kind}`, {}),
}
