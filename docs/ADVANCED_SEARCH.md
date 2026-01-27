# Advanced Search Operators

Website Searcher supports powerful advanced search operators that allow you to refine and filter your search results. These operators work across all interfaces: **CLI**, **TUI**, and **GUI**.

## Table of Contents

- [Overview](#overview)
- [Available Operators](#available-operators)
  - [Site Restriction (`site:`)](#site-restriction-site)
  - [Exclusion (`-term`)](#exclusion--term)
  - [Exact Phrase (`"phrase"`)](#exact-phrase-phrase)
  - [Regex Pattern (`regex:`)](#regex-pattern-regex)
  - [Multi-Query Separator (`|`)](#multi-query-separator-)
- [Using Operators in CLI](#using-operators-in-cli)
- [Using Operators in TUI](#using-operators-in-tui)
- [Using Operators in GUI](#using-operators-in-gui)
- [Combining Operators](#combining-operators)
- [Examples](#examples)
- [Technical Details](#technical-details)

---

## Overview

Advanced search operators allow you to:

- **Filter by specific sites** - Only search certain game repack sites
- **Exclude unwanted terms** - Remove results containing specific words
- **Match exact phrases** - Require results to contain exact word sequences
- **Use regex patterns** - Advanced pattern matching for power users

All operators are **case-insensitive** and work by filtering results after they're fetched from the sites.

---

## Available Operators

### Site Restriction (`site:`)

**Syntax:** `site:sitename` or `site:site1,site2,site3`

Restricts search results to only include results from the specified site(s). You can use multiple `site:` operators or comma-separated site names to search across several specific sites.

**Supported site names:**

- `fitgirl`
- `dodi`
- `gog-games`
- `steamrip`
- `freegog`
- `csrin` (cs.rin.ru)
- `f95zone`
- `nswpedia`
- `ankergames`

**Examples:**

```
elden ring site:fitgirl
cyberpunk site:dodi site:fitgirl
baldurs gate 3 site:gog-games
minecraft site:fitgirl,dodi,elamigos
```

**How it works:**

- The operator performs a case-insensitive substring match on the site name
- `site:fitgirl` will match results where the site field contains "fitgirl"
- Multiple `site:` operators create an OR condition (match any of the specified sites)
- Comma-separated sites work the same as multiple `site:` operators

---

### Exclusion (`-term`)

**Syntax:** `-term` or `-"multiple words"`

Excludes results that contain the specified term in either the title or URL. Use this to filter out unwanted editions, versions, or content.

**Examples:**

```
elden ring -deluxe
cyberpunk -gog -dlc
witcher 3 -"game of the year"
```

**How it works:**

- Performs case-insensitive matching on both title and URL
- Any result containing the excluded term is filtered out
- Multiple exclusions are combined with AND logic (all must be absent)

**Common use cases:**

- Remove specific editions: `-deluxe`, `-ultimate`, `-goty`
- Filter out DLC: `-dlc`, `-expansion`
- Exclude platforms: `-gog`, `-steam`
- Remove languages: `-russian`, `-chinese`

---

### Exact Phrase (`"phrase"`)

**Syntax:** `"exact phrase here"`

Requires results to contain the exact phrase (word sequence) in either the title or URL. This is useful when you want results with a specific multi-word term.

**Examples:**

```
"elden ring"
"shadow of the erdtree"
"baldurs gate 3"
```

**How it works:**

- Searches for the exact sequence of words (case-insensitive)
- Must appear in title or URL
- You can use multiple quoted phrases - all must be present

**Difference from regular search:**

- Regular: `elden ring` matches "ring of elden" or "elden's ring"
- Exact: `"elden ring"` only matches the exact phrase "elden ring"

---

### Regex Pattern (`regex:`)

**Syntax:** `regex:pattern`

Advanced pattern matching using regular expressions. This is for power users who need complex matching logic.

**Examples:**

```
game regex:v[0-9]+
cyberpunk regex:v[0-9]+\.[0-9]+
elden regex:(dlc|expansion|shadow)
```

**How it works:**

- Uses Rust regex syntax
- Matches against both title and URL
- Invalid regex patterns are silently ignored
- Case-sensitive by default (use `(?i)` for case-insensitive)

**Common patterns:**

- Version numbers: `regex:v[0-9]+\.[0-9]+`
- Alternatives: `regex:(goty|complete|ultimate)`
- Year matching: `regex:20[0-9]{2}`
- Build numbers: `regex:build[0-9]+`

**Note:** Regex is an advanced feature. If you're not familiar with regular expressions, stick to the other operators.

---

### Multi-Query Separator (`|`)

**Syntax:** `query1 | query2 | query3`

Allows searching for different games on different sites simultaneously. Each segment separated by `|` is parsed independently with its own operators.

**Examples:**

```
elden ring site:fitgirl | minecraft site:csrin
elden ring -nightreign site:fitgirl,dodi | minecraft site:elamigos,csrin
elden ring site:fitgirl | minecraft site:csrin | cyberpunk
```

**How it works:**

1. The query is split on `|` into segments
2. Each segment is parsed independently (can have its own `site:`, `-`, `"phrase"`, `regex:`)
3. For each site:
   - If the site is mentioned in any segment's `site:` restriction, only those segments apply
   - Segments without `site:` restrictions apply to ALL sites
4. Results are filtered per-site based on applicable segments

**Behavior:**

```
elden ring site:fitgirl | minecraft site:csrin | cyberpunk
```

| Site        | Searches For                     |
| ----------- | -------------------------------- |
| fitgirl     | "elden ring" AND "cyberpunk"     |
| csrin       | "minecraft" AND "cyberpunk"      |
| Other sites | "cyberpunk" only                 |

**Key points:**

- Segments WITH `site:` restrictions apply ONLY to those sites
- Segments WITHOUT `site:` restrictions apply to ALL sites
- This prevents "query leakage" between unrelated searches

**Use cases:**

- Searching for different games on different sites simultaneously
- Comparing availability of multiple games across specific repackers
- Building complex search workflows in a single query

---

## Using Operators in CLI

### Basic Usage

Simply include the operators in your search query:

```bash
# Search for Elden Ring on FitGirl only
websearcher "elden ring site:fitgirl"

# Exclude deluxe editions
websearcher "cyberpunk -deluxe"

# Exact phrase match
websearcher '"shadow of the erdtree"'

# Combine multiple operators
websearcher "elden ring site:fitgirl -deluxe"
```

### Getting Help

View the operator help text:

```bash
websearcher --help-operators
```

This displays:

```
Advanced Query Operators:
  site:name     Restrict to specific site (e.g., site:fitgirl)
  -term         Exclude results containing term (e.g., -deluxe)
  "phrase"      Require exact phrase match (e.g., "elden ring")
  regex:pattern Match using regex (e.g., regex:v[0-9]+)

Examples:
  elden ring site:fitgirl
  elden ring -deluxe -edition
  "elden ring" site:dodi
  cyberpunk regex:v[0-9]+\.[0-9]+
```

### Shell Quoting

When using operators in the CLI, be mindful of shell quoting:

**PowerShell (Windows):**

```powershell
# Use double quotes for the entire query
websearcher "elden ring site:fitgirl"

# Escape inner quotes for exact phrases
websearcher "cyberpunk `"shadow of the erdtree`" site:dodi"

# Or use single quotes
websearcher 'elden ring site:fitgirl -deluxe'
```

**Bash/Zsh (Linux/macOS):**

```bash
# Use single quotes to avoid shell expansion
websearcher 'elden ring site:fitgirl'

# Or escape special characters
websearcher "elden ring site:fitgirl"

# For exact phrases, use escaped quotes
websearcher '"elden ring" site:fitgirl'
```

---

## Using Operators in TUI

The TUI (Terminal User Interface) is the interactive mode that appears when you run `websearcher` without arguments.

### Entering Queries

1. Run `websearcher` (no arguments)
2. At the "Search phrase:" prompt, enter your query with operators:

```
Search phrase: elden ring site:fitgirl -deluxe
```

3. The TUI will parse the operators and apply filters automatically

### Interactive Features

- **Recent searches** are displayed with their operator syntax preserved
- **Real-time progress** shows which sites are being searched
- **Filtered results** appear as they arrive from each site

### Example Session

```
Website Searcher (interactive)

Recent searches:
  1. elden ring site:fitgirl (5 results)
  2. cyberpunk -deluxe (12 results)
  3. "baldurs gate 3" (8 results)

Search phrase: elden ring site:fitgirl -deluxe

⏳ Searching 1 sites: fitgirl
✅ 1/1 sites | fitgirl 3 results
```

---

## Using Operators in GUI

The GUI supports all advanced search operators through the main search input field.

### Entering Queries

1. Launch the GUI application
2. In the search input field, type your query with operators:

```
elden ring site:fitgirl -deluxe
```

3. Click "Search" or press Enter
4. Results are automatically filtered based on your operators

### Visual Feedback

- **Real-time streaming**: When enabled, you'll see per-site progress indicators
- **Cache indicators**: Shows when results are loaded from cache (operators are preserved)
- **Site badges**: Each result card shows which site it came from

### GUI-Specific Features

**Recent Searches:**

- Recent searches are shown as clickable pills below the search box
- Clicking a recent search loads it with all operators intact
- Operators are preserved in the cache

**Site Selection:**

- The GUI has a site checkbox list on the left
- Using `site:` operators **overrides** the checkbox selection
- If you use `site:fitgirl` in your query, only FitGirl will be searched regardless of checkboxes

**Settings Panel:**

- Cache size can be adjusted (affects all interfaces)
- Cache is shared between GUI, CLI, and TUI
- Operators are preserved when caching results

### Example Workflow

1. **Simple search:**

   ```
   elden ring
   ```

   → Searches all sites

2. **Refine with site restriction:**

   ```
   elden ring site:fitgirl
   ```

   → Only FitGirl results

3. **Exclude unwanted editions:**

   ```
   elden ring site:fitgirl -deluxe -ultimate
   ```

   → FitGirl results without deluxe/ultimate editions

4. **Add exact phrase:**
   ```
   "elden ring" site:fitgirl -deluxe
   ```
   → Exact phrase match, FitGirl only, no deluxe

---

## Combining Operators

You can combine multiple operators in a single query for powerful filtering:

### Multiple Site Restrictions

```
elden ring site:fitgirl site:dodi
```

→ Results from FitGirl **OR** Dodi (either site)

### Multiple Exclusions

```
cyberpunk -deluxe -gog -dlc
```

→ Results must **NOT** contain deluxe, gog, or dlc

### Multiple Exact Phrases

```
"elden ring" "shadow of the erdtree"
```

→ Results must contain **BOTH** exact phrases

### Complex Combinations

```
"elden ring" site:fitgirl site:dodi -deluxe -ultimate
```

→ Exact phrase "elden ring" from FitGirl or Dodi, excluding deluxe and ultimate editions

```
cyberpunk site:gog-games -dlc regex:v[0-9]+\.[0-9]+
```

→ Cyberpunk from GOG-Games, no DLC, must have version number pattern

---

## Examples

### Scenario 1: Finding a Specific Game Repack

**Goal:** Find Elden Ring from FitGirl, but not deluxe editions

**CLI:**

```bash
websearcher "elden ring site:fitgirl -deluxe"
```

**TUI:**

```
Search phrase: elden ring site:fitgirl -deluxe
```

**GUI:**

```
elden ring site:fitgirl -deluxe
```

---

### Scenario 2: Comparing Multiple Repackers

**Goal:** See Cyberpunk 2077 from both FitGirl and Dodi

**CLI:**

```bash
websearcher "cyberpunk 2077 site:fitgirl site:dodi"
```

**TUI:**

```
Search phrase: cyberpunk 2077 site:fitgirl site:dodi
```

**GUI:**

```
cyberpunk 2077 site:fitgirl site:dodi
```

---

### Scenario 3: Excluding Multiple Terms

**Goal:** Find Baldur's Gate 3, but exclude GOG and DLC versions

**CLI:**

```bash
websearcher "baldurs gate 3 -gog -dlc"
```

**TUI:**

```
Search phrase: baldurs gate 3 -gog -dlc
```

**GUI:**

```
baldurs gate 3 -gog -dlc
```

---

### Scenario 4: Exact Phrase with Exclusions

**Goal:** Find exact "Shadow of the Erdtree" DLC, excluding certain sites

**CLI:**

```bash
websearcher '"shadow of the erdtree" -steamrip'
```

**TUI:**

```
Search phrase: "shadow of the erdtree" -steamrip
```

**GUI:**

```
"shadow of the erdtree" -steamrip
```

---

### Scenario 5: Version-Specific Search

**Goal:** Find Cyberpunk with version numbers (v1.5, v2.0, etc.)

**CLI:**

```bash
websearcher "cyberpunk regex:v[0-9]+\.[0-9]+"
```

**TUI:**

```
Search phrase: cyberpunk regex:v[0-9]+\.[0-9]+
```

**GUI:**

```
cyberpunk regex:v[0-9]+\.[0-9]+
```

---

### Scenario 6: Multi-Query for Different Games

**Goal:** Search for Elden Ring on FitGirl and Minecraft on csrin simultaneously

**CLI:**

```bash
websearcher "elden ring site:fitgirl | minecraft site:csrin"
```

**TUI:**

```
Search phrase: elden ring site:fitgirl | minecraft site:csrin
```

**GUI:**

```
elden ring site:fitgirl | minecraft site:csrin
```

**Result:**

- FitGirl is searched for "elden ring"
- csrin is searched for "minecraft"
- Other sites are not searched (all queries have explicit `site:` restrictions)

---

### Scenario 7: Multi-Query with Shared Search

**Goal:** Search for Elden Ring on FitGirl, Minecraft on csrin, and Cyberpunk on ALL sites

**CLI:**

```bash
websearcher "elden ring site:fitgirl | minecraft site:csrin | cyberpunk"
```

**TUI:**

```
Search phrase: elden ring site:fitgirl | minecraft site:csrin | cyberpunk
```

**GUI:**

```
elden ring site:fitgirl | minecraft site:csrin | cyberpunk
```

**Result:**

- FitGirl is searched for "elden ring" AND "cyberpunk"
- csrin is searched for "minecraft" AND "cyberpunk"
- All other sites are searched for "cyberpunk" only

---

## Technical Details

### Operator Parsing

The query parser processes operators in this order:

1. **Extract quoted phrases** - Removes `"exact phrases"` from the query
2. **Parse remaining tokens** - Splits by whitespace
3. **Identify operators** - Detects `site:`, `-`, and `regex:` prefixes
4. **Extract search terms** - Remaining words become search terms

### Filtering Logic

Operators are applied **after** fetching results from sites:

1. **Fetch** - Query is sent to selected sites (without operator syntax)
2. **Parse** - Results are extracted from HTML
3. **Filter** - Operators are applied to filter the results
4. **Deduplicate** - Duplicate results are removed
5. **Return** - Filtered results are displayed

### Performance Considerations

- **Site restrictions** (`site:`) can improve performance by reducing the number of sites queried
- **Other operators** don't affect fetch performance (filtering happens after)
- **Regex patterns** are the most computationally expensive (use sparingly)

### Caching Behavior

- **Cache key** is the normalized query (lowercased, whitespace normalized)
- **Operators are preserved** in cached results
- **Filtering is re-applied** when loading from cache
- **Cache is shared** across CLI, TUI, and GUI

### Case Sensitivity

- **Site names**: Case-insensitive (`site:FitGirl` = `site:fitgirl`)
- **Exclusions**: Case-insensitive (`-Deluxe` = `-deluxe`)
- **Exact phrases**: Case-insensitive (`"Elden Ring"` = `"elden ring"`)
- **Regex**: Case-sensitive by default (use `(?i)` flag for case-insensitive)

### Operator Precedence

All operators have equal precedence and are combined with AND logic:

```
"elden ring" site:fitgirl -deluxe
```

Means:

- Must contain exact phrase "elden ring" **AND**
- Must be from fitgirl site **AND**
- Must NOT contain "deluxe"

---

## Troubleshooting

### No Results with Operators

If you're getting no results when using operators:

1. **Check spelling** - Site names must match exactly (fitgirl, dodi, etc.)
2. **Try without operators** - Verify results exist without filtering
3. **Simplify query** - Remove operators one by one to find the issue
4. **Check regex syntax** - Invalid regex patterns are silently ignored

### Unexpected Results

If results don't match your expectations:

1. **Verify operator syntax** - Ensure no typos (e.g., `site:` not `sites:`)
2. **Check for spaces** - `site: fitgirl` won't work (no space after colon)
3. **Quote exact phrases** - Use `"exact phrase"` not `exact phrase`
4. **Test in CLI with --debug** - See what's being filtered

### Shell Issues (CLI)

If operators aren't working in CLI:

1. **Quote your query** - Use `"query site:fitgirl"` not `query site:fitgirl`
2. **Escape special characters** - Some shells need escaping for `-` or `"`
3. **Use single quotes** - In bash/zsh, single quotes prevent expansion

---

## Best Practices

1. **Start broad, then narrow** - Begin with a simple query, then add operators
2. **Use site restrictions first** - Reduces fetch time and network usage
3. **Combine exclusions** - `-deluxe -gog -ultimate` is more effective than separate searches
4. **Cache your searches** - Operators are preserved in cache for quick re-runs
5. **Test regex patterns** - Use a regex tester before using in searches
6. **Use exact phrases sparingly** - They can be too restrictive

---

## Summary

Advanced search operators give you fine-grained control over your search results:

| Operator   | Purpose                           | Example                              |
| ---------- | --------------------------------- | ------------------------------------ |
| `site:`    | Restrict to specific site(s)      | `site:fitgirl`                       |
| `-term`    | Exclude results with term         | `-deluxe`                            |
| `"phrase"` | Require exact phrase              | `"elden ring"`                       |
| `regex:`   | Advanced pattern matching         | `regex:v[0-9]+`                      |
| `\|`       | Separate multiple query segments  | `game1 site:a \| game2 site:b`       |

These operators work identically across **CLI**, **TUI**, and **GUI**, making it easy to refine your searches regardless of which interface you prefer.

For more information, see:

- [CLI Documentation](./CLI.md)
- [GUI Documentation](./GUI.md)
- [API Documentation](./API.md)
