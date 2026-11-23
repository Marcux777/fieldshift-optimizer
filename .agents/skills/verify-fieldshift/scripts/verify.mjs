// AI-assisted (OpenBot agent)
import { createServer } from 'node:net';
import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync
} from 'node:fs';
import { createHash } from 'node:crypto';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CheckError, command, delay, launch, stop } from './process.mjs';
import { grep, selection } from './flows.mjs';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../..');

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function parseArgs(argv) {
  const opts = { mission: null, flows: 'main', fixture: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--mission') opts.mission = argv[++i];
    else if (a === '--flows') opts.flows = argv[++i];
    else if (a === '--fixture') opts.fixture = argv[++i];
    else throw new CheckError(`Argumento desconhecido: ${a}`, 2);
  }
  if (!opts.mission || !/^[A-Za-z0-9._-]+$/.test(opts.mission)) {
    throw new CheckError('Informe --mission <id-valido>', 2);
  }
  if (opts.fixture !== null && !['A', 'B', 'C'].includes(opts.fixture)) {
    throw new CheckError('Fixture deve ser A, B ou C', 2);
  }
  opts.flowList = selection(opts.flows);
  return opts;
}

function getFreePort() {
  return new Promise((resolvePort, reject) => {
    const srv = createServer();
    srv.listen(0, '127.0.0.1', () => {
      const addr = srv.address();
      const port = typeof addr === 'object' && addr ? addr.port : 0;
      srv.close(() => resolvePort(port));
    });
    srv.on('error', reject);
  });
}

async function waitForHealth(url, deadlineMs, apiChild) {
  while (Date.now() < deadlineMs) {
    let exited = false;
    await Promise.race([apiChild.result.then(() => { exited = true; }), delay(50)]);
    if (exited) {
      throw new CheckError('fieldshift-api encerrou prematuramente durante o startup', 1);
    }
    try {
      const resp = await fetch(`${url}/v1/health`);
      if (resp.ok) return;
    } catch {
      // aguarda servidor subir
    }
    await delay(150);
  }
  throw new CheckError(`Readiness timeout aguardando ${url}/v1/health`, 1);
}

function checkPrerequisites(fieldshiftRoot, apiBin) {
  if (!existsSync(apiBin)) {
    throw new CheckError(`Binário da API ausente: ${apiBin}`, 3);
  }
  const currentTxt = join(fieldshiftRoot, 'data', 'snapshots', 'current.txt');
  const cfgDir = join(fieldshiftRoot, 'config');
  if (!existsSync(cfgDir) || !existsSync(currentTxt)) {
    throw new CheckError(
      `Pré-requisito de dados ausente em FIELDSHIFT_ROOT (${fieldshiftRoot}): esperado config/ e data/snapshots/current.txt`,
      3
    );
  }
  const snapName = readFileSync(currentTxt, 'utf-8').trim();
  const snapDir = join(fieldshiftRoot, 'data', 'snapshots', snapName);
  const requiredFiles = ['manifest.json', 'SHA256SUMS'];
  for (const f of requiredFiles) {
    if (!existsSync(join(snapDir, f))) {
      throw new CheckError(`Arquivo obrigatório do snapshot ausente: ${join(snapDir, f)}`, 3);
    }
  }
}

async function main() {
  const startTime = Date.now();
  const totalDeadline = startTime + 3600_000;
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (err) {
    console.error(err.message);
    process.exit(2);
  }

  const evidenceDir = join(REPO_ROOT, '.vsdd', 'evidence', opts.mission);
  if (existsSync(evidenceDir)) {
    console.error(`Colisão de mission-id: ${evidenceDir} já existe`);
    process.exit(2);
  }
  mkdirSync(evidenceDir, { recursive: true });

  const scratchDir = join('/tmp', `fieldshift-verify-${opts.mission}-${process.pid}`);
  mkdirSync(scratchDir, { recursive: true });

  const children = [];
  const steps = [];
  let status = 'PASS';
  let exitCode = 0;
  let failureReason = null;

  try {
    const apiBin = join(REPO_ROOT, 'backend', 'target', 'release', 'fieldshift-api');
    let staticDir = join(REPO_ROOT, 'frontend', 'build');
    let fieldshiftRoot = process.env.FIELDSHIFT_ROOT
      ? resolve(process.env.FIELDSHIFT_ROOT)
      : REPO_ROOT;

    if (opts.fixture === 'C') {
      const cRoot = join(scratchDir, 'root-missing-snapshot');
      mkdirSync(cRoot, { recursive: true });
      cpSync(join(REPO_ROOT, 'config'), join(cRoot, 'config'), { recursive: true });
      fieldshiftRoot = cRoot;
    }

    // 1. Doctor / Prerequisite check
    checkPrerequisites(fieldshiftRoot, apiBin);
    steps.push({ step: 'doctor', status: 'ok', fieldshiftRoot });

    // 2. Build / Mutation overlay para Fixture A
    if (opts.fixture === 'A') {
      const frontendCopy = join(scratchDir, 'frontend');
      mkdirSync(frontendCopy, { recursive: true });
      for (const item of [
        'package.json',
        'package-lock.json',
        'svelte.config.js',
        'vite.config.ts',
        'tsconfig.json',
        'src',
        'static',
        '.svelte-kit',
        'node_modules'
      ]) {
        const srcItem = join(REPO_ROOT, 'frontend', item);
        if (!existsSync(srcItem)) continue;
        cpSync(srcItem, join(frontendCopy, item), {
          recursive: true,
          verbatimSymlinks: true
        });
      }
      const clientTs = join(frontendCopy, 'src', 'lib', 'api', 'client.ts');
      const origText = readFileSync(clientTs, 'utf-8');
      const mutatedText = origText.replace(
        'Frozen evaluation outcome: NEGATIVE',
        'Frozen evaluation outcome: POSITIVE'
      );
      if (origText === mutatedText) {
        throw new CheckError('Mutação da Fixture A não encontrou o texto-alvo em client.ts', 1);
      }
      writeFileSync(clientTs, mutatedText, 'utf-8');

      const buildLog = join(evidenceDir, 'fixture-a-frontend-build.log');
      await command(
        'npm',
        ['run', 'build'],
        { cwd: frontendCopy, env: { ...process.env } },
        buildLog,
        children,
        Math.min(totalDeadline, Date.now() + 300_000)
      );
      staticDir = join(frontendCopy, 'build');
      steps.push({ step: 'fixture-a-build', status: 'ok', staticDir });
    } else if (!existsSync(join(staticDir, 'index.html'))) {
      const buildLog = join(evidenceDir, 'frontend-build.log');
      await command(
        'npm',
        ['--prefix', join(REPO_ROOT, 'frontend'), 'run', 'build'],
        { cwd: REPO_ROOT, env: { ...process.env } },
        buildLog,
        children,
        Math.min(totalDeadline, Date.now() + 300_000)
      );
      steps.push({ step: 'frontend-build', status: 'ok', staticDir });
    }

    // 3. Launch API Rust em porta livre
    const port = await getFreePort();
    const baseUrl = `http://127.0.0.1:${port}`;
    const apiLog = join(evidenceDir, 'api.log');
    const apiChild = launch(
      apiBin,
      [],
      {
        cwd: REPO_ROOT,
        env: {
          PATH: process.env.PATH,
          HOME: process.env.HOME,
          FIELDSHIFT_ROOT: fieldshiftRoot,
          FIELDSHIFT_STATIC_DIR: staticDir,
          BIND_ADDRESS: `127.0.0.1:${port}`,
          FRONTEND_ORIGIN: baseUrl
        }
      },
      apiLog,
      children
    );

    await waitForHealth(baseUrl, Math.min(totalDeadline, Date.now() + 90_000), apiChild);
    steps.push({ step: 'launch-api', status: 'ok', pid: apiChild.pid, baseUrl });

    // 4. Drive Playwright nos fluxos selecionados
    const pwLog = join(evidenceDir, 'playwright.log');
    const pwBin = join(REPO_ROOT, 'frontend', 'node_modules', '.bin', 'playwright');
    if (!existsSync(pwBin)) {
      throw new CheckError(`Playwright ausente: ${pwBin}`, 3);
    }
    const pattern = grep(opts.flowList);
    await command(
      pwBin,
      [
        'test',
        '--config',
        join(__dirname, 'playwright.config.mjs'),
        '--grep',
        pattern
      ],
      {
        cwd: join(REPO_ROOT, 'frontend'),
        env: {
          ...process.env,
          SHIELD_TEST_DIR: join(REPO_ROOT, 'frontend', 'e2e'),
          SHIELD_EVIDENCE: evidenceDir,
          SHIELD_URL: baseUrl
        }
      },
      pwLog,
      children,
      Math.min(totalDeadline, Date.now() + 600_000)
    );
    steps.push({ step: 'drive-playwright', status: 'ok', flows: opts.flowList });
  } catch (err) {
    exitCode = err instanceof CheckError ? err.code : 1;
    status = exitCode === 3 ? 'BLOCKED' : exitCode === 2 ? 'USAGE_ERROR' : 'FAIL';
    failureReason = err.message;
    steps.push({ step: 'error', status, exitCode, message: err.message });
  } finally {
    const cleanupResults = [];
    for (const child of children.reverse()) {
      try {
        const r = await stop(child);
        cleanupResults.push(r);
        if (!r.gone && exitCode === 0) {
          exitCode = 1;
          status = 'FAIL';
          failureReason = `Falha de teardown no PID ${r.pid}`;
        }
      } catch (e) {
        cleanupResults.push({ pid: child.pid, error: e.message });
      }
    }
    try {
      rmSync(scratchDir, { recursive: true, force: true });
    } catch {
      // ignora erro no scratch
    }

    const resultPayload = {
      mission: opts.mission,
      fixture: opts.fixture,
      flows: opts.flowList,
      status,
      exit_code: exitCode,
      failure_reason: failureReason,
      duration_ms: Date.now() - startTime,
      api_binary_sha256: existsSync(join(REPO_ROOT, 'backend/target/release/fieldshift-api'))
        ? sha256File(join(REPO_ROOT, 'backend/target/release/fieldshift-api'))
        : null,
      steps,
      cleanup: cleanupResults
    };
    writeFileSync(
      join(evidenceDir, 'result.json'),
      JSON.stringify(resultPayload, null, 2),
      'utf-8'
    );
    console.log(JSON.stringify(resultPayload));
  }
  process.exit(exitCode);
}

main();
