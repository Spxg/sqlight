import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { checkEditor } from './editor-checks.mjs';
import { checkEngineSelection, checkWorkers } from './engine-checks.mjs';
import { checkExecutionProgress } from './loading-checks.mjs';
import { checkInteractions } from './interaction-checks.mjs';
import { checkBatchFailures, checkResultPaging } from './result-checks.mjs';
import { checkSharing } from './share-checks.mjs';
import { checkWorkspaces } from './workspace-checks.mjs';
import { checkWorkspaceMenuPosition } from './menu-checks.mjs';
import { checkArchives } from './archive-checks.mjs';
import { checkFiles } from './files-checks.mjs';
import { chooseFiles, downloadDatabase } from './files-helpers.mjs';

const dist = process.argv[2]
  ? path.resolve(process.argv[2])
  : fileURLToPath(new URL('../dist/', import.meta.url));
const types = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.wasm': 'application/wasm',
  '.woff2': 'font/woff2',
};
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://localhost').pathname;
  const filename = path.join(dist, pathname === '/' ? 'index.html' : pathname);
  try {
    const data = await readFile(filename);
    response.writeHead(200, {
      'Content-Type':
        types[path.extname(filename)] || 'application/octet-stream',
    });
    response.end(data);
  } catch {
    response.writeHead(404);
    response.end();
  }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1440, height: 960 },
});
await context.grantPermissions(['clipboard-read', 'clipboard-write']);
await context.route('**/*', (route) => {
  return route.request().url().startsWith(origin)
    ? route.continue()
    : route.abort();
});
const page = await context.newPage();
page.setDefaultTimeout(20_000);
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
page.on('response', (response) => {
  if (!response.ok()) errors.push(`${response.status()} ${response.url()}`);
});

async function edit(sql) {
  await page.evaluate(
    (sql) => globalThis.ace.edit('ace_editor').setValue(sql, -1),
    sql,
  );
}

async function run(sql) {
  await edit(sql);
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page
    .getByRole('tabpanel')
    .getByText('Finished', { exact: true })
    .waitFor();
}

async function closeMenu() {
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'hidden' });
}

try {
  await page.goto(origin);
  await page.waitForFunction(() =>
    globalThis.ace?.edit('ace_editor').getValue().includes('demo_books'),
  );
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.getByText('Finished', { exact: true }).waitFor();
  await page.waitForFunction(() =>
    JSON.parse(localStorage.getItem('config'))?.database === 'default.db',
  );
  const exampleTables = page.getByRole('table');
  assert.equal(await exampleTables.count(), 2);
  assert.equal(await exampleTables.first().locator('tbody tr').count(), 4);
  assert.deepEqual(
    await exampleTables
      .last()
      .locator('tbody tr')
      .last()
      .locator('td')
      .allTextContents(),
    ["'Small, Fast, Reliable'", '0', '0'],
  );
  await checkExecutionProgress(page);

  // Changes are saved even when the SQL has never been run.
  await edit("SELECT 'saved without running' AS message;");
  await page.waitForFunction(() =>
    JSON.parse(localStorage.getItem('config'))?.sql.includes(
      'saved without running',
    ),
  );
  await page.reload();
  await page.waitForFunction(() =>
    globalThis.ace
      ?.edit('ace_editor')
      .getValue()
      .includes('saved without running'),
  );

  // A failed execution must stop loading and allow another execution.
  await edit('SELECT FROM;');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.getByRole('tab', { name: 'Status', selected: true }).waitFor();
  assert.equal(
    await page.getByRole('button', { name: 'Run', exact: true }).isEnabled(),
    true,
  );
  await run(
    "SELECT x'' AS empty_blob, '' AS empty_text, NULL AS null_value, 'it''s SQL' AS quoted;",
  );
  assert.deepEqual(await page.locator('tbody td').allTextContents(), [
    "x''",
    "''",
    'NULL',
    "'it''s SQL'",
  ]);
  await run('CREATE TABLE empty_table(id INTEGER); SELECT * FROM empty_table;');
  assert.equal(
    await page.getByRole('columnheader', { name: 'id', exact: true }).count(),
    1,
  );
  assert.equal(await page.locator('tbody tr').count(), 0);

  // Theme and layout preferences react immediately and follow system changes.
  await page.getByRole('button', { name: 'Config', exact: true }).click();
  await page
    .getByRole('combobox', { name: 'Theme', exact: true })
    .selectOption('Dark');
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === 'dark',
  );
  await page
    .getByRole('combobox', { name: 'Theme', exact: true })
    .selectOption('System');
  await page.emulateMedia({ colorScheme: 'light' });
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === 'light',
  );
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === 'dark',
  );
  await page
    .getByRole('combobox', { name: 'Orientation', exact: true })
    .selectOption('Vertical');
  await closeMenu();
  const editorBox = await page.locator('#ace_editor').boundingBox();
  const outputBox = await page.getByRole('tabpanel').boundingBox();
  assert.ok(
    outputBox.x > editorBox.x + editorBox.width,
    'Vertical places output beside the editor',
  );

  // Share stays current while its panel is already open.
  await edit("SELECT 'first share';");
  await page.getByRole('button', { name: 'Share', exact: true }).click();
  await edit("SELECT 'second share';");
  await page.getByRole('button', { name: 'Share', exact: true }).click();
  const shareLink = page.getByRole('link', { name: 'Embedded code in link' });
  const href = await shareLink.getAttribute('href');
  assert.equal(
    new URL(href).searchParams.get('code'),
    "SELECT 'second share';",
  );
  await page.getByRole('button', { name: 'Copy share link' }).click();
  await page.getByText('Copied!', { exact: true }).waitFor();
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), href);
  const sharedPage = await context.newPage();
  await sharedPage.goto(href);
  await sharedPage.waitForFunction(
    () =>
      globalThis.ace?.edit('ace_editor').getValue() ===
      "SELECT 'second share';",
  );
  await sharedPage.close();

  // Format, embed, and keyboard execution use the same editor and worker.
  await edit('select 40+2 as answer;');
  await page.getByRole('button', { name: 'Tools', exact: true }).click();
  await page.getByRole('button', { name: /^Format SQL/ }).click();
  await page.waitForFunction(() =>
    globalThis.ace.edit('ace_editor').getValue().includes('SELECT'),
  );
  await page.getByRole('button', { name: 'Tools', exact: true }).click();
  await page.getByRole('button', { name: /^Embed Query Result/ }).click();
  await page.waitForFunction(() =>
    globalThis.ace.edit('ace_editor').getValue().startsWith('-- R:'),
  );
  await edit('SELECT 42 AS shortcut;');
  await page.locator('#ace_editor').click();
  await page.keyboard.press('Control+Enter');
  await page.getByRole('cell', { name: '42', exact: true }).waitFor();

  // Export and import preserve the database, including when a later import is invalid.
  await run(
    "CREATE TABLE imported(value TEXT); INSERT INTO imported VALUES ('roundtrip');",
  );
  const { data: database } = await downloadDatabase(page, 'default.db');
  assert.equal(database.subarray(0, 16).toString(), 'SQLite format 3\0');
  await page.waitForFunction(() =>
    JSON.parse(localStorage.getItem('config'))?.sql.includes('CREATE TABLE imported'),
  );
  const beforeImport = await page.evaluate(() => JSON.parse(localStorage.getItem('config')));
  assert.equal(beforeImport.database, 'default.db');
  await chooseFiles(page, [['import.db', database]]);
  await page.getByText('Database imported.', { exact: true }).waitFor();
  const afterImport = await page.evaluate(() => JSON.parse(localStorage.getItem('config')));
  for (const field of ['workspace_id', 'database', 'engine', 'keep_ctx', 'read_only']) {
    assert.equal(afterImport[field], beforeImport[field], `Import preserves ${field}`);
  }
  await page.getByRole('button', { name: /^Workspace: / }).click();
  await page.getByRole('combobox', { name: 'Main database', exact: true })
    .selectOption('import.db');
  await closeMenu();
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();
  await chooseFiles(page, [['invalid.db', Buffer.from('invalid')]]);
  await page.getByText('Import failed.', { exact: true }).waitFor();
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();

  // Another tab can retry after the current tab releases the OPFS locks.
  await page.waitForFunction(() => {
    const config = JSON.parse(localStorage.getItem('config'));
    return config?.database === 'import.db' && config.keep_ctx === true;
  });
  const blockedPage = await context.newPage();
  try {
    await blockedPage.goto(origin);
    await blockedPage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await blockedPage.evaluate(() => ace.edit('ace_editor').setValue('SELECT 1;', -1));
    await blockedPage.getByRole('button', { name: 'Run', exact: true }).click();
    await blockedPage.getByText('OPFS already opened', { exact: true }).waitFor();
    await blockedPage.getByText(/Close that tab and try again/).waitFor();
    await page.reload();
    await page.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await blockedPage.evaluate(() =>
      ace.edit('ace_editor').setValue('SELECT * FROM imported;', -1),
    );
    await blockedPage.getByRole('button', { name: 'Run', exact: true }).click();
    await blockedPage.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();
  } finally {
    await blockedPage.close();
  }
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();

  await checkEditor(page);
  await checkResultPaging(page);
  await checkBatchFailures(page);
  await checkInteractions(page);
  await checkSharing(page);
  await checkWorkspaceMenuPosition(page);
  await checkWorkspaces(page);
  await checkArchives(page);
  await checkFiles(page);

  await checkEngineSelection(page);
  await checkWorkers(page);
  assert.deepEqual(
    errors,
    [],
    'The app should load and run without browser errors',
  );
  console.log(
    'Browser checks passed: editor, execution, result paging, partial failures, preferences, sharing, workspaces, file management, ZIP transfers, read-only queries, all three engines, encryption, and OPFS.',
  );
} catch (error) {
  await mkdir('test-results', { recursive: true });
  await page.screenshot({
    path: 'test-results/smoke-failure.png',
    fullPage: true,
  });
  console.error('Browser errors:', errors);
  throw error;
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
