import { StrictMode, lazy, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Route, Routes, useLocation } from 'react-router-dom'
import { useEffect } from 'react'
import '@fontsource/inter-tight/latin-800.css'
import '@fontsource/inter-tight/latin-900.css'
import '@fontsource/doto/latin-700.css'
import '@fontsource/doto/latin-900.css'
import '@fontsource/geist/latin-400.css'
import '@fontsource/geist/latin-500.css'
import '@fontsource/geist/latin-600.css'
import '@fontsource/ibm-plex-mono/latin-400.css'
import '@fontsource/ibm-plex-mono/latin-500.css'
import '@fontsource/ibm-plex-mono/latin-600.css'
import './styles.css'
import { AppShell, Loading, applyTheme } from './ui'
import Landing from './pages/Landing'

const Daylight = lazy(() => import('./pages/Daylight'))
const Rooftop = lazy(() => import('./pages/Rooftop'))
const Match = lazy(() => import('./pages/Match'))
const Trace = lazy(() => import('./pages/Trace'))
const Proof = lazy(() => import('./pages/Proof'))
const BreakIt = lazy(() => import('./pages/BreakIt'))

applyTheme()

const TITLES: Record<string, string> = { daylight: 'Daylight', rooftop: 'Rooftop', match: 'Match', trace: 'Trace', proof: 'Proof', 'break-it': 'Break it' }
function Title() {
  const { pathname } = useLocation()
  useEffect(() => {
    const t = TITLES[pathname.split('/')[1]]
    document.title = t ? `SUNMINT · ${t}` : 'SUNMINT · Every hour of sunlight, minted'
    window.scrollTo(0, 0)
  }, [pathname])
  return null
}

const app = (el: React.ReactNode) => <AppShell><Suspense fallback={<Loading />}>{el}</Suspense></AppShell>

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Title />
      <Routes>
        <Route path="/" element={<Landing />} />
        <Route path="/daylight" element={app(<Daylight />)} />
        <Route path="/rooftop/:slug?" element={app(<Rooftop />)} />
        <Route path="/match" element={app(<Match />)} />
        <Route path="/trace/:id?" element={app(<Trace />)} />
        <Route path="/proof/:n" element={app(<Proof />)} />
        <Route path="/break-it" element={app(<BreakIt />)} />
        <Route path="*" element={<Landing />} />
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)

if ('serviceWorker' in navigator && import.meta.env.PROD) navigator.serviceWorker.register('/sw.js').catch(() => {})
