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

// 3. Promote CHANGELOG.md's [Unreleased] section to this version.
const changelogPath = resolve(repoRoot, 'CHANGELOG.md');
const changelog = readFileSync(changelogPath, 'utf8');

const unreleasedHeading = '## [Unreleased]';
const unreleasedIdx = changelog.indexOf(unreleasedHeading);
if (unreleasedIdx === -1) {
  console.error('Could not find "## [Unreleased]" heading in CHANGELOG.md');
  process.exit(1);
}

const today = new Date().toISOString().slice(0, 10);
const newHeading = `## v${newVersion} — ${today}`;

if (changelog.includes(newHeading) || changelog.includes(`## [${newVersion}]`)) {
  console.error(`CHANGELOG.md already has an entry for v${newVersion}`);
  process.exit(1);
}

// Keep "## [Unreleased]" as a fresh empty heading; rename what follows it
// (up to the next H2) to the version heading being released.
const promotedChangelog =
  changelog.slice(0, unreleasedIdx) +
  unreleasedHeading +
  '\n\n' +
  newHeading +
  changelog.slice(unreleasedIdx + unreleasedHeading.length);

// Update the compare links at the foot of the file: point [Unreleased] at
// the new version and add a link for the new version itself.
const linkLineRe = /^\[Unreleased\]:\s*(\S+)\/compare\/v([\d.]+)\.\.\.HEAD\s*$/m;
const linkMatch = promotedChangelog.match(linkLineRe);
if (!linkMatch) {
  console.error('Could not find the [Unreleased] compare link at the foot of CHANGELOG.md');
  process.exit(1);
}
const [, repoUrl, prevVersion] = linkMatch;
const newLinkLines =
  `[Unreleased]: ${repoUrl}/compare/v${newVersion}...HEAD\n` +
  `[${newVersion}]: ${repoUrl}/compare/v${prevVersion}...v${newVersion}`;
const finalChangelog = promotedChangelog.replace(linkLineRe, newLinkLines);

writeFileSync(changelogPath, finalChangelog);
console.log(`Promoted [Unreleased] → v${newVersion} in CHANGELOG.md`);

// 4. Commit + tag.
execSync('git add package.json src-tauri/Cargo.toml CHANGELOG.md', { cwd: repoRoot, stdio: 'inherit' });
execSync(`git commit -m "release: v${newVersion}"`, { cwd: repoRoot, stdio: 'inherit' });
execSync(`git tag v${newVersion}`, { cwd: repoRoot, stdio: 'inherit' });

console.log(`\nCreated commit + tag v${newVersion}.`);
console.log('Next: git push && git push --tags');
