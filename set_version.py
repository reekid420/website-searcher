#!/usr/bin/env python3
"""
Simple script to update the project version across all files.

Usage:
    python set_version.py 1.2.3
"""

import sys
import re
from pathlib import Path


def update_cargo_toml(file_path: Path, new_version: str) -> bool:
    """Update version in a Cargo.toml file."""
    try:
        content = file_path.read_text(encoding='utf-8')
        original = content
        
        # Match version = "x.y.z" in [package] section
        content = re.sub(
            r'(^\[package\].*?^version\s*=\s*)"[^"]*"',
            rf'\1"{new_version}"',
            content,
            count=1,
            flags=re.MULTILINE | re.DOTALL
        )
        
        # Match version = "x.y.z" in [workspace.package] section (workspace root)
        content = re.sub(
            r'(^\[workspace\.package\].*?^version\s*=\s*)"[^"]*"',
            rf'\1"{new_version}"',
            content,
            count=1,
            flags=re.MULTILINE | re.DOTALL
        )
        
        if content != original:
            file_path.write_text(content, encoding='utf-8')
            print(f'✓ Updated {file_path}')
            return True
        else:
            print(f'⚠ No changes in {file_path}')
            return False
    except Exception as e:
        print(f'✗ Error updating {file_path}: {e}')
        return False


def update_package_json(file_path: Path, new_version: str) -> bool:
    """Update version in package.json file."""
    try:
        content = file_path.read_text(encoding='utf-8')
        # Match "version": "x.y.z"
        updated = re.sub(
            r'"version"\s*:\s*"[^"]*"',
            f'"version": "{new_version}"',
            content,
            count=1
        )
        
        if updated != content:
            file_path.write_text(updated, encoding='utf-8')
            print(f'✓ Updated {file_path}')
            return True
        else:
            print(f'⚠ No changes in {file_path}')
            return False
    except Exception as e:
        print(f'✗ Error updating {file_path}: {e}')
        return False


def update_tauri_conf(file_path: Path, new_version: str) -> bool:
    """Update version in tauri.conf.json file."""
    try:
        content = file_path.read_text(encoding='utf-8')
        # Match "version": "x.y.z" in tauri.conf.json
        updated = re.sub(
            r'("version"\s*:\s*)"[^"]*"',
            rf'\1"{new_version}"',
            content,
            count=1
        )
        
        if updated != content:
            file_path.write_text(updated, encoding='utf-8')
            print(f'✓ Updated {file_path}')
            return True
        else:
            print(f'⚠ No changes in {file_path}')
            return False
    except Exception as e:
        print(f'✗ Error updating {file_path}: {e}')
        return False


def main():
    if len(sys.argv) != 2:
        print('Usage: python set_version.py <version>')
        print('Example: python set_version.py 1.2.3')
        sys.exit(1)
    
    new_version = sys.argv[1]
    
    # Validate version format (semver)
    if not re.match(r'^\d+\.\d+\.\d+$', new_version):
        print(f'Error: Invalid version format "{new_version}"')
        print('Version must be in format: major.minor.patch (e.g., 1.2.3)')
        sys.exit(1)
    
    print(f'\nUpdating project version to {new_version}...\n')
    
    root = Path(__file__).parent
    updated_count = 0
    
    # Update Cargo.toml files
    cargo_files = [
        root / 'Cargo.toml',
        root / 'crates' / 'cli' / 'Cargo.toml',
        root / 'crates' / 'core' / 'Cargo.toml',
        root / 'src-tauri' / 'Cargo.toml',
    ]
    
    for cargo_file in cargo_files:
        if cargo_file.exists():
            if update_cargo_toml(cargo_file, new_version):
                updated_count += 1
    
    # Update package.json
    package_json = root / 'gui' / 'package.json'
    if package_json.exists():
        if update_package_json(package_json, new_version):
            updated_count += 1
    
    # Update tauri.conf.json (for MSI version)
    tauri_conf = root / 'src-tauri' / 'tauri.conf.json'
    if tauri_conf.exists():
        if update_tauri_conf(tauri_conf, new_version):
            updated_count += 1
    
    print(f'\n✓ Updated {updated_count} files to version {new_version}')
    print('\nRun `cargo update` or `python compile.py` to apply changes.')


if __name__ == '__main__':
    main()
