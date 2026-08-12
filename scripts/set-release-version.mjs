import fs from 'node:fs';

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error('Usage: node scripts/set-release-version.mjs <major.minor.patch>');
}

updateJson('package.json', (packageJson) => {
  packageJson.version = version;
});
updateJson('package-lock.json', (packageLock) => {
  packageLock.version = version;
  packageLock.packages[''].version = version;
});
updateJson('src-tauri/tauri.conf.json', (config) => {
  config.version = version;
});

replaceVersion('src-tauri/Cargo.toml', /^(version = ")[^"]+("\s*$)/m);
replaceVersion(
  'src-tauri/Cargo.lock',
  /(^\[\[package\]\]\nname = "shipyard"\nversion = ")[^"]+("\s*$)/m,
);

function updateJson(path, update) {
  const value = JSON.parse(fs.readFileSync(path, 'utf8'));
  update(value);
  fs.writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function replaceVersion(path, pattern) {
  const contents = fs.readFileSync(path, 'utf8');
  const updated = contents.replace(pattern, `$1${version}$2`);
  if (updated === contents) throw new Error(`Could not update the version in ${path}`);
  fs.writeFileSync(path, updated);
}
