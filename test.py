#!/usr/bin/env python3
"""
Cross-platform test script for running project tests.
Assumes the project is already compiled.

Usage: python test.py [OPTIONS]

Options:
    -r, --rust      Run Rust tests only
    -g, --gui       Run GUI tests only
    -e, --e2e       Run E2E tests only
    -c, --clippy    Run Clippy linter only
    --coverage      Generate coverage reports
    -v, --verbose   Show full output (also logs everything to file with -l)
    -l, --log       Enable logging to timestamped file
    -a, --audit     Run cargo audit
    --all           Run all tests (default if no specific option given)
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
CYAN = "\033[36m"
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


def info(msg: str) -> None:
    """Print info message."""
    text = f"{CYAN}    {msg}{RESET}"
    print(text)
    log_write(f"    {msg}\n")


def warn(msg: str) -> None:
    """Print warning message."""
    text = f"{YELLOW}    Warning: {msg}{RESET}"
    print(text)
    log_write(f"    Warning: {msg}\n")


def error(msg: str) -> None:
    """Print error message."""
    text = f"{RED}    Error: {msg}{RESET}"
    print(text)
    log_write(f"    Error: {msg}\n")


def success(msg: str) -> None:
    """Print success message."""
    text = f"{GREEN}    ✓ {msg}{RESET}"
    print(text)
    log_write(f"    ✓ {msg}\n")


def run_cmd(cmd, shell: bool = False, check: bool = True,
            cwd: Optional[str] = None, quiet: bool = False) -> Optional[subprocess.CompletedProcess]:
    """
    Run a command with output handling.
    
    When VERBOSE: stream output to terminal AND log file
    When quiet and not VERBOSE: suppress stdout
    """
    global VERBOSE, LOG_HANDLE
    
    try:
        if VERBOSE or not quiet:
            # Stream output in real-time
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
                try:
                    print(line, end='')
                except UnicodeEncodeError:
                    # Strip non-ASCII for Windows cp1252 compatibility
                    print(line.encode('ascii', 'replace').decode(), end='')
                # Log to file when verbose (captures all command output)
                if VERBOSE:
                    log_write(line)
            
            process.wait()
            
            if check and process.returncode != 0:
                raise subprocess.CalledProcessError(process.returncode, cmd)
            
            return subprocess.CompletedProcess(cmd, process.returncode)
        
        else:
            # Quiet mode
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
    
    except subprocess.CalledProcessError as e:
        if check:
            raise
        return e
    except FileNotFoundError as e:
        error(f"Command not found: {cmd}")
        if check:
            raise
        return None


def command_exists(name: str) -> bool:
    """Check if a command exists in PATH."""
    return shutil.which(name) is not None


def ensure_cargo_in_path() -> None:
    """Add cargo bin to PATH if not already present."""
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.exists():
        path_sep = ";" if platform.system() == "Windows" else ":"
        if str(cargo_bin) not in os.environ.get("PATH", ""):
            os.environ["PATH"] = f"{cargo_bin}{path_sep}{os.environ.get('PATH', '')}"


def cleanup_old_logs(max_logs: int = 3) -> None:
    """Keep only the N most recent log files, delete older ones."""
    log_pattern = "test-script-*.log"
    log_files = sorted(glob.glob(log_pattern), key=os.path.getmtime, reverse=True)
    
    # Delete all but the most recent max_logs files
    for old_log in log_files[max_logs:]:
        try:
            os.remove(old_log)
        except OSError as e:
            print_and_log(f"{YELLOW}Warning: Could not remove {old_log}: {e}{RESET}")

def setup_logging() -> None:
    """Set up log file with UTF-8 encoding."""
    global LOG_FILE, LOG_HANDLE
    
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    LOG_FILE = Path(f"test-script-{timestamp}.log")
    LOG_HANDLE = open(LOG_FILE, "w", encoding="utf-8")
    print_and_log(f"Logging to: {LOG_FILE}")

def close_logging() -> None:
    """Close log file handle."""
    global LOG_HANDLE
    if LOG_HANDLE:
        LOG_HANDLE.close()
        LOG_HANDLE = None


# ============================================================
# Test Functions
# ============================================================

def run_rust_tests(coverage: bool = False) -> bool:
    """Run Rust tests with optional coverage."""
    step("Running Rust Tests")
    
    ensure_cargo_in_path()
    
    if coverage:
        if command_exists("cargo-llvm-cov"):
            info("Generating coverage report with cargo-llvm-cov...")
            try:
                # Use --test-threads=1 to prevent race conditions with env var tests
                run_cmd(["cargo", "llvm-cov", "--workspace", "--html", "--", "--test-threads=1"])
                success("Coverage report generated at target/llvm-cov/html/index.html")
                return True
            except subprocess.CalledProcessError:
                error("Coverage generation failed")
                return False
        else:
            warn("cargo-llvm-cov not installed. Install with: cargo install cargo-llvm-cov")
            warn("Falling back to standard tests...")
    
    # Use nextest if available
    if command_exists("cargo-nextest"):
        info("Using cargo-nextest for parallel test execution...")
        try:
            cmd = ["cargo", "nextest", "run", "--workspace"]
            if not VERBOSE:
                cmd.insert(1, "-q")
            run_cmd(cmd)
            success("All Rust tests passed")
            return True
        except subprocess.CalledProcessError:
            error("Rust tests failed")
            return False
    else:
        info("Using cargo test (install cargo-nextest for faster parallel tests)")
        try:
            # Use --test-threads=1 to prevent race conditions with env var tests
            cmd = ["cargo", "test", "--workspace", "--", "--test-threads=1"]
            if not VERBOSE:
                cmd.insert(1, "-q")
            run_cmd(cmd)
            success("All Rust tests passed")
            return True
        except subprocess.CalledProcessError:
            error("Rust tests failed")
            return False


def run_gui_tests(coverage: bool = False) -> bool:
    """Run GUI unit tests with Vitest."""
    step("Running GUI Tests")
    
    gui_dir = Path("gui")
    if not gui_dir.exists():
        error("GUI directory not found")
        return False
    
    # Check if node_modules exists
    node_modules = gui_dir / "node_modules"
    if not node_modules.exists():
        info("Installing pnpm dependencies...")
        try:
            run_cmd(["pnpm", "install"], cwd=str(gui_dir))
        except subprocess.CalledProcessError:
            error("Failed to install pnpm dependencies")
            return False
    
    # Run tests
    try:
        if coverage:
            info("Running Vitest with coverage...")
            run_cmd(["pnpm", "run", "test:coverage"], cwd=str(gui_dir))
            success("GUI tests passed with coverage")
        else:
            info("Running Vitest...")
            run_cmd(["pnpm", "test"], cwd=str(gui_dir))
            success("GUI tests passed")
        return True
    except subprocess.CalledProcessError:
        error("GUI tests failed")
        return False


def run_e2e_tests() -> bool:
    """Run E2E tests with Playwright."""
    step("Running E2E Tests")
    
    gui_dir = Path("gui")
    if not gui_dir.exists():
        error("GUI directory not found")
        return False
    
    # Check if Playwright is installed
    playwright_config = gui_dir / "playwright.config.ts"
    if not playwright_config.exists():
        warn("Playwright not configured. Skipping E2E tests.")
        return True
    
    # Check for node_modules
    node_modules = gui_dir / "node_modules"
    if not node_modules.exists():
        info("Installing pnpm dependencies...")
        try:
            run_cmd(["pnpm", "install"], cwd=str(gui_dir))
        except subprocess.CalledProcessError:
            error("Failed to install pnpm dependencies")
            return False
    
    # Install Playwright browsers if needed
    info("Ensuring Playwright browsers are installed...")
    try:
        run_cmd(["pnpm", "exec", "playwright", "install", "--with-deps"], cwd=str(gui_dir), quiet=not VERBOSE, check=False)
    except (subprocess.CalledProcessError, FileNotFoundError):
        warn("Playwright browser installation may have failed")
    
    try:
        info("Running Playwright E2E tests...")
        run_cmd(["pnpm", "run", "test:e2e"], cwd=str(gui_dir))
        success("E2E tests passed")
        return True
    except subprocess.CalledProcessError:
        error("E2E tests failed")
        return False


def run_audit() -> bool:
    """Run cargo audit for security vulnerabilities."""
    step("Running Cargo Audit")
    
    if not command_exists("cargo-audit"):
        warn("cargo-audit not installed. Install with: cargo install cargo-audit")
        return True
    
    try:
        run_cmd(["cargo", "audit"], check=False)
        success("Audit completed")
        return True
    except Exception as e:
        warn(f"Audit encountered issues: {e}")
        return True  # Don't fail the build for audit issues


def run_clippy() -> bool:
    """Run Clippy linter."""
    step("Running Clippy")
    
    try:
        cmd = ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]
        run_cmd(cmd)
        success("Clippy passed")
        return True
    except subprocess.CalledProcessError:
        error("Clippy found issues")
        return False


# ============================================================
# Main
# ============================================================

def main() -> int:
    global VERBOSE
    
    parser = argparse.ArgumentParser(
        description="Run project tests",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python test.py                    # Run all tests
  python test.py --rust             # Run Rust tests only
  python test.py --gui              # Run GUI tests only
  python test.py --e2e              # Run E2E tests only
  python test.py --rust --coverage  # Run Rust tests with coverage
  python test.py --all --verbose    # Run all tests with full output
  python test.py --log              # Run all tests and log to file
"""
    )
    parser.add_argument("-r", "--rust", action="store_true", help="Run Rust tests")
    parser.add_argument("-g", "--gui", action="store_true", help="Run GUI tests")
    parser.add_argument("-e", "--e2e", action="store_true", help="Run E2E tests")
    parser.add_argument("--coverage", action="store_true", help="Generate coverage reports")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show full output")
    parser.add_argument("-l", "--log", action="store_true", help="Enable logging to file")
    parser.add_argument("-a", "--audit", action="store_true", help="Run cargo audit")
    parser.add_argument("-c", "--clippy", action="store_true", help="Run Clippy linter")
    parser.add_argument("--all", action="store_true", help="Run all tests (default)")
    args = parser.parse_args()
    
    VERBOSE = args.verbose
    LOG_ENABLED = args.log
    
    # Set up logging if enabled
    if LOG_ENABLED:
        setup_logging()
        # Clean up old logs (keep only 3)
        cleanup_old_logs(max_logs=3)
    
    # Determine what to run
    # Only run all if --all specified OR no specific test flags given
    has_specific_test = args.rust or args.gui or args.e2e or args.audit or args.clippy
    run_all = args.all or not has_specific_test
    
    print(f"\n{CYAN}+=========================================+{RESET}")
    print(f"{CYAN}|     Website Searcher Test Suite       |{RESET}")
    print(f"{CYAN}+=========================================+{RESET}\n")
    
    # Log the test suite header
    log_write("\n+=========================================+\n")
    log_write("|     Website Searcher Test Suite       |\n")
    log_write("+=========================================+\n\n")
    
    results = {}
    
    try:
        # Clippy
        if args.clippy or run_all:
            results["Clippy"] = run_clippy()
        
        # Rust tests
        if args.rust or run_all:
            results["Rust Tests"] = run_rust_tests(coverage=args.coverage)
        
        # GUI tests
        if args.gui or run_all:
            results["GUI Tests"] = run_gui_tests(coverage=args.coverage)
        
        # E2E tests
        if args.e2e or run_all:
            results["E2E Tests"] = run_e2e_tests()
        
        # Audit
        if args.audit or run_all:
            results["Audit"] = run_audit()
        
        # Summary
        print(f"\n{CYAN}========================================={RESET}")
        print(f"{CYAN}              Test Summary             {RESET}")
        print(f"{CYAN}========================================={RESET}\n")
        
        all_passed = True
        for name, passed in results.items():
            if passed:
                print(f"  {GREEN}[OK]{RESET} {name}")
            else:
                print(f"  {RED}[FAIL]{RESET} {name}")
                all_passed = False
        
        print()
        
        if all_passed:
            print(f"{GREEN}All tests passed!{RESET}\n")
            return 0
        else:
            print(f"{RED}Some tests failed.{RESET}\n")
            return 1
    
    except KeyboardInterrupt:
        print(f"\n{YELLOW}Tests interrupted{RESET}")
        return 130
    finally:
        close_logging()


if __name__ == "__main__":
    sys.exit(main())

