import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import TOML from '@iarna/toml';

const cargo = TOML.parse(fs.readFileSync('Cargo.toml', 'utf8'));
const cargoLock = TOML.parse(fs.readFileSync('Cargo.lock', 'utf8'));
const worker = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const workerLock = JSON.parse(fs.readFileSync('package-lock.json', 'utf8'));
const packages = cargoLock.package.filter((entry) => entry.name === 'allie');
const sourceVersion = cargo.package.version;
if (!/^0\.(0|[1-9][0-9]*)\.0$/.test(sourceVersion)) {
  throw new Error('The source release line must be 0.<minor>.0; CI owns the patch version');
}
if (packages.length !== 1 || packages[0].version !== sourceVersion ||
    worker.version !== sourceVersion || workerLock.version !== sourceVersion ||
    workerLock.packages[''].version !== sourceVersion) {
  throw new Error('Rust and browser-worker manifests and lockfiles must agree before release');
}
if (process.argv[2] === '--check') {
  console.log(`Source release line validated: ${sourceVersion}`);
  process.exit(0);
}

// Full first-parent history gives each default-branch revision a stable,
// increasing patch without a version-bump commit or shared tag-allocation race.
const sequence = execFileSync('git', ['rev-list', '--first-parent', '--count', 'HEAD'], { encoding: 'utf8' }).trim();
if (!/^[1-9][0-9]*$/.test(sequence)) throw new Error('Release requires nonempty full Git history');
if (execFileSync('git', ['rev-parse', '--is-shallow-repository'], { encoding: 'utf8' }).trim() !== 'false') {
  throw new Error('Release requires full Git history, not a shallow checkout');
}
const version = `0.${sourceVersion.split('.')[1]}.${sequence}`;
cargo.package.version = version;
packages[0].version = version;
worker.version = version;
workerLock.version = version;
workerLock.packages[''].version = version;
fs.writeFileSync('Cargo.toml', TOML.stringify(cargo));
fs.writeFileSync('Cargo.lock', TOML.stringify(cargoLock));
fs.writeFileSync('package.json', `${JSON.stringify(worker, null, 2)}\n`);
fs.writeFileSync('package-lock.json', `${JSON.stringify(workerLock, null, 2)}\n`);
const tag = `v${version}`;
if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\n`);
console.log(tag);
