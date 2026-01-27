import { useState, useEffect, useCallback, useRef } from 'react'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { 
  invokeSearchStreaming, 
  type SearchArgs, 
  type SearchResult, 
  type SearchProgress, 
  type StreamedResult, 
  type SearchComplete 
} from '../api'

export type SiteProgress = {
  site: string
  status: 'pending' | 'fetching' | 'parsing' | 'completed' | 'failed'
  resultsCount: number
  message?: string
}

type UseRealtimeSearchReturn = {
  results: SearchResult[]
  progress: Map<string, SiteProgress>
  isSearching: boolean
  error: string | null
  completionInfo: SearchComplete | null
  startSearch: (args: SearchArgs) => Promise<void>
  clearResults: () => void
}

/**
 * React hook for real-time streaming search with per-site progress updates.
 * 
 * @example
 * ```tsx
 * const { results, progress, isSearching, startSearch } = useRealtimeSearch()
 * 
 * // Start a streaming search
 * await startSearch({ query: 'elden ring', limit: 10 })
 * 
 * // Results and progress update in real-time
 * progress.forEach((p, site) => console.log(`${site}: ${p.status}`))
 * ```
 */
export function useRealtimeSearch(): UseRealtimeSearchReturn {
  const [results, setResults] = useState<SearchResult[]>([])
  const [progress, setProgress] = useState<Map<string, SiteProgress>>(new Map())
  const [isSearching, setIsSearching] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [completionInfo, setCompletionInfo] = useState<SearchComplete | null>(null)
  
  // Track unsubscribe functions
  const unlistenRefs = useRef<UnlistenFn[]>([])

  // Cleanup listeners on unmount
  useEffect(() => {
    return () => {
      unlistenRefs.current.forEach((unlisten) => unlisten())
      unlistenRefs.current = []
    }
  }, [])

  const clearResults = useCallback(() => {
    setResults([])
    setProgress(new Map())
    setError(null)
    setCompletionInfo(null)
  }, [])

  const startSearch = useCallback(async (args: SearchArgs) => {
    // Clear previous state
    clearResults()
    setIsSearching(true)

    // Cleanup any existing listeners
    unlistenRefs.current.forEach((unlisten) => unlisten())
    unlistenRefs.current = []

    try {
      // Set up event listeners before starting the search
      const unlistenProgress = await listen<SearchProgress>('search:progress', (event) => {
        const data = event.payload
        setProgress((prev) => {
          const updated = new Map(prev)
          updated.set(data.site, {
            site: data.site,
            status: data.status as SiteProgress['status'],
            resultsCount: data.results_count,
            message: data.message,
          })
          return updated
        })
      })
      unlistenRefs.current.push(unlistenProgress)

      const unlistenResult = await listen<StreamedResult>('search:result', (event) => {
        const data = event.payload
        setResults((prev) => [...prev, data.result])
      })
      unlistenRefs.current.push(unlistenResult)

      const unlistenComplete = await listen<SearchComplete>('search:complete', (event) => {
        setCompletionInfo(event.payload)
        setIsSearching(false)
      })
      unlistenRefs.current.push(unlistenComplete)

      // Start the streaming search
      await invokeSearchStreaming(args)

    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setIsSearching(false)
    }
  }, [clearResults])

  return {
    results,
    progress,
    isSearching,
    error,
    completionInfo,
    startSearch,
    clearResults,
  }
}

export default useRealtimeSearch
