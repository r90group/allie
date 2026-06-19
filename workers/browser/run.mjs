#!/usr/bin/env node
import AxeBuilder from '@axe-core/playwright';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const WORKER_REQUEST_SCHEMA = 'allie.worker.request.v0';
const WORKER_RESPONSE_SCHEMA = 'allie.worker.response.v0';
const moduleDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(moduleDir, '../..');

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.smoke) {
    await runSmoke(args.smoke);
    return;
  }

  if (!args.request || !args.response) {
    throw new Error('Usage: node workers/browser/run.mjs --request <request.json> --response <response.json>');
  }

  const request = await readJson(args.request);
  const response = await runWorker(request);
  await writeJson(args.response, response);
}

async function runSmoke(outDir) {
  const smokeOut = path.resolve(repoRoot, outDir);
  const smokeRequestPath = path.resolve(repoRoot, 'workers/browser/smoke-request.json');
  const request = await readJson(smokeRequestPath);
  request.artifacts_dir = path.join(smokeOut, 'artifacts');

  await fs.mkdir(smokeOut, { recursive: true });
  await writeJson(path.join(smokeOut, 'worker-request.json'), request);

  const response = await runWorker(request);
  await writeJson(path.join(smokeOut, 'worker-response.json'), response);

  if (response.status !== 'passed') {
    console.error(JSON.stringify(response, null, 2));
    process.exitCode = 1;
  } else {
    console.log(`worker smoke passed: ${path.join(smokeOut, 'worker-response.json')}`);
  }
}

async function runWorker(request) {
  if (request.schema !== WORKER_REQUEST_SCHEMA) {
    return errorResponse(`unexpected request schema ${request.schema}`);
  }

  let fixtureServer = null;
  let browser = null;

  try {
    const artifactsDir = path.resolve(repoRoot, request.artifacts_dir);
    await fs.mkdir(artifactsDir, { recursive: true });

    const target = await resolveTarget(request.target);
    fixtureServer = target.server;

    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
      viewport: request.browser.viewport,
      colorScheme: request.browser.color_scheme,
      reducedMotion: request.browser.reduced_motion,
      locale: request.browser.locale,
    });

    const states = [];
    for (const state of request.states) {
      states.push(await inspectState(context, target.baseUrl, state, artifactsDir, request.browser.zoom));
    }

    await context.close();
    await browser.close();
    browser = null;

    const hasViolations = states.some((state) => state.axe_violations.length > 0);
    const hasStateErrors = states.some((state) => state.state_errors.length > 0);
    return {
      schema: WORKER_RESPONSE_SCHEMA,
      status: hasViolations || hasStateErrors ? 'failed' : 'passed',
      actual_base_url: target.baseUrl,
      states,
      errors: [],
    };
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error));
  } finally {
    if (browser) {
      await browser.close().catch(() => {});
    }
    if (fixtureServer) {
      await closeServer(fixtureServer).catch(() => {});
    }
  }
}

async function inspectState(context, baseUrl, state, artifactsDir, zoom) {
  const page = await context.newPage();
  const consoleErrors = [];
  const networkErrors = [];

  page.on('console', (message) => {
    if (message.type() === 'error') {
      consoleErrors.push(message.text());
    }
  });

  page.on('requestfailed', (request) => {
    networkErrors.push(`${request.method()} ${request.url()} ${request.failure()?.errorText ?? 'failed'}`);
  });

  const targetUrl = new URL(state.path, baseUrl).toString();
  const navigationResponse = await page.goto(targetUrl, { waitUntil: 'networkidle' });
  const httpStatus = navigationResponse?.status() ?? null;
  const stateErrors = [];

  if (state.required && httpStatus !== null && (httpStatus < 200 || httpStatus >= 400)) {
    stateErrors.push(`required route returned HTTP ${httpStatus}`);
  }

  if (zoom && zoom !== 1) {
    await page.evaluate((value) => {
      document.documentElement.style.zoom = String(value);
    }, zoom);
  }

  const title = await page.title();
  const screenshotPath = state.screenshot ? path.join(artifactsDir, `${state.id}.png`) : null;
  if (screenshotPath) {
    await page.screenshot({ path: screenshotPath, fullPage: true });
  }

  let axeJsonPath = null;
  let axeViolations = [];
  if (state.axe) {
    const axeResult = await new AxeBuilder({ page }).analyze();
    axeJsonPath = path.join(artifactsDir, `axe-${state.id}.json`);
    await fs.writeFile(axeJsonPath, `${JSON.stringify(axeResult, null, 2)}\n`);
    axeViolations = axeResult.violations.map((violation) => ({
      id: violation.id,
      impact: violation.impact ?? null,
      help: violation.help ?? null,
      description: violation.description ?? null,
      tags: violation.tags ?? [],
      nodes: violation.nodes?.length ?? 0,
    }));
  }

  await page.close();

  return {
    id: state.id,
    route: state.path,
    url: targetUrl,
    title,
    http_status: httpStatus,
    screenshot_path: screenshotPath ? path.relative(path.resolve(repoRoot, path.dirname(path.dirname(screenshotPath))), screenshotPath) : null,
    axe_json_path: axeJsonPath ? path.relative(path.resolve(repoRoot, path.dirname(path.dirname(axeJsonPath))), axeJsonPath) : null,
    axe_violations: axeViolations,
    console_errors: consoleErrors,
    network_errors: networkErrors,
    state_errors: stateErrors,
  };
}

async function resolveTarget(target) {
  if (target.kind === 'local_fixture') {
    if (!target.fixture_dir) {
      throw new Error('local_fixture target requires fixture_dir');
    }
    const fixtureDir = path.resolve(repoRoot, target.fixture_dir);
    const server = await startFixtureServer(fixtureDir);
    const { port } = server.address();
    return {
      baseUrl: `http://127.0.0.1:${port}/`,
      server,
    };
  }

  if (!target.base_url) {
    throw new Error('non-fixture target requires base_url');
  }

  return {
    baseUrl: target.base_url,
    server: null,
  };
}

async function startFixtureServer(fixtureDir) {
  const root = await fs.realpath(fixtureDir);
  const rootWithSeparator = root.endsWith(path.sep) ? root : `${root}${path.sep}`;

  const server = http.createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
      const relativePath = requestUrl.pathname === '/'
        ? 'index.html'
        : decodeURIComponent(requestUrl.pathname).replace(/^\/+/, '');
      const candidate = path.resolve(root, relativePath);

      if (candidate !== root && !candidate.startsWith(rootWithSeparator)) {
        response.writeHead(403);
        response.end('Forbidden');
        return;
      }

      const bytes = await fs.readFile(candidate);
      response.writeHead(200, { 'content-type': contentType(candidate) });
      response.end(bytes);
    } catch {
      response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
      response.end('Not found');
    }
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });

  return server;
}

function contentType(filePath) {
  switch (path.extname(filePath)) {
    case '.html':
      return 'text/html; charset=utf-8';
    case '.css':
      return 'text/css; charset=utf-8';
    case '.js':
      return 'text/javascript; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    case '.png':
      return 'image/png';
    case '.svg':
      return 'image/svg+xml';
    default:
      return 'application/octet-stream';
  }
}

async function closeServer(server) {
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

function errorResponse(message) {
  return {
    schema: WORKER_RESPONSE_SCHEMA,
    status: 'error',
    actual_base_url: null,
    states: [],
    errors: [message],
  };
}

async function readJson(filePath) {
  const text = await fs.readFile(path.resolve(repoRoot, filePath), 'utf8');
  return JSON.parse(text);
}

async function writeJson(filePath, value) {
  const resolved = path.resolve(repoRoot, filePath);
  await fs.mkdir(path.dirname(resolved), { recursive: true });
  await fs.writeFile(resolved, `${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--request') {
      parsed.request = args[++index];
    } else if (arg === '--response') {
      parsed.response = args[++index];
    } else if (arg === '--smoke') {
      parsed.smoke = args[++index];
    } else {
      throw new Error(`unexpected argument: ${arg}`);
    }
  }
  return parsed;
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : error);
  process.exit(2);
});
