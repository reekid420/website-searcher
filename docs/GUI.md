# GUI Application

The website-searcher GUI provides a graphical interface for searching game download sites, built with Tauri 2.x and React.

## Launching the GUI

### From CLI

```bash
# Using the ws alias
ws --gui

# Or directly
website-searcher-gui
```

### Development Mode

```bash
# Start Tauri dev server (hot reload)
cd src-tauri
cargo tauri dev
```

## Interface Overview

### Main Window

- **Search Bar**: Enter game name to search
- **Site Selection**: Choose which sites to search
- **Results List**: Grouped by site, clickable entries
- **Status Bar**: Shows search progress and errors

### Window Properties

| Property     | Value     |
| ------------ | --------- |
| Default Size | 800 x 600 |
| Theme        | Dark      |
| Resizable    | Yes       |

## Features

### Search

1. Enter a game name in the search bar
2. Select sites (or leave on "All" for all sites)
3. Press Enter or click Search
4. Results appear grouped by site, sorted alphabetically

### Multi-Query Syntax

Use pipe (`|`) separator for different searches on different sites:

```
elden ring site:fitgirl | minecraft site:csrin
```

- Each segment is parsed independently
- Sites not mentioned in any segment search ALL segments without site restrictions

### Site Selection

- **Invert Selection**: Click "⇆ Invert" button to flip site selection
  - If no sites selected: selects all sites
  - If some sites selected: inverts the selection (selects unselected, deselects selected)
- **Empty selection**: Searches all sites by default

### Results Display

- Results grouped by site (sorted A→Z by site name)
- Items within each group sorted A→Z by title
- Click any URL to copy it to clipboard

### Link Handling

When you click a result link:

- URL is **copied to clipboard**
- "Copied!" animation displays
- Link opens in default browser

### Keyboard Shortcuts

| Key      | Action                       |
| -------- | ---------------------------- |
| `Enter`  | Execute search               |
| `Ctrl+V` | Paste into search            |
| `Esc`    | Clear search / Close dialogs |

### Search Caching

The GUI caches recent searches for instant retrieval:

- **Recent Searches**: Click a recent search pill to reload cached results instantly
- **Cache Hit Indicator**: Green banner shows when results are loaded from cache
- **Settings Panel**: Click ⚙️ Settings to:
  - Adjust cache size (3-20 searches)
  - Clear all cached searches
- **Persistence**: Cache persists across sessions via localStorage

### Logging Controls

Control log output with toggles in the search options:

| Toggle    | Effect                                              |
| --------- | --------------------------------------------------- |
| `verbose` | Shows info-level logs in console                    |
| `debug`   | Shows debug-level logs (more detailed than verbose) |

**Environment variable override:**

```bash
# Set log level before launching GUI
LOG_LEVEL=debug website-searcher-gui
```

Valid values: `error` (default in release), `warn`, `info`/`verbose`, `debug`

## Configuration

The GUI is configured in `src-tauri/tauri.conf.json`:

```json
{
  "app": {
    "windows": [
      {
        "title": "website-searcher",
        "width": 800,
        "height": 600,
        "theme": "Dark"
      }
    ]
  }
}
```

## Tauri Commands

The GUI communicates with the backend via Tauri IPC commands defined in `src-tauri/src/lib.rs`:

| Command     | Description                                |
| ----------- | ------------------------------------------ |
| `search`    | Execute search with query and site filters |
| `get_sites` | List available sites                       |

## Frontend Stack

| Technology | Version | Purpose       |
| ---------- | ------- | ------------- |
| React      | 19.x    | UI framework  |
| TypeScript | 5.8     | Type safety   |
| Vite       | 7.x     | Build tool    |
| Tauri API  | 2.x     | Native bridge |

## Building the GUI

```bash
# Development (with hot reload)
cargo tauri dev

# Production build
python compile.py

# The GUI binary is at:
# Windows: target/release/website-searcher-gui.exe
# Linux/macOS: target/release/website-searcher-gui
```

## Bundled Packages

GUI is included in platform packages:

| Platform | Package     | GUI Included           |
| -------- | ----------- | ---------------------- |
| Windows  | MSI         | Yes                    |
| macOS    | DMG         | Yes                    |
| Linux    | AppImage    | Yes                    |
| Linux    | deb/rpm     | Yes                    |
| Arch     | pkg.tar.zst | Yes (unless `--nogui`) |

## Troubleshooting

### GUI won't start

1. Ensure WebKit is installed (Linux):

   ```bash
   # Debian/Ubuntu
   sudo apt install libwebkit2gtk-4.1-0

   # Arch
   sudo pacman -S webkit2gtk-4.1
   ```

2. Check for errors in terminal:
   ```bash
   website-searcher-gui 2>&1 | head -50
   ```

### Search returns no results

1. Try `--debug` mode in CLI to verify sites work:

   ```bash
   websearcher "test" --debug --format table
   ```

2. Check if FlareSolverr is needed for the site

### Slow performance

- GUI starts FlareSolverr requests which may be slower
- Consider using CLI for bulk searches
