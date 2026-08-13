import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const updaterPlatforms = [
  'darwin-aarch64',
  'darwin-aarch64-app',
  'darwin-x86_64',
  'darwin-x86_64-app',
];

export function findReleaseByTag(releases, tag) {
  if (!Array.isArray(releases)) {
    throw new Error('The GitHub release listing was not an array.');
  }

  const matches = releases.filter((release) => release?.tag_name === tag);
  if (matches.length === 0) {
    throw new Error(
      `No GitHub Release with the exact tag ${tag} was found. `
      + 'The draft release expected from tauri-action may be missing.',
    );
  }
  if (matches.length > 1) {
    throw new Error(`Expected one GitHub Release with tag ${tag}, found ${matches.length}.`);
  }

  const [release] = matches;
  if (!Number.isInteger(release.id) || release.id < 1) {
    throw new Error(`The GitHub Release with tag ${tag} did not have a valid numeric ID.`);
  }

  return release;
}

export function findUpdaterManifestAsset(release, tag) {
  if (release?.tag_name !== tag) {
    throw new Error(`The fetched GitHub Release did not match the expected tag ${tag}.`);
  }

  const assets = Array.isArray(release.assets)
    ? release.assets.filter((asset) => asset?.name === 'latest.json')
    : [];
  if (assets.length !== 1) {
    throw new Error(
      `Expected exactly one latest.json asset for ${tag}, found ${assets.length}.`,
    );
  }

  const [asset] = assets;
  if (asset.state !== 'uploaded') {
    throw new Error(
      `The latest.json asset for ${tag} is not uploaded (state: ${asset.state ?? 'unknown'}).`,
    );
  }
  if (!Number.isInteger(asset.id) || asset.id < 1) {
    throw new Error(`The latest.json asset for ${tag} did not have a valid numeric ID.`);
  }

  return asset;
}

export function validateUpdaterManifest(manifest, tag) {
  const version = releaseVersion(tag);
  if (manifest?.version !== version) {
    throw new Error(
      `The updater manifest version ${manifest?.version ?? 'missing'} does not match ${version}.`,
    );
  }

  for (const platform of updaterPlatforms) {
    const entry = manifest.platforms?.[platform];
    if (
      typeof entry?.url !== 'string'
      || entry.url.length === 0
      || typeof entry.signature !== 'string'
      || entry.signature.length === 0
    ) {
      throw new Error(`The updater manifest is missing ${platform} URL or signature.`);
    }
  }
}

export function validatePublishedRelease(release, tag) {
  if (release?.tag_name !== tag) {
    throw new Error(`The published GitHub Release did not match the expected tag ${tag}.`);
  }
  if (release.draft !== false) {
    throw new Error(`The GitHub Release ${tag} is still a draft after publishing.`);
  }
}

export function releaseVersion(tag) {
  const match = /^v(\d+\.\d+\.\d+)$/.exec(tag);
  if (!match) {
    throw new Error(`Expected a stable release tag such as v0.1.10, received ${tag}.`);
  }
  return match[1];
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function runCommand(args) {
  const [command, ...commandArgs] = args;
  if (command === 'release-id' && commandArgs.length === 2) {
    console.log(findReleaseByTag(readJson(commandArgs[0]), commandArgs[1]).id);
    return;
  }
  if (command === 'manifest-asset-id' && commandArgs.length === 2) {
    console.log(findUpdaterManifestAsset(readJson(commandArgs[0]), commandArgs[1]).id);
    return;
  }
  if (command === 'validate-manifest' && commandArgs.length === 2) {
    validateUpdaterManifest(readJson(commandArgs[0]), commandArgs[1]);
    return;
  }
  if (command === 'verify-published' && commandArgs.length === 2) {
    validatePublishedRelease(readJson(commandArgs[0]), commandArgs[1]);
    return;
  }

  throw new Error(
    'Usage: release-workflow.mjs '
    + '(release-id|manifest-asset-id|validate-manifest|verify-published) <json-file> <tag>',
  );
}

const isMainModule = process.argv[1]
  && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;

if (isMainModule) {
  try {
    runCommand(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
