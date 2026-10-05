import { test as base, expect } from '@playwright/test';
import type { Page, APIRequestContext } from '@playwright/test';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdtemp, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

interface Fixture { url: string; workspace: string; token: string; restart: () => Promise<void> }
const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const test = base.extend<{ kernel: Fixture }>({
  kernel: async ({}, use) => {
    const root = await mkdtemp(path.join(tmpdir(), 'lain-web-'));
    const workspace = path.join(root, 'Wired'); await mkdir(workspace);
    const token = randomBytes(32).toString('hex');
    const helper = spawn(process.env.PYTHON ?? 'python', [path.join(repository, 'scripts/web_fixture.py')], {
      cwd: repository, env: { ...process.env, LAIN_WEB_TEST_ROOT: root, LAIN_WEB_TEST_TOKEN: token }, windowsHide: true, stdio: 'pipe',
    });
    let output = ''; let pending: ((line: string) => void) | undefined;
    const lines: string[] = []; let errors = '';
    helper.stderr.on('data', chunk => { errors += chunk.toString(); });
    helper.stdout.on('data', chunk => {
      output += chunk.toString(); let newline: number;
      while ((newline = output.indexOf('\n')) >= 0) {
        const line = output.slice(0, newline).trim(); output = output.slice(newline + 1);
        if (pending) { const resolve = pending; pending = undefined; resolve(line); } else lines.push(line);
      }
    });
    const next = () => new Promise<string>((resolve, reject) => {
      if (lines.length) { resolve(lines.shift()!); return; }
      const timer = setTimeout(() => { pending = undefined; reject(new Error('Fake Kernel startup timed out')); }, 60_000);
      pending = line => { clearTimeout(timer); resolve(line); };
      helper.once('error', () => { clearTimeout(timer); reject(new Error('Cannot launch Python fake Kernel fixture')); });
    });
    try {
      const port = JSON.parse(await next()).port as number;
      await use({ url: `http://127.0.0.1:${port}`, workspace, token, restart: async () => {
        helper.stdin.write('restart\n'); expect(await next()).toBe('restarted');
      } });
    } finally {
      helper.stdin.end('stop\n');
      await new Promise<void>(resolve => {
        if (helper.exitCode !== null) { resolve(); return; }
        const timer = setTimeout(() => { helper.kill(); resolve(); }, 45_000);
        helper.once('exit', () => { clearTimeout(timer); resolve(); });
      });
      expect(errors.includes(token)).toBe(false);
      expect(helper.exitCode).toBe(0);
      if (path.dirname(path.resolve(root)) !== path.resolve(tmpdir()) || !path.basename(root).startsWith('lain-web-')) {
        throw new Error('Refusing cleanup outside the private browser-test directory');
      }
      await rm(root, { recursive: true, force: true });
    }
  },
});
async function connect(page: Page, kernel: Fixture) {
  await page.goto(kernel.url);
  await page.getByLabel('Kernel API token').fill(kernel.token);
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.getByRole('status').first()).toContainText('Connected · Kernel Ready');
}
async function call(request: APIRequestContext, kernel: Fixture, url: string, body?: unknown) {
  const response = body === undefined
    ? await request.get(kernel.url + url, { headers: { Authorization: `Bearer ${kernel.token}` } })
    : await request.post(kernel.url + url, { headers: { Authorization: `Bearer ${kernel.token}` }, data: body });
  expect(response.ok()).toBe(true); return response.json();
}
async function send(page: Page, text: string) {
  await page.getByLabel('Follow-up instruction').fill(text);
  await page.getByRole('button', { name: 'Send instruction' }).click();
}

test('persistent browser work: creation, live history, decisions, closure, restart and detached follow-up', async ({ page, context, request, kernel }, testInfo) => {
  const pageErrors: string[] = []; const consoleErrors: string[] = [];
  const inspect = (target: Page) => {
    target.on('pageerror', error => pageErrors.push(error.message));
    target.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
  };
  inspect(page); context.on('page', inspect);
  await page.goto(kernel.url);
  await page.getByLabel('Kernel API token').fill('invalid-credentials-0123456789abcdef');
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.getByRole('alert')).toContainText('Credentials invalid');
  await connect(page, kernel);
  await page.getByRole('button', { name: '+ New Work' }).click();
  await page.getByLabel('Workspace on daemon host').fill(kernel.workspace);
  await page.getByLabel('Model (optional)').fill('fixture-model');
  await page.getByRole('button', { name: 'Create WorkSession' }).click();
  await expect(page.getByRole('heading', { name: 'Wired', exact: true })).toBeVisible();
  await expect(page.locator('.timeline')).toContainText('WorkSession created');
  const sid = new URL(page.url()).hash.slice(1); expect(sid).toMatch(/^[a-f\d-]{36}$/);
  const url = `/v1/work-sessions/${sid}`;
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  const observers = await context.newPage(); await connect(observers, kernel);
  await observers.getByRole('button', { name: /Wired/ }).click();
  await expect(observers.locator('.detail-header')).toContainText('Runtime detached');
  expect((await call(request, kernel, '/v1/work-sessions')).length).toBe(1);
  expect((await call(request, kernel, url)).runtime).toBe('detached');
  await observers.close();
  await send(page, 'history-output');
  await expect(page.locator('.session-detail > .metadata')).toContainText('Completed');
  await expect(page.locator('.timeline')).toContainText('$ echo safe');
  await page.getByText('Command output', { exact: true }).first().click();
  await expect(page.locator('.output').first()).toBeVisible();
  await expect(page.locator('.timeline')).toContainText('file-0.txt');
  const initialHistory = await call(request, kernel, url + '/history?limit=200');
  await send(page, 'approve');
  const card = page.getByRole('article', { name: 'Command approval' });
  await expect(card).toBeVisible(); await expect(card).toContainText('echo fixture');
  await card.getByRole('button', { name: 'Deny', exact: true }).click();
  await expect(card).toHaveCount(0);
  await expect.poll(async () => (await call(request, kernel, url + '/approvals'))[0].decision).toBe('deny');
  await send(page, 'permissions');
  const permission = page.getByRole('article', { name: 'Permissions approval' });
  await expect(permission).toBeVisible(); await expect(permission.getByRole('button', { name: 'Cancel', exact: true })).toHaveCount(0);
  await permission.getByRole('button', { name: 'Approve once' }).click(); await expect(permission).toHaveCount(0);
  await send(page, 'wait');
  await expect(page.getByRole('button', { name: 'Cancel work', exact: true })).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Send instruction' })).toBeDisabled();
  await page.getByRole('button', { name: 'Cancel work', exact: true }).click();
  await expect(page.locator('.session-detail > .metadata')).toContainText('Cancelled');
  await send(page, '<img src=x onerror="globalThis.injected=true">\u001b[31m\u202e');
  await expect(page.locator('.assistant-text').last()).toContainText('<img');
  expect(await page.evaluate(() => 'injected' in globalThis)).toBe(false);
  expect(await page.locator('.timeline img').count()).toBe(0);
  await expect(page.locator('.assistant-text').last()).toContainText('\\u001b');
  await page.screenshot({ path: testInfo.outputPath('desktop.png'), fullPage: true });
  for (const viewport of [{ width: 1280, height: 720 }, { width: 820, height: 1180 }]) {
    await page.setViewportSize(viewport);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await expect(page.getByLabel('Follow-up instruction')).toBeVisible();
  }
  await page.setViewportSize({ width: 1440, height: 1000 });

  await send(page, 'wait');
  await expect(page.getByRole('button', { name: 'Cancel work', exact: true })).toBeEnabled();
  await page.close();
  expect((await call(request, kernel, url)).state).toBe('running');
  await call(request, kernel, url + '/cancel', {});
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  const returned = await context.newPage(); await connect(returned, kernel);
  await returned.getByRole('button', { name: /Wired/ }).click();
  await expect(returned.locator('.session-detail > .metadata')).toContainText('Cancelled');
  for (const event of initialHistory.events) await expect(returned.locator(`[data-event-id="${event.id}"]`)).toHaveCount(1);
  await kernel.restart();
  await expect(returned.getByRole('status').first()).toContainText('Connected · Kernel Ready', { timeout: 20_000 });
  await expect(returned.locator('.detail-header')).toContainText('Runtime detached');
  await send(returned, 'browser-followup');
  await expect(returned.locator('.assistant-text').last()).toHaveText('browser-followup');
  await returned.reload();
  await expect(returned.getByLabel('Kernel API token')).toBeVisible();
  await returned.getByLabel('Kernel API token').fill(kernel.token);
  await returned.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(returned.locator('.assistant-text').last()).toHaveText('browser-followup');
  expect((await call(request, kernel, '/v1/work-sessions')).length).toBe(1);
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  await returned.setViewportSize({ width: 390, height: 844 });
  await expect(returned.getByRole('button', { name: '← Work sessions' })).toBeVisible();
  expect(await returned.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await send(returned, 'mobile-followup');
  await expect(returned.locator('.assistant-text').last()).toHaveText('mobile-followup');
  await send(returned, 'approve');
  await expect(returned.getByRole('article', { name: 'Command approval' })).toBeVisible();
  await returned.screenshot({ path: testInfo.outputPath('mobile.png'), fullPage: true });
  await returned.getByRole('article', { name: 'Command approval' }).getByRole('button', { name: 'Deny', exact: true }).click();
  await returned.getByRole('button', { name: '← Work sessions' }).click();
  await expect(returned.getByRole('button', { name: /Wired/ })).toBeVisible();
  expect(await returned.evaluate(() => ({ local: localStorage.length, session: sessionStorage.length, cookie: document.cookie }))).toEqual({ local: 0, session: 0, cookie: '' });
  const body = await returned.locator('body').innerText(); expect(body.includes(kernel.token)).toBe(false);
  expect(pageErrors).toEqual([]); expect(consoleErrors.some(message => message.includes(kernel.token))).toBe(false);
  expect(consoleErrors.some(message => /Content Security Policy|Refused to/i.test(message))).toBe(false);
});

test('history pagination, missed live records, Unknown reconciliation and reconnect', async ({ page, context, request, kernel }) => {
  const session = await call(request, kernel, '/v1/work-sessions', { workspace: kernel.workspace });
  const url = `/v1/work-sessions/${session.id}`;
  // Deterministic output events create a history larger than the initial window.
  for (let i = 0; i < 35; i++) {
    await call(request, kernel, url + '/turns', { text: `history-${i}` });
    await expect.poll(async () => (await call(request, kernel, url)).state).toBe('idle');
  }
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  await connect(page, kernel);
  await call(request, kernel, url + '/turns', { text: 'while-unselected' });
  await expect.poll(async () => (await call(request, kernel, url)).state).toBe('idle');
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  await page.getByRole('button', { name: /Wired/ }).click();
  await expect(page.locator('.assistant-text').last()).toHaveText('while-unselected');
  await expect(page.locator('.events > li')).toHaveCount(200);
  await page.getByRole('button', { name: 'Load older history' }).click();
  await expect(page.locator('.events > li').first()).toContainText('WorkSession created');
  await page.getByRole('button', { name: 'Latest activity' }).click();
  await expect(page.locator('.events > li')).toHaveCount(200);
  await context.setOffline(true);
  await call(request, kernel, url + '/turns', { text: 'missed-while-offline' });
  await expect.poll(async () => (await call(request, kernel, url)).state).toBe('idle');
  await context.setOffline(false);
  await expect(page.locator('.assistant-text').last()).toHaveText('missed-while-offline', { timeout: 20_000 });
  const ids = await page.locator('[data-event-id]').evaluateAll(elements => elements.map(e => e.getAttribute('data-event-id')));
  expect(new Set(ids).size).toBe(ids.length);
  await send(page, 'crash'); await expect(page.getByRole('button', { name: 'Cancel work', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Cancel work', exact: true }).click();
  await expect(page.locator('.detail-header')).toContainText('Unknown');
  await expect(page.locator('.state-explanation').first()).toContainText('lost reliable knowledge');
  await expect(page.locator('.detail-header')).not.toContainText('Failed');
  await page.getByRole('button', { name: 'Resume / reconcile' }).click();
  await expect(page.locator('.detail-header')).toContainText('Idle');
  await send(page, 'reconciled-followup');
  await expect(page.locator('.assistant-text').last()).toHaveText('reconciled-followup');
});

test('unreachable Kernel, readiness and backend availability have explicit independent UI states', async ({ page, kernel }) => {
  await page.route('**/health', route => route.abort('connectionrefused'));
  await page.goto(kernel.url);
  await page.getByLabel('Kernel API token').fill(kernel.token);
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.getByRole('alert')).toContainText('Kernel unreachable');
  await expect(page.getByLabel('Kernel API token')).toHaveValue('');
  await page.unroute('**/health');
  await page.route('**/ready', route => route.fulfill({ status: 503, contentType: 'application/json', body: '{"state":"recovering"}' }));
  await page.route('**/v1/backends', route => route.fulfill({ contentType: 'application/json', body: '[{"backend":"codex","version":null,"available":false,"capabilities":[]}]' }));
  await page.getByLabel('Kernel API token').fill(kernel.token);
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.getByRole('status').first()).toContainText('Connected · Kernel Not ready');
  await expect(page.locator('.backend-status')).toHaveText('Codex unavailable');
  await page.getByRole('button', { name: '+ New Work' }).click();
  await expect(page.getByRole('button', { name: 'Create WorkSession' })).toBeDisabled();
  await expect(page.getByText('Worker backend unavailable. Existing history remains readable.')).toBeVisible();
  await expect(page.getByLabel('Model (optional)')).toHaveCount(0);
});

test('profile selection and frozen revision survive registry edits, detach and browser refresh', async ({ page, request, kernel }) => {
  const profile = { id: 'codex/browser-b', backend: 'codex', driver: 'app-server', enabled: true, model: 'browser-model-b', provider: null, base_url: null, reasoning_effort: 'high', sandbox_default: 'workspace_write', approval_policy: 'on_request', secret_ref: null };
  await call(request, kernel, '/v1/runtime-profiles', profile);
  await connect(page, kernel);
  await page.getByRole('button', { name: '+ New Work' }).click();
  await page.getByLabel('Workspace on daemon host').fill(kernel.workspace);
  await page.getByLabel('Runtime profile', { exact: true }).selectOption(profile.id);
  await expect(page.locator('.new-work')).toContainText('browser-model-b');
  await expect(page.getByLabel('Sandbox', { exact: true })).toHaveValue('');
  await page.getByRole('button', { name: 'Create WorkSession' }).click();
  await expect(page.getByText('Frozen profile · codex/browser-b / revision 1', { exact: true })).toBeVisible();
  const sid = new URL(page.url()).hash.slice(1);
  const url = `/v1/work-sessions/${sid}`;
  await page.getByText('Frozen profile · codex/browser-b / revision 1', { exact: true }).click();
  await expect(page.locator('.session-detail')).toContainText('browser-model-b');
  const original = await call(request, kernel, url + '/resolved-profile');
  expect(original.sandbox).toBe('workspace_write');
  await send(page, 'wait');
  await expect(page.getByRole('button', { name: 'Cancel work', exact: true })).toBeEnabled();
  const edit = await request.put(kernel.url + '/v1/runtime-profiles', { headers: { Authorization: `Bearer ${kernel.token}` }, data: { definition: { ...profile, model: 'browser-model-new' }, expected_revision: 1 } });
  expect(edit.ok()).toBe(true);
  expect((await call(request, kernel, url)).state).toBe('running');
  expect(await call(request, kernel, url + '/resolved-profile')).toEqual(original);
  await page.getByRole('button', { name: 'Cancel work', exact: true }).click();
  await expect.poll(async () => (await call(request, kernel, url)).runtime).toBe('detached');
  await send(page, 'retry-success');
  await expect(page.locator('.events')).toContainText('backend will retry');
  await expect(page.locator('.events')).toContainText('Turn Completed');
  await page.reload(); await page.getByLabel('Kernel API token').fill(kernel.token);
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.getByText('Frozen profile · codex/browser-b / revision 1', { exact: true })).toBeVisible();
  expect(await call(request, kernel, url + '/resolved-profile')).toEqual(original);
  await send(page, 'auth-failure');
  await expect(page.locator('.session-detail')).toContainText('worker authentication failed');
  await page.reload(); await page.getByLabel('Kernel API token').fill(kernel.token);
  await page.getByRole('button', { name: 'Connect to Kernel' }).click();
  await expect(page.locator('.state-explanation').last()).toContainText('worker authentication failed');
});
