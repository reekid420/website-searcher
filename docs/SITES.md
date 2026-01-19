# Supported Sites

Website-searcher supports 13 game download sites with various search strategies.

## Site Overview

| # | Site | Domain | Search Type | Cloudflare | Notes |
|---|------|--------|-------------|------------|-------|
| 1 | steamgg | steamgg.net | Query Param | No | WordPress site |
| 2 | gog-games | gog-games.to | Query Param | No | GOG releases, cookies help |
| 3 | atopgames | atopgames.com | Query Param | No | WordPress site |
| 4 | elamigos | elamigos.site | Front Page | No | No search; filters homepage |
| 5 | fitgirl | fitgirl-repacks.site | Query Param | **Yes** | Popular repacks |
| 6 | dodi | dodi-repacks.download | Query Param | **Yes** | Repack releases |
| 7 | skidrowrepacks | skidrowrepacks.com | Query Param | No | Scene releases |
| 8 | steamrip | steamrip.com | Query Param | No | Pre-installed games |
| 9 | reloadedsteam | reloadedsteam.com | Query Param | No | Steam releases |
| 10 | ankergames | ankergames.net | Path Encoded | No | Falls back to listing |
| 11 | csrin | cs.rin.ru | phpBB Search | No* | Uses Playwright |
| 12 | nswpedia | nswpedia.com | Query Param | No | Nintendo Switch ROMs |
| 13 | f95zone | f95zone.to | Listing Page | No | Adult games forum |

*csrin bypasses Cloudflare via Playwright browser automation

## Search Types Explained

### Query Param
Standard URL query parameter search:
```
https://example.com/?s=query
```
Used by most WordPress-based sites.

### Front Page
No search functionality; parses homepage and filters locally:
```
https://elamigos.site/  → filter for "query"
```

### Path Encoded
Query embedded in URL path:
```
https://ankergames.net/search/elden%20ring
```

### Listing Page
Uses predefined listing URL and filters:
```
https://f95zone.to/forums/games.2/
```

### phpBB Search
Forum search with keywords parameter:
```
https://cs.rin.ru/forum/search.php?keywords=query&fid[]=10
```

## Site Details

### steamgg
- **URL**: https://steamgg.net/
- **Selector**: `h2.entry-title a`
- WordPress theme with entry titles

### gog-games
- **URL**: https://gog-games.to/
- **Selector**: `a.card, .games-list a`
- DRM-free GOG releases
- Cookies may improve results (no CF solver needed)

### atopgames
- **URL**: https://atopgames.com/
- **Selector**: `h2.entry-title a`
- WordPress theme

### elamigos
- **URL**: https://elamigos.site/
- **Selector**: `h2.entry-title a`
- Homepage only; results filtered client-side if they contain query

### fitgirl
- **URL**: https://fitgirl-repacks.site/
- **Selector**: `h2.entry-title a`
- **Requires**: FlareSolverr
- Popular repack creator, often Cloudflare-protected

### dodi
- **URL**: https://dodi-repacks.download/
- **Selector**: `h2.entry-title a`
- **Requires**: FlareSolverr
- Repack releases with Cloudflare protection

### skidrowrepacks
- **URL**: https://skidrowrepacks.com/
- **Selector**: `h2.entry-title a, article h2 a`
- Scene group releases

### steamrip
- **URL**: https://steamrip.com/
- **Selector**: `h2.entry-title a`
- Pre-installed Steam games

### reloadedsteam
- **URL**: https://reloadedsteam.com/
- **Selector**: `h2.entry-title a`
- Steam releases

### ankergames
- **URL**: https://ankergames.net/search/
- **Selector**: `div a[href^='/game/']`
- Uses path-encoded search with `%20` for spaces
- Falls back to listing page if search fails

### csrin (cs.rin.ru)
- **URL**: https://cs.rin.ru/forum/
- **Selector**: `a.topictitle, a[href^='viewtopic.php']`
- phpBB forum for Steam releases
- **Playwright**: Auto-enabled for JavaScript rendering
- **Environment**: `CSRIN_PAGES=N` controls result pages

### nswpedia
- **URL**: https://nswpedia.com/
- **Selector**: `h2 a, article h2 a`
- Nintendo Switch ROMs (WordPress)

### f95zone
- **URL**: https://f95zone.to/
- **Selector**: `a[href*='/threads/']`
- Adult games forum
- Search requires auth; uses listing page instead

## Filtering Sites

Specify sites with `--sites`:

```bash
# Single site
websearcher "elden ring" --sites fitgirl

# Multiple sites
websearcher "elden ring" --sites fitgirl,dodi,steamrip

# All sites (default)
websearcher "elden ring"
```

## Cloudflare Sites

Sites requiring FlareSolverr (CF bypass):

- `fitgirl`
- `dodi`

Start FlareSolverr:
```bash
docker run -d --name flaresolverr -p 8191:8191 \
  ghcr.io/flaresolverr/flaresolverr:latest
```

Or disable CF bypass:
```bash
websearcher "elden ring" --no-cf
```

## Adding a New Site

See [DEVELOPMENT.md](DEVELOPMENT.md) for instructions on adding new site configurations.
