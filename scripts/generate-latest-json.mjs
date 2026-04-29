#!/usr/bin/env node
// Compose latest.json from a directory of release artifacts. Written for
// the GitHub Actions release.yml — runs after the matrix build downloads
// every platform's artifacts into one folder.
//
// Usage:
//   node scripts/generate-latest-json.mjs --tag v0.2.0 --dir ./artifacts > latest.json
//
// Required artifacts in <dir>:
//   claude-limits_<ver>_universal.app.tar.gz       + .sig
//   claude-limits_<ver>_x64-setup.nsis.zip         + .sig

import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, cur, i, arr) => {
    if (cur.startsWith('--')) acc.push([cur.slice(2), arr[i + 1]]);
    return acc;
  }, []),
);

const tag = args.tag;
const dir = args.dir;
const repo = args.repo ?? 'FeiXu-1131372/claude-limits';

if (!tag || !dir) {
  console.error('Usage: --tag v0.2.0 --dir ./artifacts [--repo owner/name]');
  process.exit(1);
}

const version = tag.replace(/^v/, '');
const baseUrl = `https://github.com/${repo}/releases/download/${tag}`;
const files = readdirSync(resolve(dir));

function findArtifact(suffix) {
  const match = files.find((f) => f.endsWith(suffix));
  if (!match) {
    console.error(`Missing artifact: *${suffix}`);
    process.exit(1);
  }
  return match;
}

function readSig(artifactName) {
  const sigName = `${artifactName}.sig`;
  const sigPath = resolve(dir, sigName);
  try {
    return readFileSync(sigPath, 'utf8').trim();
  } catch {
    console.error(`Missing signature file: ${sigName}`);
    process.exit(1);
  }
}

const macArtifact = findArtifact('.app.tar.gz');
const winArtifact = findArtifact('.nsis.zip');

const manifest = {
  version,
  notes: `See release notes at https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'darwin-x86_64': {
      signature: readSig(macArtifact),
      url: `${baseUrl}/${macArtifact}`,
    },
    'darwin-aarch64': {
      signature: readSig(macArtifact),
      url: `${baseUrl}/${macArtifact}`,
    },
    'windows-x86_64': {
      signature: readSig(winArtifact),
      url: `${baseUrl}/${winArtifact}`,
    },
  },
};

process.stdout.write(JSON.stringify(manifest, null, 2) + '\n');
