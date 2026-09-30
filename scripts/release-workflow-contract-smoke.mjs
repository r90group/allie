import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import TOML from '@iarna/toml';

const AUDIT_PATH = '.cargo/audit.toml';
const WAIVER_PATH = '.cargo/audit-waivers.toml';

function fail(message) {
  throw new Error(message);
}


function exactKeys(value, expected, label) {
  const actual = Object.keys(value || {}).sort();
  const wanted = [...expected].sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) {
    fail(`${label} keys must be exactly [${wanted.join(', ')}], got [${actual.join(', ')}]`);
  }
}


function parseToml(text, label) {
  try {
    return TOML.parse(text);
  } catch (error) {
    fail(`${label} is malformed TOML: ${error.message}`);
  }
}

function validateAuditPolicy(auditText, waiverText, today = new Date()) {
  const audit = parseToml(auditText, 'cargo-audit policy');
  const policy = parseToml(waiverText, 'audit waiver policy');
  const ignored = audit.advisories?.ignore;
  if (!Array.isArray(ignored)) fail('cargo-audit policy must declare advisories.ignore');
  if (policy.schema !== 1 || !Array.isArray(policy.waiver)) fail('audit waiver policy must declare schema=1 and waiver=[]');
  const records = new Map();
  for (const waiver of policy.waiver) {
    exactKeys(waiver, ['advisory', 'expiry', 'owner', 'rationale', 'removal', 'tracking_ref'], 'audit waiver');
    for (const field of ['advisory', 'owner', 'rationale', 'removal', 'tracking_ref']) {
      if (typeof waiver[field] !== 'string' || waiver[field].trim() === '') fail(`audit waiver has invalid ${field}`);
    }
    if (!/^RUSTSEC-\d{4}-\d{4}$/.test(waiver.advisory)) fail(`invalid advisory ID ${waiver.advisory}`);
    if (!(waiver.expiry instanceof Date) || waiver.expiry.isDate !== true || Number.isNaN(waiver.expiry.valueOf())) {
      fail(`${waiver.advisory} expiry must be a valid TOML calendar date`);
    }
    const todayUtc = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate());
    if (waiver.expiry.valueOf() <= todayUtc) fail(`${waiver.advisory} waiver is expired`);
    if (records.has(waiver.advisory)) fail(`duplicate waiver metadata for ${waiver.advisory}`);
    records.set(waiver.advisory, waiver);
  }
  for (const advisory of ignored) {
    if (!records.has(advisory)) fail(`${advisory} is ignored without structured waiver metadata`);
  }
  for (const advisory of records.keys()) {
    if (!ignored.includes(advisory)) fail(`${advisory} waiver metadata is not present in advisories.ignore`);
  }
}



function expectRejected(action, label) {
  try {
    action();
  } catch {
    return;
  }
  fail(`negative control was accepted: ${label}`);
}

const auditText = fs.readFileSync(AUDIT_PATH, 'utf8');
const waiverText = fs.readFileSync(WAIVER_PATH, 'utf8');
validateAuditPolicy(auditText, waiverText);

// Exercise the version preparer as a consumer: real Git history, all four
// manifests/locks, and rejection before any file changes on invalid inputs.
const preparer = path.resolve('scripts/prepare-release.mjs');
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'allie-release-version-'));
const inputs = {
  'Cargo.toml': '[package]\nname = "allie"\nversion = "0.3.0"\n[dependencies]\nserde = "1"\n',
  'Cargo.lock': 'version = 4\n[[package]]\nname = "allie"\nversion = "0.3.0"\n[[package]]\nname = "serde"\nversion = "1.0.228"\n',
  'package.json': JSON.stringify({ name: 'allie-browser-worker', version: '0.3.0' }),
  'package-lock.json': JSON.stringify({
    name: 'allie-browser-worker', version: '0.3.0', lockfileVersion: 3,
    packages: { '': { version: '0.3.0' }, 'node_modules/playwright': { version: '1.61.0' } },
  }),
};
const git = (...args) => {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) fail(result.stderr);
};
const restore = () => {
  for (const [file, contents] of Object.entries(inputs)) fs.writeFileSync(path.join(root, file), contents);
};
const prepare = (cwd = root) => spawnSync(process.execPath, [preparer], { cwd, encoding: 'utf8' });
const rejectWithoutMutation = (label) => {
  const before = Object.keys(inputs).map((file) => fs.readFileSync(path.join(root, file), 'utf8'));
  const result = prepare();
  if (result.status === 0) fail(`${label} was accepted`);
  const after = Object.keys(inputs).map((file) => fs.readFileSync(path.join(root, file), 'utf8'));
  if (JSON.stringify(before) !== JSON.stringify(after)) fail(`${label} partially mutated release inputs`);
};
try {
  restore();
  git('init', '-q');
  git('add', '.');
  for (let i = 0; i < 2; i++) {
    git('-c', 'user.name=Allie Release Smoke', '-c', 'user.email=allie@example.invalid',
      'commit', '-q', '--allow-empty', '-m', `release fixture ${i}`);
  }
  const result = prepare();
  if (result.status !== 0 || result.stdout.trim() !== 'v0.3.2') fail(`version preparation failed: ${result.stderr}`);
  const cargo = TOML.parse(fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8'));
  const lock = TOML.parse(fs.readFileSync(path.join(root, 'Cargo.lock'), 'utf8'));
  const worker = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  const workerLock = JSON.parse(fs.readFileSync(path.join(root, 'package-lock.json'), 'utf8'));
  const versions = [cargo.package.version, lock.package[0].version, worker.version, workerLock.version, workerLock.packages[''].version];
  if (versions.some((version) => version !== '0.3.2')) fail(`release version disagreement: ${versions}`);
  if (cargo.dependencies.serde !== '1' || lock.package[1].version !== '1.0.228' ||
      workerLock.packages['node_modules/playwright'].version !== '1.61.0') fail('version preparation changed a dependency');
  restore();
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ version: '0.4.0' }));
  rejectWithoutMutation('mismatched worker version');
  restore();
  fs.writeFileSync(path.join(root, 'Cargo.toml'), '[package]\nname = "allie"\nversion = "1.0.0"\n');
  rejectWithoutMutation('stable source release line');
  restore();
  const shallow = path.join(root, 'shallow');
  git('clone', '--quiet', '--depth', '1', `file://${root}`, shallow);
  const shallowResult = prepare(shallow);
  if (shallowResult.status === 0 || !shallowResult.stderr.includes('shallow checkout')) fail('shallow history was accepted');
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}

const documentedAdvisory = '[advisories]\nignore = ["RUSTSEC-2099-0001"]\n';
const validWaiver = 'schema = 1\n[[waiver]]\nadvisory = "RUSTSEC-2099-0001"\ntracking_ref = "AL-999"\nrationale = "test"\nowner = "security"\nexpiry = 2099-01-01\nremoval = "upgrade"\n';
validateAuditPolicy(documentedAdvisory, validWaiver, new Date('2026-01-01T00:00:00Z'));

expectRejected(() => validateAuditPolicy('[advisories\nignore = []', waiverText), 'malformed TOML');
expectRejected(
  () => validateAuditPolicy(
    documentedAdvisory,
    'schema = 1\n[[waiver]]\nadvisory = "RUSTSEC-2099-0001"\ntracking_ref = "AL-999"\nrationale = "test"\nowner = "security"\nexpiry = 2026-02-30\nremoval = "upgrade"\n',
  ),
  'invalid calendar expiry',
);
expectRejected(
  () => validateAuditPolicy(
    documentedAdvisory,
    'schema = 1\n[[waiver]]\nadvisory = "RUSTSEC-2099-0001"\ntracking_ref = "AL-999"\nrationale = "test"\nowner = "security"\nexpiry = 2000-01-01\nremoval = "upgrade"\n',
    new Date('2026-01-01T00:00:00Z'),
  ),
  'expired waiver',
);
expectRejected(
  () => validateAuditPolicy('[advisories]\nignore = ["RUSTSEC-2099-0001"]\n', 'schema = 1\nwaiver = []\n'),
  'undocumented ignored advisory',
);

console.log('release policy smoke passed: coherent monotonic artifact versions and fail-closed audit waivers');
