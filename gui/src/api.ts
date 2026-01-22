import { invoke } from '@tauri-apps/api/core'

export type SearchResult = {
  site: string
  title: string
  url: string
}

export type SearchArgs = {
  query: string
  limit?: number
  sites?: string[]
  debug?: boolean
  no_cf?: boolean
  cf_url?: string
  cookie?: string
  csrin_pages?: number
  csrin_search?: boolean
  no_playwright?: boolean
}

export async function invokeSearch(args: SearchArgs): Promise<SearchResult[]> {
  if (!args.query || !args.query.trim()) {
    throw new Error('Query is required')
  }
  return await invoke<SearchResult[]>('search_gui', { args })
}

export async function fetchSites(): Promise<string[]> {
  return await invoke<string[]>('list_sites')
}

// Cache types
export type CacheEntry = {
  query: string
  result_count: number
  timestamp: number
}

// Cache API functions
export async function getCache(): Promise<CacheEntry[]> {
  return await invoke<CacheEntry[]>('get_cache')
}

export async function getCachedResults(query: string): Promise<SearchResult[] | null> {
  return await invoke<SearchResult[] | null>('get_cached_results', { query })
}

export async function addToCache(query: string, results: SearchResult[]): Promise<void> {
  await invoke('add_to_cache', { query, results })
}

export async function removeCacheEntry(query: string): Promise<boolean> {
  return await invoke<boolean>('remove_cache_entry', { query })
}

export async function clearCache(): Promise<void> {
  await invoke('clear_cache')
}

export async function getCacheSettings(): Promise<number> {
  return await invoke<number>('get_cache_settings')
}

export async function setCacheSize(size: number): Promise<void> {
  await invoke('set_cache_size', { size })
}
