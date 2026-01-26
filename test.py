#!/usr/bin/env python3
"""
Cross-platform test script for running project tests.
Assumes the project is already compiled.

Usage: python test.py [OPTIONS]

Options:
    -r, --rust      Run Rust tests only
    -g, --gui       Run GUI tests only
    -e, --e2e       Run E2E tests only
    -c, --coverage  Generate coverage reports
    -v, --verbose   Show full output
    -a, --audit     Run cargo audit
    --all           Run all tests (default if no specific option given)
"""
from __future__ import annotations

import argparse
import glob
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

ANSI_ESCAPE = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')


def strip_ansi(text: str) -> str:
    """Remove ANSI escape codes from text."""
    return ANSI_ESCAPE.sub('', text)


def step(name: str) -> None:
    """Print a step header."""
    print(f"{GREEN}==> {name}{RESET}")


def info(msg: str) -> None:
    """Print info message."""
    print(f"{CYAN}    {msg}{RESET}")


def warn(msg: str) -> None:
    """Print warning message."""
    print(f"{YELLOW}    Warning: {msg}{RESET}")


def error(msg: str) -> None:
    """Print error message."""
    print(f"{RED}    Error: {msg}{RESET}")


def success(msg: str) -> None:
    """Print success message."""
    print(f"{GREEN}    ✓ {msg}{RESET}")


def run_cmd(cmd, shell: bool = False, check: bool = True,
            cwd: Optional[str] = None, quiet: bool = False) -> Optional[subprocess.CompletedProcess]:
    """
    Run a command with output handling.
    """
    global VERBOSE
    
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
"""
    )
    parser.add_argument("-r", "--rust", action="store_true", help="Run Rust tests")
    parser.add_argument("-g", "--gui", action="store_true", help="Run GUI tests")
    parser.add_argument("-e", "--e2e", action="store_true", help="Run E2E tests")
    parser.add_argument("-c", "--coverage", action="store_true", help="Generate coverage reports")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show full output")
    parser.add_argument("-a", "--audit", action="store_true", help="Run cargo audit")
    parser.add_argument("--clippy", action="store_true", help="Run Clippy linter")
    parser.add_argument("--all", action="store_true", help="Run all tests (default)")
    args = parser.parse_args()
    
    VERBOSE = args.verbose
    
    # Determine what to run
    run_all = args.all or not (args.rust or args.gui or args.e2e or args.audit or args.clippy)
    
    print(f"\n{CYAN}+=========================================+{RESET}")
    print(f"{CYAN}|     Website Searcher Test Suite       |{RESET}")
    print(f"{CYAN}+=========================================+{RESET}\n")
    
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


if __name__ == "__main__":
    sys.exit(main())

