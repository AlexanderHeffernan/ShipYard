import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const baseVersion = parseVersion(config.version);
const parseTags = (output) => output
  .trim()
  .split('\n')
  .filter(Boolean)
  .map((tag) => parseVersion(tag.slice(1)))
  .filter(Boolean);
const tags = parseTags(execFileSync('git', ['tag', '--list', 'v[0-9]*'], { encoding: 'utf8' }));
const tagsOnCurrentCommit = parseTags(
  execFileSync('git', ['tag', '--points-at', 'HEAD', 'v[0-9]*'], { encoding: 'utf8' }),
);

if (tagsOnCurrentCommit.length > 0) {
  console.log(formatVersion(tagsOnCurrentCommit.reduce((latest, version) => {
    if (!latest || compareVersions(version, latest) > 0) return version;
    return latest;
  }, null)));
  process.exit(0);
}

const latestVersion = tags.reduce((latest, version) => {
  if (!latest || compareVersions(version, latest) > 0) return version;
  return latest;
}, null);

const nextVersion = latestVersion && compareVersions(latestVersion, baseVersion) >= 0
  ? { ...latestVersion, patch: latestVersion.patch + 1 }
  : baseVersion;

console.log(formatVersion(nextVersion));

function parseVersion(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(value);
  if (!match) throw new Error(`Expected a stable semver version, received: ${value}`);
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
  };
}

function compareVersions(left, right) {
  return left.major - right.major || left.minor - right.minor || left.patch - right.patch;
}

function formatVersion(version) {
  return `${version.major}.${version.minor}.${version.patch}`;
}
