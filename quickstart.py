#!/usr/bin/env python3
"""
Cross-platform quickstart script to install build prerequisites.
Replaces quickstart.ps1 and quickstart.sh.

Usage: python quickstart.py [--install-coverage]
"""

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

# ANSI color codes
CYAN = "\033[36m"
YELLOW = "\033[33m"
RED = "\033[31m"
GREEN = "\033[32m"
RESET = "\033[0m"

def info(msg: str) -> None:
    print(f"{CYAN}[info]{RESET} {msg}")

def warn(msg: str) -> None:
    print(f"{YELLOW}[warn]{RESET} {msg}")

def err(msg: str) -> None:
    print(f"{RED}[err ]{RESET} {msg}")

def command_exists(name: str) -> bool:
    return shutil.which(name) is not None

def run_cmd(cmd: list[str], check: bool = False, capture: bool = False) -> subprocess.CompletedProcess:
    """Run a command, optionally checking for errors."""
    try:
        return subprocess.run(cmd, check=check, capture_output=capture, text=True)
    except subprocess.CalledProcessError as e:
        return e
    except FileNotFoundError:
        return None

def get_version(cmd: list[str]) -> str:
    """Get version string from a command."""
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        return result.stdout.strip() or result.stderr.strip()
    except Exception:
        return ""

def ensure_cargo_in_path() -> None:
    """Add cargo bin to PATH if not already present."""
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.exists():
        path_sep = ";" if platform.system() == "Windows" else ":"
        if str(cargo_bin) not in os.environ.get("PATH", ""):
            os.environ["PATH"] = f"{cargo_bin}{path_sep}{os.environ.get('PATH', '')}"
            info(f"Added {cargo_bin} to PATH for current session")

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

# ============================================================
# Installation functions
# ============================================================

def install_build_tools() -> None:
    """Install build tools on Linux."""
    if platform.system() != "Linux":
        return
    
    info("Ensuring build tools are installed")
    
    if command_exists("apt-get"):
        if not command_exists("cc") or not command_exists("pkg-config"):
            info("Installing build-essential and Tauri dependencies")
            subprocess.run([
                "sudo", "apt-get", "update", "-y"
            ], check=False)
            subprocess.run([
                "sudo", "apt-get", "install", "-y",
                "build-essential", "pkg-config", "libssl-dev",
                "libgtk-3-dev", "libwebkit2gtk-4.1-dev",
                "libayatana-appindicator3-dev", "librsvg2-dev"
            ], check=False)
    
    elif command_exists("dnf"):
        if not command_exists("cc") or not command_exists("pkg-config"):
            info("Installing development tools and Tauri dependencies")
            subprocess.run(["sudo", "dnf", "groupinstall", "-y", "Development Tools"], check=False)
            subprocess.run([
                "sudo", "dnf", "install", "-y",
                "pkg-config", "openssl-devel", "gtk3-devel",
                "webkit2gtk4.1-devel", "libappindicator-gtk3-devel", "librsvg2-devel"
            ], check=False)
    
    elif command_exists("pacman"):
        if not command_exists("cc") or not command_exists("pkg-config"):
            info("Installing base-devel and Tauri dependencies")
            subprocess.run([
                "sudo", "pacman", "-Sy", "--noconfirm",
                "base-devel", "pkg-config", "openssl",
                "gtk3", "webkit2gtk-4.1", "libappindicator-gtk3", "librsvg"
            ], check=False)

def install_node() -> None:
    """Install Node.js if not present."""
    if command_exists("node"):
        return
    
    info("Installing Node.js")
    system = platform.system()
    
    if system == "Windows":
        if command_exists("winget"):
            try:
                subprocess.run([
                    "winget", "install", "-e", "--id", "OpenJS.NodeJS.LTS",
                    "--accept-package-agreements", "--accept-source-agreements"
                ], check=True)
            except subprocess.CalledProcessError:
                warn("winget NodeJS install failed")
        else:
            warn("winget not found. Please install Node.js LTS from https://nodejs.org/ and re-run.")
    
    elif system == "Darwin":  # macOS
        if command_exists("brew"):
            subprocess.run(["brew", "install", "node"], check=False)
        else:
            warn("Homebrew not found. Please install Node.js from https://nodejs.org/")
    
    elif system == "Linux":
        if command_exists("brew"):
            subprocess.run(["brew", "install", "node"], check=False)
        elif command_exists("apt-get"):
            subprocess.run(["sudo", "apt-get", "update", "-y"], check=False)
            subprocess.run([
                "sudo", "apt-get", "install", "-y", "curl", "ca-certificates", "gnupg"
            ], check=False)
            # Use NodeSource setup script
            subprocess.run(
                "curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -",
                shell=True, check=False
            )
            subprocess.run(["sudo", "apt-get", "install", "-y", "nodejs"], check=False)
        elif command_exists("dnf"):
            subprocess.run(["sudo", "dnf", "module", "enable", "-y", "nodejs:20"], check=False)
            subprocess.run(["sudo", "dnf", "install", "-y", "nodejs"], check=False)
        elif command_exists("pacman"):
            subprocess.run(["sudo", "pacman", "-Sy", "--noconfirm", "nodejs", "npm"], check=False)
        else:
            warn("Please install Node.js LTS from https://nodejs.org/ and re-run.")

def activate_pnpm_via_corepack() -> None:
    """Activate pnpm via Corepack."""
    if not command_exists("node"):
        return
    if command_exists("pnpm"):
        return
    
    info("Activating pnpm via Corepack")
    try:
        if platform.system() != "Windows" and command_exists("sudo"):
            subprocess.run(["sudo", "corepack", "enable"], check=False)
        else:
            subprocess.run(["corepack", "enable"], check=False)
        subprocess.run(["corepack", "prepare", "pnpm@latest", "--activate"], check=False)
    except Exception as e:
        warn(f"Corepack activation failed: {e}")

def install_npm() -> None:
    """Ensure npm is available via Corepack."""
    if not command_exists("node"):
        return
    if command_exists("npm"):
        return
    
    info("Ensuring npm is available (via Corepack)")
    try:
        if platform.system() != "Windows" and command_exists("sudo"):
            subprocess.run(["sudo", "corepack", "enable"], check=False)
        else:
            subprocess.run(["corepack", "enable"], check=False)
    except Exception:
        warn("Could not enable npm via Corepack")

def install_rustup() -> None:
    """Install Rust via rustup if not present."""
    if command_exists("cargo"):
        return
    
    info("Installing Rust (rustup)")
    system = platform.system()
    
    if system == "Windows":
        if command_exists("winget"):
            try:
                subprocess.run([
                    "winget", "install", "-e", "--id", "Rustlang.Rustup",
                    "--accept-package-agreements", "--accept-source-agreements"
                ], check=True)
            except subprocess.CalledProcessError:
                warn("winget rustup install failed")
        
        # Fallback to direct download
        if not command_exists("cargo"):
            try:
                import urllib.request
                tmp = Path(os.environ.get("TEMP", "/tmp")) / "rustup-init.exe"
                info("Downloading rustup-init.exe...")
                urllib.request.urlretrieve("https://win.rustup.rs/x86_64", tmp)
                subprocess.run([str(tmp), "-y"], check=True)
            except Exception as e:
                err(f"Failed to install rustup: {e}. Install from https://rustup.rs")
    
    elif system == "Darwin":  # macOS
        if command_exists("brew"):
            subprocess.run(["brew", "install", "rustup-init"], check=False)
            subprocess.run(["rustup-init", "-y"], check=False)
        else:
            subprocess.run(
                "curl https://sh.rustup.rs -sSf | sh -s -- -y",
                shell=True, check=False
            )
    
    else:  # Linux
        subprocess.run(
            "curl https://sh.rustup.rs -sSf | sh -s -- -y",
            shell=True, check=False
        )
    
    # Source cargo env
    cargo_env = Path.home() / ".cargo" / "env"
    if cargo_env.exists() and platform.system() != "Windows":
        subprocess.run(f". {cargo_env}", shell=True, check=False)
    
    ensure_cargo_in_path()

def ensure_rust_components() -> None:
    """Ensure rustfmt and clippy are installed."""
    if not command_exists("cargo"):
        return
    
    info("Ensuring rustfmt and clippy components")
    subprocess.run(["rustup", "component", "add", "rustfmt"], check=False)
    subprocess.run(["rustup", "component", "add", "clippy"], check=False)

def ensure_cargo_tool(display: str, check_cmd: list[str], crate: str, args: str = "") -> None:
    """Install a cargo tool if not present."""
    # Check if tool exists
    try:
        subprocess.run(check_cmd, capture_output=True, check=True, timeout=10)
        return  # Tool exists
    except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired):
        pass  # Need to install
    
    if not command_exists("cargo"):
        warn(f"{display} requires Cargo; skipping (Cargo not found)")
        return
    
    info(f"Installing {display} ({crate})")
    try:
        cmd = ["cargo", "install", crate]
        if args:
            cmd.extend(args.split())
        subprocess.run(cmd, check=False)
    except Exception as e:
        warn(f"Failed to install {display}: {e}")

def print_versions() -> None:
    """Print installed tool versions."""
    print(f"\n{GREEN}==> Versions{RESET}")
    
    if command_exists("node"):
        print(f"node    : {get_version(['node', '-v'])}")
    if command_exists("npm"):
        print(f"npm     : {get_version(['npm', '-v'])}")
    if command_exists("pnpm"):
        print(f"pnpm    : {get_version(['pnpm', '-v'])}")
    if command_exists("cargo"):
        print(f"cargo   : {get_version(['cargo', '-V'])}")
    if command_exists("rustc"):
        print(f"rustc   : {get_version(['rustc', '-V'])}")
    
    # Cargo subcommands
    tauri_ver = get_version(["cargo", "tauri", "--version"])
    if tauri_ver:
        print(f"tauri   : {tauri_ver}")
    
    audit_ver = get_version(["cargo", "audit", "-V"])
    if audit_ver:
        print(f"audit   : {audit_ver}")
    
    nextest_ver = get_version(["cargo", "nextest", "--version"])
    if nextest_ver:
        print(f"nextest : {nextest_ver}")
    
    llvmcov_ver = get_version(["cargo", "llvm-cov", "--version"])
    if llvmcov_ver:
        print(f"llvm-cov: {llvmcov_ver}")

def main() -> int:
    print(f"{GREEN}==> Quickstart: installing prerequisites{RESET}")
    
    # Linux build tools
    install_build_tools()
    
    # Node.js ecosystem
    install_node()
    activate_pnpm_via_corepack()
    install_npm()
    
    # Rust ecosystem
    install_rustup()
    ensure_cargo_in_path()
    ensure_rust_components()
    
    # Cargo tools
    ensure_cargo_tool("Tauri CLI", ["cargo", "tauri", "--version"], "tauri-cli", "--locked")
    ensure_cargo_tool("cargo-audit", ["cargo", "audit", "-V"], "cargo-audit")
    ensure_cargo_tool("cargo-nextest", ["cargo", "nextest", "--version"], "cargo-nextest")
    ensure_cargo_tool("cargo-llvm-cov", ["cargo", "llvm-cov", "--version"], "cargo-llvm-cov")
    
    # Report versions
    print_versions()
    
    print(f"\n{CYAN}[info]{RESET} Quickstart complete. Restart your terminal and run: python compile.py")
    return 0

if __name__ == "__main__":
    sys.exit(main())
