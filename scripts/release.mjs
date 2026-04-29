#!/usr/bin/env node
// Bump app version across package.json and src-tauri/Cargo.toml,
// commit, and create a git tag. Push is left to the user.
//
// Usage:  node scripts/release.mjs <new-version>
// Example: node scripts/release.mjs 0.2.0

import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const newVersion = process.argv[2];

if (!newVersion || !/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error('Usage: node scripts/release.mjs <MAJOR.MINOR.PATCH>');
  process.exit(1);
}

// Refuse to release with a dirty tree.
const status = execSync('git status --porcelain', { cwd: repoRoot }).toString().trim();
if (status) {
  console.error('Refusing to release: working tree not clean.');
  console.error(status);
  process.exit(1);
}

// 1. Update package.json
const pkgPath = resolve(repoRoot, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
const oldVersion = pkg.version;
pkg.version = newVersion;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');

// 2. Update src-tauri/Cargo.toml — surgical replace, NOT a parser, to preserve formatting.
const cargoPath = resolve(repoRoot, 'src-tauri', 'Cargo.toml');
const cargo = readFileSync(cargoPath, 'utf8');
const cargoLine = /^version\s*=\s*"[^"]+"/m;
if (!cargoLine.test(cargo)) {
  console.error('Could not find version line in src-tauri/Cargo.toml');
  process.exit(1);
}
writeFileSync(cargoPath, cargo.replace(cargoLine, `version = "${newVersion}"`));

console.log(`Bumped ${oldVersion} → ${newVersion}`);

// 3. Commit + tag.
execSync('git add package.json src-tauri/Cargo.toml', { cwd: repoRoot, stdio: 'inherit' });
execSync(`git commit -m "release: v${newVersion}"`, { cwd: repoRoot, stdio: 'inherit' });
execSync(`git tag v${newVersion}`, { cwd: repoRoot, stdio: 'inherit' });

console.log(`\nCreated commit + tag v${newVersion}.`);
console.log('Next: git push && git push --tags');
