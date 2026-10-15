import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import TOML from '@iarna/toml';
import { parseDocument, stringify as stringifyYaml } from 'yaml';

const AUDIT_PATH = '.cargo/audit.toml';
const WAIVER_PATH = '.cargo/audit-waivers.toml';
const RELEASE_PATH = '.github/workflows/release.yml';
const CI_PATH = '.github/workflows/ci.yml';


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
// Executable supply-chain policy, evaluated on the actual candidate config.
// These are independently required security boundaries, not workflow snapshots.
function parseYaml(text) {
  const document = parseDocument(text);
  if (document.errors.length) fail(document.errors[0].message);
  return document.toJS();
}

function expression(value) {
  return String(value || '').replace(/\$\{\{|\}\}|\s/g, '');
}

function validateReleaseWorkflow(text) {
  const workflow = parseYaml(text);
  exactKeys(workflow.permissions, [], 'global release permissions');
  exactKeys(workflow.on, ['workflow_run'], 'release trigger');
  const trigger = workflow.on.workflow_run;
  if (JSON.stringify(trigger.workflows) !== '["ci"]' || JSON.stringify(trigger.types) !== '["completed"]' ||
      JSON.stringify(trigger.branches) !== '["master"]') fail('Release must follow completed master CI');
  const jobs = workflow.jobs;
  exactKeys(jobs, ['select-release', 'build-release', 'sign-and-publish', 'smoke-published', 'release-channel', 'retract-failed'], 'release jobs');
  for (const name of ['select-release', 'build-release', 'smoke-published']) {
    exactKeys(jobs[name].permissions, ['contents'], `${name} permissions`);
    if (jobs[name].permissions.contents !== 'read') fail(`${name} must be read-only`);
  }
  for (const name of ['release-channel', 'retract-failed']) {
    exactKeys(jobs[name].permissions, ['contents'], `${name} permissions`);
    if (jobs[name].permissions.contents !== 'write') fail(`${name} may only mutate release state`);
  }
  exactKeys(jobs['sign-and-publish'].permissions, ['contents', 'id-token', 'pull-requests'], 'signing permissions');
  if (jobs['sign-and-publish'].permissions.contents !== 'write' ||
      jobs['sign-and-publish'].permissions['id-token'] !== 'write' ||
      jobs['sign-and-publish'].permissions['pull-requests'] !== 'write') fail('Only isolated signing gets OIDC and completes the release PR');
  const build = jobs['build-release'];
  const signer = jobs['sign-and-publish'];
  const smoke = jobs['smoke-published'];
  const channel = jobs['release-channel'];
  const retract = jobs['retract-failed'];
  const trustedPush = "github.event.workflow_run.event=='push'&&github.event.workflow_run.conclusion=='success'&&github.event.workflow_run.head_branch=='master'&&github.event.workflow_run.head_repository.full_name==github.repository";
  if (expression(jobs['select-release'].if) !== trustedPush || build.needs !== 'select-release' ||
      expression(build.if) !== "needs.select-release.outputs.publish=='true'") fail('Build must follow trusted CI and a selected release');
  if (build['runs-on'] !== 'ubuntu-22.04') fail('Build must preserve the glibc 2.35 compatibility floor');
  if (signer.needs !== 'build-release' || JSON.stringify(smoke.needs) !== '["build-release","sign-and-publish"]') {
    fail('Signing and downloaded smoke must follow the candidate build');
  }
  if (expression(channel.if) !== "needs.smoke-published.result=='success'" ||
      expression(retract.if) !== "!cancelled()&&needs.sign-and-publish.result=='success'&&needs.smoke-published.result!='success'") {
    fail('Healthy promotion and failed-candidate quarantine must be mutually exclusive');
  }
  if (!channel.concurrency?.group || channel.concurrency['cancel-in-progress'] !== false || retract.concurrency) {
    fail('Serialize only healthy promotion; quarantine may never be coalesced');
  }
  for (const [name, job] of Object.entries(jobs)) {
    if (job['continue-on-error']) fail(`${name} bypasses failure`);
    for (const step of job.steps || []) {
      if (step['continue-on-error'] || step.if) fail(`${name} may not skip release safety steps`);
      if (step.uses && !/^[^@\s]+@[0-9a-f]{40}$/.test(step.uses)) fail(`Unpinned release action ${step.uses}`);
      if (String(step.uses || '').startsWith('actions/checkout@') && step.with?.['persist-credentials'] !== false) {
        fail('Release checkouts may not persist credentials');
      }
      if (/\|\|\s*(true|:)(\s|$)/m.test(String(step.run || ''))) fail(`${name} bypasses a failed command`);
      if (['sign-and-publish', 'release-channel', 'retract-failed'].includes(name) &&
          (String(step.uses || '').startsWith('actions/checkout@') ||
           /\b(cargo|npm|playwright|package-release|release-checksums)\b/.test(String(step.run || '')))) {
        fail(`${name} may not checkout, build or run the product with write credentials`);
      }
    }
  }
  const uses = (job, owner) => job.steps.find((step) => String(step.uses || '').startsWith(`${owner}@`));
  const checkout = uses(build, 'actions/checkout');
  if (checkout?.with?.['fetch-depth'] !== 0 ||
      checkout.with.ref !== '${{ github.event.workflow_run.head_sha }}') fail('Build must checkout the exact green SHA with full history');
  const upload = uses(build, 'actions/upload-artifact');
  const download = uses(signer, 'actions/download-artifact');
  if (upload?.with?.name !== 'unsigned-release-bundle' || upload.with['if-no-files-found'] !== 'error' ||
      upload.with.path.trim().split(/\s+/).sort().join(',') !== 'dist/SHA256SUMS,dist/allie-linux-x64.tar.gz' ||
      download?.with?.name !== upload.with.name || download.with.path !== 'dist') fail('Unsigned inputs must be only the archive and checksum');
  const installers = signer.steps.filter((step) => /\/cosign-installer@/.test(String(step.uses || '')));
  if (installers.length !== 1 || !installers[0].uses.startsWith('sigstore/cosign-installer@')) fail('Only the official pinned Cosign installer may sign');
  const signing = signer.steps.find((step) => /\bcosign sign-blob\b/.test(String(step.run || '')));
  if (String(signing?.run || '').trim().replace(/\s+/g, ' ') !==
      'cosign sign-blob --yes --bundle dist/allie-linux-x64.tar.gz.sigstore.json dist/allie-linux-x64.tar.gz') {
    fail('Only the expected archive may receive a real fail-closed signature');
  }
  const publish = signer.steps.find((step) => /\bgh release upload\b/.test(String(step.run || '')));
  const run = String(publish?.run || '');
  const draftAt = run.indexOf('gh api --method POST "repos/$GITHUB_REPOSITORY/releases"');
  const uploadAt = run.indexOf('gh release upload "$tag"');
  const readbackAt = run.indexOf('"repos/$GITHUB_REPOSITORY/releases/$release_id" --jq');
  const publishAt = run.indexOf('-F draft=false');
  if (!(draftAt >= 0 && uploadAt > draftAt && readbackAt > uploadAt && publishAt > readbackAt)) {
    fail('Draft, exact upload and asset readback must precede public release');
  }
  const uploaded = [...run.slice(uploadAt, run.indexOf('\n\n', uploadAt)).matchAll(/dist\/([A-Za-z0-9._-]+)/g)].map((match) => match[1]).sort();
  if (uploaded.join(',') !== 'SHA256SUMS,allie-linux-x64.tar.gz,allie-linux-x64.tar.gz.sigstore.json' ||
      !run.includes('--repo "$GITHUB_REPOSITORY"') || run.includes('--clobber') || run.includes('dist/*')) {
    fail('Publication may upload exactly the three expected assets without clobbering');
  }
  if (!run.includes('-F body=@dist/RELEASE_NOTES.md') || run.includes('generate_release_notes') || !run.includes('-f target_commitish="$RELEASE_SHA"') ||
      !run.includes('-f make_latest=false') || !run.includes('trap cleanup EXIT') ||
      !run.includes('gh api --method DELETE "repos/$GITHUB_REPOSITORY/releases/$release_id"')) {
    fail('Publication must bind the revision, keep healthy latest, and remove failed drafts');
  }
  if (run.indexOf('gh pr edit "$release_pr"') < publishAt ||
      !run.includes('.merge_commit_sha == env.RELEASE_SHA') ||
      !run.includes('.head.ref == "release-please--branches--master"') ||
      !run.includes("--remove-label 'autorelease: pending'")) {
    fail('Only successful exact-source publication may complete release-please label custody');
  }
}

function validateCiWorkflow(text) {
  const workflow = parseYaml(text);
  const audit = workflow.jobs?.['supply-chain-audit'];
  if (!audit || audit === workflow.jobs.verify) fail('Supply-chain audit must remain a distinct gate');
  exactKeys(audit.permissions, ['contents'], 'supply-chain audit permissions');
  if (audit.permissions.contents !== 'read') fail('Supply-chain audit may only read contents');
  const commands = audit.steps.map((step) => step.run).filter(Boolean);
  if (!commands.includes('cargo audit') || !commands.includes('npm audit --audit-level=high')) fail('Both dependency audits must remain enabled');
  if (audit.steps.some((step) => step['continue-on-error'] || /\|\|\s*(true|:)/.test(String(step.run || '')))) {
    fail('Supply-chain audit may not bypass failure');
  }
}

function exerciseInstallBoundary(text) {
  const block = [...text.matchAll(/```sh\n([\s\S]*?)\n```/g)].map((match) => match[1]).find((code) => code.includes('cosign verify-blob'));
  if (!block || !block.includes('--certificate-identity "https://github.com/r90group/allie/.github/workflows/release.yml@refs/heads/master"') ||
      !block.includes('--certificate-oidc-issuer https://token.actions.githubusercontent.com') ||
      block.includes('--certificate-identity-regexp')) fail('Consumer install must constrain exact signer and issuer');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'allie-install-boundary-'));
  const bin = path.join(root, 'bin');
  const marker = path.join(root, 'extracted');
  fs.mkdirSync(bin);
  const stub = (name, body) => {
    const file = path.join(bin, name);
    fs.writeFileSync(file, `#!/bin/sh\nset -eu\n${body}\n`);
    fs.chmodSync(file, 0o755);
  };
  stub('gh', `if [ "$1" = api ]; then echo v0.3.2; exit; fi
shift 3
while [ "$#" -gt 0 ]; do
  if [ "$1" = --dir ]; then download=$2; fi
  shift
done
mkdir -p "$download"
printf 'archive\\n' > "$download/allie-linux-x64.tar.gz"
printf '{}\\n' > "$download/allie-linux-x64.tar.gz.sigstore.json"
printf '%s' "$CHECKSUM_MANIFEST" > "$download/SHA256SUMS"`);
  stub('sha256sum', 'exit "$CHECKSUM_STATUS"');
  stub('cosign', 'exit "$SIGNATURE_STATUS"');
  stub('tar', ': > "$EXTRACTION_MARKER"');
  stub('allie', 'exit 0');
  const selected = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  allie-linux-x64.tar.gz\n';
  try {
    for (const [label, manifest, checksum, signature, extracts] of [
      ['verified install', selected, 0, 0, true],
      ['corrupt checksum', selected, 1, 0, false],
      ['untrusted signature', selected, 0, 1, false],
      ['missing checksum', '', 0, 0, false],
      ['duplicate checksum', selected + selected, 0, 0, false],
    ]) {
      fs.rmSync(marker, { force: true });
      const result = spawnSync('/bin/sh', ['-c', block], {
        cwd: root, encoding: 'utf8',
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, EXTRACTION_MARKER: marker,
          CHECKSUM_MANIFEST: manifest, CHECKSUM_STATUS: String(checksum), SIGNATURE_STATUS: String(signature) },
      });
      if ((result.status === 0) !== extracts || fs.existsSync(marker) !== extracts) fail(`${label} broke verify-before-extract: ${result.stderr}`);
    }
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
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
const releaseText = fs.readFileSync(RELEASE_PATH, 'utf8');
validateReleaseWorkflow(releaseText);
const trusted = new Function('github', `return (${expression(parseYaml(releaseText).jobs['select-release'].if)});`);
for (const conclusion of ['failure', 'skipped', 'cancelled', null]) {
  if (trusted({ event: { workflow_run: { event: 'push', conclusion, head_branch: 'master', head_repository: { full_name: 'r90group/allie' } } }, repository: 'r90group/allie' })) {
    fail(`Unsuccessful CI selected a release: ${conclusion}`);
  }
}
validateCiWorkflow(fs.readFileSync(CI_PATH, 'utf8'));
exerciseInstallBoundary(fs.readFileSync('README.md', 'utf8'));
const unsafeBuild = parseYaml(releaseText);
unsafeBuild.jobs['build-release'].permissions['id-token'] = 'write';
expectRejected(() => validateReleaseWorkflow(stringifyYaml(unsafeBuild)), 'build receives signing authority');
const unsafeSigner = parseYaml(releaseText);
unsafeSigner.jobs['sign-and-publish'].steps.push({ uses: 'actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5', with: { 'persist-credentials': false } });
expectRejected(() => validateReleaseWorkflow(stringifyYaml(unsafeSigner)), 'signer executes checked-out code');
const unsafeQuarantine = parseYaml(releaseText);
unsafeQuarantine.jobs['retract-failed'].concurrency = { group: 'allie-release-channel' };
expectRejected(() => validateReleaseWorkflow(stringifyYaml(unsafeQuarantine)), 'failed-candidate quarantine can be coalesced');


// Validate generator output without another version authority or file mutation.
const preparer = path.resolve('scripts/prepare-release.mjs');
const sourceCheck = spawnSync(process.execPath, [preparer, '--check'], { encoding: 'utf8' });
if (sourceCheck.status !== 0) fail(`Real source versions are invalid: ${sourceCheck.stderr}`);
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'allie-release-version-'));
const actualVersion = TOML.parse(fs.readFileSync('Cargo.toml', 'utf8')).package.version;
const inputs = Object.fromEntries(['Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json',
  '.release-please-manifest.json', 'release-please-config.json', 'CHANGELOG.md']
  .map((file) => [file, fs.readFileSync(file, 'utf8').replaceAll(actualVersion, '0.1.85')]));
const sourceVersion = TOML.parse(inputs['Cargo.toml']).package.version;
const git = (...args) => {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) fail(result.stderr);
  return result.stdout.trim();
};
const restore = () => {
  for (const [file, contents] of Object.entries(inputs)) fs.writeFileSync(path.join(root, file), contents);
};
const prepare = (mode = '--select', cwd = root) => spawnSync(process.execPath, [preparer, mode], { cwd, encoding: 'utf8' });
const expectSelection = (publish) => {
  const result = prepare();
  if (result.status !== 0 || !result.stdout.includes(`publish=${publish}`)) fail(`selection failed: ${result.stderr}`);
};
const rejectWithoutMutation = (label, mode = '--select') => {
  const before = Object.keys(inputs).map((file) => fs.readFileSync(path.join(root, file), 'utf8'));
  const result = prepare(mode);
  if (result.status === 0) fail(`${label} was accepted`);
  const after = Object.keys(inputs).map((file) => fs.readFileSync(path.join(root, file), 'utf8'));
  if (JSON.stringify(before) !== JSON.stringify(after)) fail(`${label} partially mutated release inputs`);
};
const commit = (title) => {
  git('add', '.');
  git('-c', 'user.name=Allie Release Smoke', '-c', 'user.email=allie@example.invalid',
    'commit', '-q', '--allow-empty', '-m', title);
};
const version = (value) => {
  restore();
  for (const [file, contents] of Object.entries(inputs)) {
    fs.writeFileSync(path.join(root, file), contents.replaceAll(sourceVersion, value));
  }
};
try {
  restore();
  git('init', '-q');
  commit('chore: baseline');
  rejectWithoutMutation('missing immutable baseline');
  const baseline = git('rev-parse', 'HEAD');
  commit('docs: a non-release change');
  expectSelection(false);
  version('0.1.86');
  commit('chore(master): release 0.1.86 (#123)');
  expectSelection(true);
  expectSelection(true); // Pure validation is idempotent and writes no version files.
  const notes = prepare('--notes');
  if (notes.status !== 0 || !notes.stdout.startsWith('## [0.1.86]') || notes.stdout.includes('# Changelog')) fail('canonical notes projection failed');
  git('tag', 'v0.1.86');
  expectSelection(false);
  git('tag', '-d', 'v0.1.86');
  git('tag', 'v0.1.86', baseline);
  rejectWithoutMutation('conflicting tag');
  git('tag', '-d', 'v0.1.86');
  git('reset', '--hard', baseline);
  version('1.0.0');
  commit('chore(master): release 1.0.0');
  expectSelection(true);
  git('reset', '--hard', baseline);
  version('0.1.84');
  commit('chore(master): release 0.1.84');
  rejectWithoutMutation('decreasing version');
  git('reset', '--hard', baseline);
  version('0.1.86');
  commit('fix: pretend this is a release');
  rejectWithoutMutation('false release title');
  restore();
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ version: '1.0.0' }));
  rejectWithoutMutation('mismatched worker version');
  restore();
  fs.writeFileSync(path.join(root, 'CHANGELOG.md'), '# Changelog\n\n## 0.0.1\nStale notes\n');
  rejectWithoutMutation('stale notes', '--check');
  restore();
  const shallow = path.join(root, 'shallow');
  git('clone', '--quiet', '--depth', '1', `file://${root}`, shallow);
  const shallowResult = prepare('--select', shallow);
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

console.log('release policy smoke passed: coherent SemVer/notes, no-op/repeat/conflict/decrease/stale refusal and fail-closed custody');
