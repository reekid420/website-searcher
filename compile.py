#!/usr/bin/env python3
"""
Cross-platform compile script for building the project.
Replaces compile.ps1 and compile.sh.

Usage: python compile.py [-v|--verbose] [-l|--log] [-d|--deb] [-r|--rpm] [-p|--pacman] [--nogui]

Options:
    -v, --verbose   Show full command output
    -l, --log       Enable logging to timestamped file (always shows in terminal too)
    -d, --deb       Force .deb bundle (Linux only)
    -r, --rpm       Force .rpm bundle (Linux only)
    -p, --pacman    Build Arch Linux .pkg.tar.zst package
    --nogui         Exclude GUI from Arch package (GUI included by default)
"""
from __future__ import annotations

import argparse
import glob
import io
import os
import platform
import re
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional

# ANSI color codes
GREEN = "\033[32m"
YELLOW = "\033[33m"
RED = "\033[31m"
RESET = "\033[0m"

# Global state
VERBOSE: bool = False
LOG_ENABLED: bool = False
LOG_FILE: Optional[Path] = None
LOG_HANDLE: Optional[io.TextIOWrapper] = None

ANSI_ESCAPE = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

def strip_ansi(text: str) -> str:
    """Remove ANSI escape codes from text."""
    return ANSI_ESCAPE.sub('', text)

def log_write(text: str) -> None:
    """Write to log file if enabled, stripping ANSI codes."""
    global LOG_HANDLE
    if LOG_HANDLE:
        LOG_HANDLE.write(strip_ansi(text))
        LOG_HANDLE.flush()

def print_and_log(text: str) -> None:
    """Print to terminal and optionally log."""
    print(text)
    log_write(text + "\n")

def step(name: str) -> None:
    """Print a step header."""
    msg = f"{GREEN}==> {name}{RESET}"
    print(msg)
    log_write(f"==> {name}\n")

def run_cmd(cmd, shell: bool = False, check: bool = True, 
            cwd: Optional[str] = None, quiet: bool = False) -> Optional[subprocess.CompletedProcess]:
    """
    Run a command with output handling.
    
    When VERBOSE or LOG_ENABLED: stream output in real-time
    When quiet and not VERBOSE: suppress stdout
    Always write to log file if LOG_ENABLED
    """
    global VERBOSE, LOG_ENABLED, LOG_HANDLE
    
    # Determine if we should show output
    show_output = VERBOSE or LOG_ENABLED
    
    try:
        if show_output:
            # Stream output line by line for real-time display and logging
            process = subprocess.Popen(
                cmd,
                shell=shell,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                cwd=cwd,
                encoding='utf-8',
                errors='replace',
                bufsize=1
            )
            
            for line in process.stdout:
                # Always print to terminal when verbose or logging
                print(line, end='')
                # Write to log file (strip ANSI codes)
                log_write(line)
            
            process.wait()
            
            if check and process.returncode != 0:
                raise subprocess.CalledProcessError(process.returncode, cmd)
            
            return subprocess.CompletedProcess(cmd, process.returncode)
        
        elif quiet:
            # Quiet mode: suppress stdout, but capture for potential logging
            result = subprocess.run(
                cmd,
                shell=shell,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
                cwd=cwd,
                encoding='utf-8',
                errors='replace',
                check=check
            )
            return result
        
        else:
            # Normal mode: let output flow through
            result = subprocess.run(
                cmd,
                shell=shell,
                text=True,
                cwd=cwd,
                encoding='utf-8',
                errors='replace',
                check=check
            )
            return result
    
    except subprocess.CalledProcessError as e:
        if check:
            raise
        return e
    except FileNotFoundError as e:
        print_and_log(f"{RED}Command not found: {cmd}{RESET}")
        if check:
            raise
        return None

def command_exists(name: str) -> bool:
    return shutil.which(name) is not None

def get_host_triple() -> str:
    """Get the Rust host triple."""
    try:
        result = subprocess.run(
            ["rustc", "-Vv"],
            capture_output=True,
            text=True,
            check=True
        )
        for line in result.stdout.splitlines():
            if line.startswith("host:"):
                return line.split(":")[1].strip()
    except Exception:
        pass
    
    # Fallback
    system = platform.system()
    if system == "Windows":
        return "x86_64-pc-windows-msvc"
    elif system == "Darwin":
        return "x86_64-apple-darwin"
    else:
        return "x86_64-unknown-linux-gnu"

def get_distro_info() -> dict:
    """Get Linux distribution info from /etc/os-release."""
    info_dict = {}
    os_release = Path("/etc/os-release")
    if os_release.exists():
        for line in os_release.read_text().splitlines():
            if "=" in line:
                key, _, value = line.partition("=")
                info_dict[key] = value.strip('"')
    return info_dict

def is_arch_based() -> bool:
    """Detect if running on Arch Linux or a derivative."""
    if platform.system() != "Linux":
        return False
    distro = get_distro_info()
    distro_id = distro.get("ID", "").lower()
    distro_like = distro.get("ID_LIKE", "").lower()
    
    arch_ids = {"arch", "manjaro", "endeavouros", "garuda", "artix", "arcolinux", "archcraft"}
    if distro_id in arch_ids:
        return True
    if "arch" in distro_like:
        return True
    return False

def generate_pkgbuild(version: str = "0.1.0", include_gui: bool = False) -> Path:
    """Generate PKGBUILD file for building Arch package."""
    # Use absolute paths since makepkg creates a src/ subdir
    project_root = Path.cwd().resolve()
    cli_binary_path = project_root / "target" / "release" / "website-searcher"
    gui_binary_path = project_root / "target" / "release" / "website-searcher-gui"
    license_path = project_root / "LICENSE"
    ws_script_path = project_root / "scripts" / "ws"
    
    # Build the package() function contents
    install_commands = f'''    # Install CLI binary
    install -Dm755 "{cli_binary_path}" "$pkgdir/usr/bin/website-searcher"
    ln -s website-searcher "$pkgdir/usr/bin/websearcher"
    
    # Install ws alias script
    if [ -f "{ws_script_path}" ]; then
        install -Dm755 "{ws_script_path}" "$pkgdir/usr/bin/ws"
    fi'''
    
    if include_gui:
        install_commands += f'''
    
    # Install GUI binary
    if [ -f "{gui_binary_path}" ]; then
        install -Dm755 "{gui_binary_path}" "$pkgdir/usr/bin/website-searcher-gui"
    fi'''
    
    install_commands += f'''
    
    # Install license
    if [ -f "{license_path}" ]; then
        install -Dm644 "{license_path}" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    fi'''
    
    provides = "'website-searcher' 'websearcher' 'ws'"
    if include_gui:
        provides += " 'website-searcher-gui'"
    
    pkgbuild_content = f'''# Maintainer: Auto-generated
pkgname=website-searcher
pkgver={version.replace('-', '_')}
pkgrel=1
pkgdesc="Cross-platform CLI that queries multiple game-download sites"
arch=('x86_64')
url="https://github.com/reekid420/website-searcher"
license=('MIT')
depends=('glibc' 'openssl' 'gtk3' 'webkit2gtk-4.1')
provides=({provides})
source=()

package() {{
{install_commands}
}}
'''
    pkg_dir = Path("pkg")
    pkg_dir.mkdir(exist_ok=True)
    pkgbuild_path = pkg_dir / "PKGBUILD"
    pkgbuild_path.write_text(pkgbuild_content)
    return pkgbuild_path

def build_pacman_package(include_gui: bool = False) -> None:
    """Build Arch Linux package using makepkg."""
    step("Build Arch Linux package")
    
    if not command_exists("makepkg"):
        print_and_log(f"{YELLOW}makepkg not found; skipping Arch package build{RESET}")
        return
    
    # Check that binary exists
    binary = Path("target/release/website-searcher")
    if not binary.exists():
        print_and_log(f"{RED}Release binary not found; build CLI first{RESET}")
        return
    
    # Generate PKGBUILD  
    pkgbuild = generate_pkgbuild(include_gui=include_gui)
    print_and_log(f"Generated: {pkgbuild}")
    if not include_gui:
        print_and_log("Excluding GUI binary from package (--nogui)")
    
    # Run makepkg
    try:
        run_cmd(["makepkg", "-sf", "--noconfirm"], cwd="pkg")
        
        # Find built package
        pkg_files = list(Path("pkg").glob("*.pkg.tar.zst"))
        if pkg_files:
            print_and_log(f"{GREEN}Arch package built: {pkg_files[0]}{RESET}")
            print_and_log(f"Install with: sudo pacman -U {pkg_files[0]}")
    except subprocess.CalledProcessError as e:
        print_and_log(f"{YELLOW}makepkg failed: {e}{RESET}")

def ensure_cargo_in_path() -> None:
    """Add cargo bin to PATH if not already present."""
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.exists():
        path_sep = ";" if platform.system() == "Windows" else ":"
        if str(cargo_bin) not in os.environ.get("PATH", ""):
            os.environ["PATH"] = f"{cargo_bin}{path_sep}{os.environ.get('PATH', '')}"

def cleanup_old_logs(max_logs: int = 3) -> None:
    """Keep only the N most recent log files, delete older ones."""
    log_pattern = "compile-script-*.log"
    log_files = sorted(glob.glob(log_pattern), key=os.path.getmtime, reverse=True)
    
    # Delete all but the most recent max_logs files
    for old_log in log_files[max_logs:]:
        try:
            os.remove(old_log)
            print_and_log(f"Removed old log: {old_log}")
        except OSError as e:
            print_and_log(f"{YELLOW}Warning: Could not remove {old_log}: {e}{RESET}")

def setup_logging() -> None:
    """Set up log file with UTF-8 encoding."""
    global LOG_FILE, LOG_HANDLE
    
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    LOG_FILE = Path(f"compile-script-{timestamp}.log")
    LOG_HANDLE = open(LOG_FILE, "w", encoding="utf-8")
    print_and_log(f"Logging to: {LOG_FILE}")

def close_logging() -> None:
    """Close log file handle."""
    global LOG_HANDLE
    if LOG_HANDLE:
        LOG_HANDLE.close()
        LOG_HANDLE = None

# ============================================================
# Build steps
# ============================================================

def cargo_q() -> str:
    """Return -q flag if not verbose."""
    return "" if VERBOSE else "-q"

def check_formatting() -> None:
    """Check and fix formatting."""
    result = subprocess.run(
        ["cargo", "fmt", "--all", "--", "--check"],
        capture_output=True,
        text=True
    )
    
    if result.returncode == 0:
        print_and_log("Formatting is correct")
    else:
        print_and_log("Formatting is incorrect, fixing...")
        run_cmd(["cargo", "fmt", "--all"])

def run_clippy() -> None:
    """Run clippy linter."""
    step("Clippy")
    q = cargo_q()
    cmd = ["cargo", "clippy", "--all-targets"]
    if q:
        cmd.insert(1, q)
    run_cmd(cmd)

def build_cli_release() -> None:
    """Build CLI in release mode."""
    step("Build CLI (release)")
    q = cargo_q()
    cmd = ["cargo", "build", "-p", "website-searcher", "--release"]
    if q:
        cmd.insert(1, q)
    run_cmd(cmd)

def build_cli_debug() -> None:
    """Build CLI in debug mode."""
    step("Build CLI (debug)")
    q = cargo_q()
    cmd = ["cargo", "build", "-p", "website-searcher"]
    if q:
        cmd.insert(1, q)
    run_cmd(cmd)

def build_workspace_release() -> None:
    """Build entire workspace in release mode."""
    step("Build workspace (release)")
    q = cargo_q()
    cmd = ["cargo", "build", "--workspace", "--release"]
    if q:
        cmd.insert(1, q)
    run_cmd(cmd)

def stage_tauri_sidecar() -> None:
    """Stage CLI binary as Tauri sidecar."""
    step("Stage Tauri sidecar")
    
    triple = get_host_triple()
    bin_dir = Path("src-tauri/bin")
    bin_dir.mkdir(parents=True, exist_ok=True)
    
    system = platform.system()
    if system == "Windows":
        src = Path("target/release/website-searcher.exe")
        dst = bin_dir / f"website-searcher-{triple}.exe"
    else:
        src = Path("target/release/website-searcher")
        dst = bin_dir / f"website-searcher-{triple}"
    
    if src.exists():
        shutil.copy2(src, dst)
        print_and_log(f"Staged sidecar: {dst}")
    else:
        print_and_log(f"{YELLOW}Warning: Source binary not found: {src}{RESET}")

def stage_wix_inputs() -> None:
    """Stage WiX inputs for Windows MSI build."""
    if platform.system() != "Windows":
        return
    
    step("Stage WiX inputs")
    
    ws_cmd = Path("scripts/ws.cmd")
    if not ws_cmd.exists():
        print_and_log(f"{YELLOW}Warning: Missing scripts/ws.cmd{RESET}")
        return
    
    wix_bin = Path("src-tauri/wix/bin")
    wix_bin.mkdir(parents=True, exist_ok=True)
    
    src_exe = Path("target/release/website-searcher.exe")
    if src_exe.exists():
        shutil.copy2(src_exe, wix_bin / "website-searcher.exe")
    shutil.copy2(ws_cmd, wix_bin / "ws.cmd")

def normalize_linux_scripts() -> None:
    """Normalize Linux maintainer scripts (remove CR, ensure shebang)."""
    if platform.system() == "Windows":
        return
    
    scripts = [
        "src-tauri/scripts/linux/postinst.sh",
        "src-tauri/scripts/linux/prerm.sh",
        "scripts/ws"
    ]
    
    for script_path in scripts:
        path = Path(script_path)
        if not path.exists():
            continue
        
        content = path.read_text()
        # Remove CR
        content = content.replace('\r', '')
        # Ensure shebang
        if not content.startswith('#!'):
            content = '#!/bin/sh\n' + content
        path.write_text(content)
        # Make executable
        path.chmod(path.stat().st_mode | 0o755)

def determine_linux_bundles(want_deb: bool, want_rpm: bool) -> str:
    """Determine which Linux bundles to build."""
    bundles = set(["appimage"])  # Always build AppImage
    
    # Auto-detect distro
    distro = get_distro_info()
    distro_id = distro.get("ID", "").lower()
    distro_like = distro.get("ID_LIKE", "").lower()
    detect_src = f"{distro_id} {distro_like}"
    
    if detect_src:
        print_and_log(f"Detected distro: {detect_src}")
    
    if re.search(r'debian|ubuntu', detect_src, re.I):
        bundles.add("deb")
    if re.search(r'rhel|fedora|suse', detect_src, re.I):
        bundles.add("rpm")
    
    # Manual overrides
    if want_deb:
        bundles.add("deb")
    if want_rpm:
        bundles.add("rpm")
    
    return ",".join(sorted(bundles))

def build_tauri(want_deb: bool = False, want_rpm: bool = False) -> None:
    """Build Tauri application."""
    step("Tauri build")
    
    # Check if tauri-cli is available
    try:
        subprocess.run(["cargo", "tauri", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_and_log(f"{YELLOW}cargo-tauri not found; skipping bundling.{RESET}")
        print_and_log("Install with: cargo install tauri-cli --locked")
        return
    
    system = platform.system()
    q = cargo_q()
    
    if system == "Windows":
        bundles = "msi"
    elif system == "Darwin":
        bundles = "dmg"
    else:  # Linux
        normalize_linux_scripts()
        bundles = determine_linux_bundles(want_deb, want_rpm)
    
    print_and_log(f"Tauri bundles to build: {bundles}")
    
    # Set environment for Linux AppImage
    if system == "Linux":
        os.environ["APPIMAGE_EXTRACT_AND_RUN"] = "1"
    
    cmd = ["cargo", "tauri", "build", "--bundles", bundles]
    if q:
        cmd.insert(1, q)
    
    try:
        run_cmd(cmd, cwd="src-tauri")
    except subprocess.CalledProcessError:
        print_and_log(f"{YELLOW}Tauri build failed{RESET}")
        if system == "Windows":
            print_and_log("Attempting manual WiX link...")
            manual_link_msi()

def manual_link_msi() -> None:
    """Manually link MSI if Tauri build fails on Windows."""
    if platform.system() != "Windows":
        return
    
    wix_root = Path(os.environ.get("LOCALAPPDATA", "")) / "tauri/WixTools314"
    if not wix_root.exists():
        print_and_log(f"{YELLOW}WiX tools not found at {wix_root}{RESET}")
        return
    
    cfg = "release"
    obj = Path(f"target/{cfg}/wix/x64/main.wixobj")
    wxs = Path("src-tauri/wix/main.wxs")
    out_dir = Path(f"target/{cfg}/bundle/msi")
    out_file = out_dir / "website-searcher_0.1.0_x64_en-US.msi"
    
    obj.parent.mkdir(parents=True, exist_ok=True)
    out_dir.mkdir(parents=True, exist_ok=True)
    
    if not wxs.exists():
        print_and_log(f"{YELLOW}WiX source not found: {wxs}{RESET}")
        return
    
    candle = wix_root / "candle.exe"
    light = wix_root / "light.exe"
    
    print_and_log(f"Manual WiX compile (candle) -> {obj}")
    result = run_cmd([
        str(candle), "-ext", "WixUIExtension", "-ext", "WixUtilExtension",
        "-arch", "x64", "-out", str(obj.parent) + "\\", str(wxs)
    ], cwd="src-tauri", check=False)
    
    if result and result.returncode != 0:
        print_and_log(f"{YELLOW}candle.exe failed{RESET}")
        return
    
    print_and_log(f"Manual WiX link (light) -> {out_file}")
    run_cmd([
        str(light), "-ext", "WixUIExtension", "-ext", "WixUtilExtension",
        str(obj), "-out", str(out_file)
    ], check=False)

# ============================================================
# Main
# ============================================================

def main() -> int:
    global VERBOSE, LOG_ENABLED
    
    parser = argparse.ArgumentParser(description="Build the project")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show full command output")
    parser.add_argument("-l", "--log", action="store_true", help="Enable logging to file")
    parser.add_argument("-d", "--deb", action="store_true", help="Force .deb bundle (Linux)")
    parser.add_argument("-r", "--rpm", action="store_true", help="Force .rpm bundle (Linux)")
    parser.add_argument("-p", "--pacman", action="store_true", help="Build Arch Linux .pkg.tar.zst package")
    parser.add_argument("--nogui", action="store_true", help="Exclude GUI from Arch package")
    args = parser.parse_args()
    
    VERBOSE = args.verbose
    LOG_ENABLED = args.log
    
    # Set up logging if enabled
    if LOG_ENABLED:
        setup_logging()
        # Clean up old logs (keep only 3)
        cleanup_old_logs(max_logs=3)
    
    try:
        # Ensure cargo is in PATH
        ensure_cargo_in_path()
        
        # Format check
        check_formatting()
        
        # Build CLI first (needed for sidecar)
        build_cli_release()
        
        # Stage sidecar and WiX inputs
        stage_wix_inputs()
        stage_tauri_sidecar()
        
        # Lint
        run_clippy()
        
        # Debug build (for development)
        build_cli_debug()
        
        # Build workspace
        build_workspace_release()
        
        # Tauri build
        build_tauri(want_deb=args.deb, want_rpm=args.rpm)
        
        # Build Arch package if requested or auto-detected
        if args.pacman or is_arch_based():
            build_pacman_package(include_gui=not args.nogui)
        
        print_and_log(f"\n{GREEN}Build complete!{RESET}")
        return 0
    
    except subprocess.CalledProcessError as e:
        print_and_log(f"\n{RED}Build failed: {e}{RESET}")
        return 1
    except KeyboardInterrupt:
        print_and_log(f"\n{YELLOW}Build interrupted{RESET}")
        return 130
    finally:
        close_logging()

if __name__ == "__main__":
    sys.exit(main())
