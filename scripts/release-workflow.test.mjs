import { describe, expect, it } from 'vitest';

import {
  findReleaseByTag,
  findUpdaterManifestAsset,
  releaseVersion,
  updaterPlatforms,
  validatePublishedRelease,
  validateUpdaterManifest,
} from './release-workflow.mjs';

const tag = 'v0.1.10';
const release = {
  id: 123,
  tag_name: tag,
  draft: true,
  assets: [
    { id: 456, name: 'latest.json', state: 'uploaded' },
  ],
};

function completeManifest() {
  return {
    version: '0.1.10',
    platforms: Object.fromEntries(updaterPlatforms.map((platform) => [platform, {
      url: `https://example.test/${platform}`,
      signature: 'signature',
    }])),
  };
}

describe('release workflow helpers', () => {
  it('resolves a draft release by exact tag from the release list', () => {
    expect(findReleaseByTag([release], tag)).toBe(release);
  });

  it('rejects a missing or ambiguous release tag', () => {
    expect(() => findReleaseByTag([], tag)).toThrow('No GitHub Release');
    expect(() => findReleaseByTag([release, { ...release, id: 789 }], tag))
      .toThrow('Expected one GitHub Release');
  });

  it('requires one uploaded updater manifest asset', () => {
    expect(findUpdaterManifestAsset(release, tag).id).toBe(456);
    expect(() => findUpdaterManifestAsset({ ...release, assets: [] }, tag))
      .toThrow('exactly one latest.json');
    expect(() => findUpdaterManifestAsset({
      ...release,
      assets: [{ id: 456, name: 'latest.json', state: 'new' }],
    }, tag)).toThrow('not uploaded');
  });

  it('validates the version and every macOS updater platform', () => {
    expect(() => validateUpdaterManifest(completeManifest(), tag)).not.toThrow();
    expect(() => validateUpdaterManifest({ ...completeManifest(), version: '0.1.9' }, tag))
      .toThrow('does not match');

    const missingPlatform = completeManifest();
    delete missingPlatform.platforms['darwin-x86_64-app'];
    expect(() => validateUpdaterManifest(missingPlatform, tag))
      .toThrow('darwin-x86_64-app');
  });

  it('only accepts stable v-prefixed release tags', () => {
    expect(releaseVersion(tag)).toBe('0.1.10');
    expect(() => releaseVersion('0.1.10')).toThrow('stable release tag');
  });

  it('confirms that publishing cleared the draft flag', () => {
    expect(() => validatePublishedRelease({ ...release, draft: false }, tag)).not.toThrow();
    expect(() => validatePublishedRelease(release, tag)).toThrow('still a draft');
  });
});
