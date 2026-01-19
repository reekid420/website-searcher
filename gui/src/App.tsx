import React, { useState, useCallback, useMemo } from 'react'
import './App.css'
import { invokeSearch, fetchSites, type SearchResult } from './api'

function App() {
  const [q, setQ] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [results, setResults] = useState<SearchResult[]>([])
  const [siteOptions, setSiteOptions] = useState<string[]>([])
  const [selectedSites, setSelectedSites] = useState<string[]>([])
  const [limit, setLimit] = useState<number>(10)
  const [noCf, setNoCf] = useState<boolean>(false)
  const [cfUrl, setCfUrl] = useState<string>('')
  const [cookie, setCookie] = useState<string>('')
  const [csrinPages, setCsrinPages] = useState<number>(1)
  const [csrinSearch, setCsrinSearch] = useState<boolean>(false)
  const [noPlaywright, setNoPlaywright] = useState<boolean>(false)
  const [debug, setDebug] = useState<boolean>(false)
  const [copiedUrl, setCopiedUrl] = useState<string | null>(null)

  const copyToClipboard = useCallback(async (url: string) => {
    try {
      await navigator.clipboard.writeText(url)
      setCopiedUrl(url)
      setTimeout(() => setCopiedUrl(null), 1500)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }, [])

  // Group results by site
  const groupedResults = useMemo(() => {
    const groups = new Map<string, { site: string; items: { title: string; url: string }[] }>()
    for (const r of results) {
      const key = r.site.toLowerCase()
      if (!groups.has(key)) {
        groups.set(key, { site: r.site, items: [] })
      }
      groups.get(key)!.items.push({ title: r.title, url: r.url })
    }
    return Array.from(groups.values())
  }, [results])

  // Load site list once
  React.useEffect(() => {
    fetchSites().then(setSiteOptions).catch(() => setSiteOptions([]))
  }, [])

  async function onSearch() {
    setError(null)
    if (!q.trim()) {
      setError('Enter a search phrase')
      return
    }
    setLoading(true)
    try {
      const rs = await invokeSearch({
        query: q,
        limit,
        sites: selectedSites.length ? selectedSites : undefined,
        debug,
        no_cf: noCf,
        cf_url: cfUrl || undefined,
        cookie: cookie || undefined,
        csrin_pages: csrinPages,
        csrin_search: csrinSearch,
        no_playwright: noPlaywright,
      })
      setResults(rs)
      console.log('results', rs)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{ padding: 16 }}>
      <h1>Website Searcher</h1>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="e.g., elden ring"
          style={{ flex: 1, padding: 8 }}
        />
        <button onClick={onSearch} disabled={loading}>
          {loading ? 'Searching…' : 'Search'}
        </button>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 12 }}>
        <div>
          <label>Sites</label>
          <div style={{ border: '1px solid #444', padding: 8, maxHeight: 160, overflow: 'auto' }}>
            {siteOptions.map((s) => {
              const checked = selectedSites.includes(s)
              return (
                <label key={s} style={{ display: 'block', cursor: 'pointer' }}>
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={(e) => {
                      setSelectedSites((prev) => {
                        if (e.target.checked) return [...prev, s]
                        return prev.filter((x) => x !== s)
                      })
                    }}
                  />{' '}
                  {s}
                </label>
              )
            })}
            {siteOptions.length === 0 && <div style={{ color: '#888' }}>Loading…</div>}
          </div>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label>
            <span>Limit</span>
            <input type="number" min={1} value={limit} onChange={(e) => setLimit(Number(e.target.value) || 1)} style={{ width: '100%', padding: 6 }} />
          </label>
          <label>
            <span>CF URL</span>
            <input value={cfUrl} onChange={(e) => setCfUrl(e.target.value)} placeholder="http://localhost:8191/v1" style={{ width: '100%', padding: 6 }} />
          </label>
          <label style={{ gridColumn: '1 / span 2' }}>
            <span>Cookie</span>
            <input value={cookie} onChange={(e) => setCookie(e.target.value)} placeholder="key=value; other=value2" style={{ width: '100%', padding: 6 }} />
          </label>
          <label>
            <span>csrin_pages</span>
            <input type="number" min={1} value={csrinPages} onChange={(e) => setCsrinPages(Number(e.target.value) || 1)} style={{ width: '100%', padding: 6 }} />
          </label>
          <label>
            <input type="checkbox" checked={csrinSearch} onChange={(e) => setCsrinSearch(e.target.checked)} /> csrin_search
          </label>
          <label>
            <input type="checkbox" checked={noPlaywright} onChange={(e) => setNoPlaywright(e.target.checked)} /> no_playwright
          </label>
          <label>
            <input type="checkbox" checked={noCf} onChange={(e) => setNoCf(e.target.checked)} /> no_cf
          </label>
          <label>
            <input type="checkbox" checked={debug} onChange={(e) => setDebug(e.target.checked)} /> debug
          </label>
        </div>
      </div>
      {error && <p style={{ color: 'tomato' }}>{error}</p>}
      <div className="results-container">
        {groupedResults.map((group, i) => (
          <div key={i} className="result-card">
            <h3 className="result-title">{group.site}</h3>
            <div className="result-links">
              {group.items.map((item, j) => (
                <div key={j} className="link-row">
                  <span
                    className="copy-link"
                    onClick={() => copyToClipboard(item.url)}
                    title="Click to copy"
                  >
                    {item.url}
                    {copiedUrl === item.url && <span className="copied-toast">Copied!</span>}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
        {results.length === 0 && !loading && <p>No results yet.</p>}
      </div>
    </div>
  )
}

export default App
