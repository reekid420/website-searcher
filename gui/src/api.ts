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


