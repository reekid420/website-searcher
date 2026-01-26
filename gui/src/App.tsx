import { useState, useCallback, useMemo, useEffect } from 'react'
import './App.css'
import { 
  invokeSearch, 
  fetchSites, 
  type SearchResult,
  type CacheEntry,
  getCache,
  getCachedResults,
  addToCache,
  removeCacheEntry,
  clearCache as apiClearCache,
  getCacheSettings,
  setCacheSize as apiSetCacheSize
} from './api'

// Cache configuration constants
const MIN_CACHE_SIZE = 3
const MAX_CACHE_SIZE = 20

function App() {
  const [q, setQ] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [results, setResults] = useState<SearchResult[]>([])
  const [siteOptions, setSiteOptions] = useState<string[]>([])
  const [selectedSites, setSelectedSites] = useState<string[]>([])
  const [limit, setLimit] = useState<number>(10)
  const [cutoff, setCutoff] = useState<number>(0)
  const [noCf, setNoCf] = useState<boolean>(false)
  const [cfUrl, setCfUrl] = useState<string>('')
  const [cookie, setCookie] = useState<string>('')
  const [csrinPages, setCsrinPages] = useState<number>(1)
  const [csrinSearch, setCsrinSearch] = useState<boolean>(false)
  const [noPlaywright, setNoPlaywright] = useState<boolean>(false)
  const [noRateLimit, setNoRateLimit] = useState<boolean>(false)
  const [debug, setDebug] = useState<boolean>(false)
  const [copiedUrl, setCopiedUrl] = useState<string | null>(null)

  // Cache state (now from Tauri backend)
  const [cache, setCache] = useState<CacheEntry[]>([])
  const [cacheSize, setCacheSize] = useState<number>(MIN_CACHE_SIZE)
  const [showSettings, setShowSettings] = useState(false)
  const [cacheHit, setCacheHit] = useState(false)

  // Load cache and settings from Tauri backend
  useEffect(() => {
    getCache().then(setCache).catch(console.error)
    getCacheSettings().then(setCacheSize).catch(console.error)
  }, [])

  // Reload cache from backend
  const reloadCache = useCallback(async () => {
    try {
      const entries = await getCache()
      setCache(entries)
    } catch (e) {
      console.error('Failed to reload cache:', e)
    }
  }, [])

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
  useEffect(() => {
    fetchSites().then(setSiteOptions).catch(() => setSiteOptions([]))
  }, [])

  // Load a cached search
  const loadCachedSearch = useCallback(async (entry: CacheEntry) => {
    setQ(entry.query)
    try {
      const cachedResults = await getCachedResults(entry.query)
      if (cachedResults) {
        setResults(cachedResults)
        setCacheHit(true)
        setTimeout(() => setCacheHit(false), 2000)
      }
    } catch (e) {
      console.error('Failed to load cached results:', e)
    }
  }, [])

  // Delete a cache entry
  const deleteEntry = useCallback(async (query: string, event: React.MouseEvent) => {
    event.stopPropagation() // Don't trigger loadCachedSearch
    try {
      await removeCacheEntry(query)
      await reloadCache()
    } catch (e) {
      console.error('Failed to delete cache entry:', e)
    }
  }, [reloadCache])

  // Clear all cache
  const handleClearCache = useCallback(async () => {
    try {
      await apiClearCache()
      setCache([])
    } catch (e) {
      console.error('Failed to clear cache:', e)
    }
  }, [])

  // Update cache size
  const handleSetCacheSize = useCallback(async (size: number) => {
    try {
      await apiSetCacheSize(size)
      setCacheSize(size)
      await reloadCache() // Reload in case entries were evicted
    } catch (e) {
      console.error('Failed to set cache size:', e)
    }
  }, [reloadCache])

  async function onSearch() {
    setError(null)
    setCacheHit(false)
    if (!q.trim()) {
      setError('Enter a search phrase')
      return
    }

    // Check cache first
    try {
      const cached = await getCachedResults(q)
      if (cached) {
        setResults(cached)
        setCacheHit(true)
        setTimeout(() => setCacheHit(false), 2000)
        console.log('Cache hit for:', q)
        return
      }
    } catch (e) {
      console.error('Cache lookup failed:', e)
    }

    setLoading(true)
    try {
      const rs = await invokeSearch({
        query: q,
        limit,
        cutoff: cutoff || undefined,
        sites: selectedSites.length ? selectedSites : undefined,
        debug,
        no_cf: noCf,
        cf_url: cfUrl || undefined,
        cookie: cookie || undefined,
        csrin_pages: csrinPages,
        csrin_search: csrinSearch,
        no_playwright: noPlaywright,
        no_rate_limit: noRateLimit,
      })
      setResults(rs)
      // Add to cache if we got results
      if (rs.length > 0) {
        try {
          await addToCache(q, rs)
          await reloadCache()
        } catch (e) {
          console.error('Failed to cache results:', e)
        }
      }
      console.log('results', rs)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
        <h1 style={{ margin: 0 }}>Website Searcher</h1>
        <button
          onClick={() => setShowSettings(!showSettings)}
          style={{ padding: '6px 12px', cursor: 'pointer' }}
        >
          ⚙️ Settings
        </button>
      </div>

      {/* Settings Panel */}
      {showSettings && (
        <div className="settings-panel" style={{ 
          border: '1px solid #444', 
          borderRadius: 8, 
          padding: 16, 
          marginBottom: 16,
          background: '#1a1a1a'
        }}>
          <h3 style={{ marginTop: 0 }}>Settings</h3>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 12 }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span>Cache Size:</span>
              <input
                type="range"
                min={MIN_CACHE_SIZE}
                max={MAX_CACHE_SIZE}
                value={cacheSize}
                onChange={(e) => handleSetCacheSize(Number(e.target.value))}
              />
              <span style={{ minWidth: 24 }}>{cacheSize}</span>
            </label>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={handleClearCache} style={{ padding: '6px 12px' }}>
              🗑️ Clear Cache ({cache.length} entries)
            </button>
          </div>
          <p style={{ fontSize: 12, color: '#888', marginBottom: 0 }}>
            Cache is shared with CLI/TUI
          </p>
        </div>
      )}

      {/* Recent Searches */}
      {cache.length > 0 && (
        <div className="recent-searches" style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Recent searches:</div>
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            {cache.slice(0, 5).map((entry, i) => (
              <div
                key={i}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '4px 8px',
                  fontSize: 12,
                  border: '1px solid #555',
                  borderRadius: 16,
                  background: '#2a2a2a',
                  color: '#ddd',
                }}
              >
                <button
                  onClick={() => loadCachedSearch(entry)}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: '#ddd',
                    cursor: 'pointer',
                    padding: 0,
                    fontSize: 12
                  }}
                  title={`${entry.result_count} results`}
                >
                  {entry.query}
                </button>
                <button
                  onClick={(e) => deleteEntry(entry.query, e)}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: '#888',
                    cursor: 'pointer',
                    padding: '0 2px',
                    fontSize: 10,
                    lineHeight: 1
                  }}
                  title="Remove from cache"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="e.g., elden ring"
          style={{ flex: 1, padding: 8 }}
          onKeyDown={(e) => e.key === 'Enter' && onSearch()}
        />
        <button onClick={onSearch} disabled={loading}>
          {loading ? 'Searching…' : 'Search'}
        </button>
      </div>

      {/* Cache hit indicator */}
      {cacheHit && (
        <div style={{ 
          background: '#2d5a2d', 
          color: '#8f8', 
          padding: '4px 12px', 
          borderRadius: 4, 
          marginBottom: 8,
          fontSize: 13
        }}>
          ⚡ Results loaded from cache (shared with CLI)
        </div>
      )}

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
            <span>Cutoff (total)</span>
            <input type="number" min={0} value={cutoff} onChange={(e) => setCutoff(Number(e.target.value) || 0)} style={{ width: '100%', padding: 6 }} title="Maximum total results across all sites (0 = no limit)" />
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
            <input type="checkbox" checked={noRateLimit} onChange={(e) => setNoRateLimit(e.target.checked)} /> no_rate_limit
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
