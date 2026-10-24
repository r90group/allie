import fs from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import TOML from '@iarna/toml';

const cargo = TOML.parse(fs.readFileSync('Cargo.toml', 'utf8'));
const cargoLock = TOML.parse(fs.readFileSync('Cargo.lock', 'utf8'));
const worker = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const workerLock = JSON.parse(fs.readFileSync('package-lock.json', 'utf8'));
const packages = cargoLock.package.filter((entry) => entry.name === 'allie');
const sourceVersion = cargo.package.version;
const semver = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/;
if (!semver.test(sourceVersion)) {
  throw new Error('Release requires a stable SemVer version');
}
if (packages.length !== 1 || packages[0].version !== sourceVersion ||
    worker.version !== sourceVersion || workerLock.version !== sourceVersion ||
    workerLock.packages[''].version !== sourceVersion ||
    JSON.parse(fs.readFileSync('.release-please-manifest.json', 'utf8'))['.'] !== sourceVersion) {
  throw new Error('Rust and browser-worker manifests and lockfiles must agree before release');
}
const entries = fs.readFileSync('CHANGELOG.md', 'utf8').split(/(?=^## )/m).slice(1);
if (entries[0]?.match(/^## \[?(\d+\.\d+\.\d+)/)?.[1] !== sourceVersion) {
  throw new Error('Current changelog entry must match the release version');
}
const tag = `v${sourceVersion}`;
if (process.argv[2] === '--notes') {
  process.stdout.write(entries[0].trimEnd() + '\n');
  process.exit(0);
}
if (process.argv[2] === '--select') {
  const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
  if (git('rev-parse', '--is-shallow-repository') !== 'false') throw new Error('Release requires full Git history, not a shallow checkout');
  const parent = spawnSync('git', ['show', 'HEAD^:.release-please-manifest.json'], { encoding: 'utf8' });
  let previous;
  if (parent.status === 0) {
    previous = JSON.parse(parent.stdout)['.'];
  } else {
    const baseline = JSON.parse(fs.readFileSync('release-please-config.json', 'utf8'))['bootstrap-sha'];
    if (git('rev-parse', `refs/tags/${tag}^{commit}`) !== baseline) throw new Error('Missing immutable release baseline');
    previous = sourceVersion; // Adoption aligns an existing release; never republishes it.
  }
  let publish = previous !== sourceVersion;
  if (publish) {
    if (!semver.test(previous)) throw new Error('Missing valid previous release version');
    const before = previous.split('.').map(BigInt);
    const after = sourceVersion.split('.').map(BigInt);
    const index = after.findIndex((value, i) => value !== before[i]);
    if (index < 0 || after[index] < before[index]) throw new Error('Refusing a decreasing release version');
    const title = `chore(master): release ${sourceVersion}`;
    const subject = git('log', '-1', '--format=%s').replace(/ \(#\d+\)$/, '');
    if (subject !== title) throw new Error('Only a generated release commit may publish');
    const existing = spawnSync('git', ['rev-parse', '--verify', `refs/tags/${tag}^{commit}`], { encoding: 'utf8' });
    if (existing.status === 0) {
      if (existing.stdout.trim() !== git('rev-parse', 'HEAD')) throw new Error('Refusing a conflicting release tag');
      publish = false;
    }
  }
  if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `publish=${publish}\n`);
  console.log(`publish=${publish}`);
}
if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\n`);
console.log(tag);
