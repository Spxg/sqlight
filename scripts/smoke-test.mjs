import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { checkEditor } from './editor-checks.mjs';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
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
    globalThis.ace?.edit('ace_editor').getValue().includes('Hello World!'),
  );
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page
    .getByRole('cell', { name: "'Hello World!'", exact: true })
    .waitFor();
  await page.getByText('Finished', { exact: true }).waitFor();

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
  await page.getByRole('button', { name: 'Database', exact: true }).click();
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: /^Download/ }).click();
  const download = await downloadPromise;
  const database = await readFile(await download.path());
  assert.equal(database.subarray(0, 16).toString(), 'SQLite format 3\0');
  await page.locator('input[type=file]').setInputFiles({
    name: 'import.db',
    mimeType: 'application/vnd.sqlite3',
    buffer: database,
  });
  await page.getByText('Database imported.', { exact: true }).waitFor();
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();
  await page.locator('input[type=file]').setInputFiles({
    name: 'invalid.db',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('invalid'),
  });
  await page.getByText('Import failed.', { exact: true }).waitFor();
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();

  // An OPFS lock failure must not leave the cached Memory connection unusable.
  await page.evaluate(async () => {
    const worker = new Worker('./sqlite3_loader.js', { type: 'module' });
    globalThis.blockingWorker = worker;
    await new Promise((resolve) => {
      worker.onmessage = resolve;
    });
    const opened = new Promise((resolve) => {
      worker.onmessage = (event) => resolve(event.data);
    });
    worker.postMessage({ Open: { filename: 'locked.db', persist: true } });
    const response = await opened;
    if (!Object.hasOwn(response.Open, 'Ok'))
      throw new Error(JSON.stringify(response));
  });
  await page.getByRole('button', { name: 'Memory', exact: true }).click();
  await page.getByRole('button', { name: /^OPFS Persistent/ }).click();
  await closeMenu();
  await edit('SELECT 1;');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.getByText('OPFS already opened', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'OPFS', exact: true }).click();
  await page.getByRole('button', { name: /^Memory Data/ }).click();
  await closeMenu();
  await run('SELECT * FROM imported;');
  await page.getByRole('cell', { name: "'roundtrip'", exact: true }).waitFor();
  await page.evaluate(() => globalThis.blockingWorker.terminate());

  await checkEditor(page);

  // Both standalone workers support SQL, extensions, transfers, and persistent storage.
  const workers = await page.evaluate(async () => {
    const checked = [];
    for (const engine of ['sqlite3', 'sqlite3mc']) {
      const worker = new Worker(`./${engine}_loader.js`, { type: 'module' });
      const pending = [];
      worker.onmessage = (event) => pending.shift()(event.data);
      const receive = () => new Promise((resolve) => pending.push(resolve));
      const ready = await receive();
      if (ready !== 'Ready')
        throw new Error(`Unexpected worker ready: ${JSON.stringify(ready)}`);
      const request = async (value) => {
        const response = receive();
        worker.postMessage(value);
        return response;
      };
      const unwrap = (response) => {
        const result = Object.values(response)[0];
        if (!Object.hasOwn(result, 'Ok'))
          throw new Error(JSON.stringify(response));
        return result.Ok;
      };
      const sql = async (value) =>
        unwrap(
          await request({
            Run: { sql: value, embed: false, clear_on_prepare: false },
          }),
        );
      try {
        unwrap(
          await request({ Open: { filename: 'smoke.db', persist: false } }),
        );
        const result = await sql(
          "; -- comment\n SELECT '你好' AS unicode, vec_version() AS vec; ; SELECT 2 AS second;",
        );
        if (result.result.filter((item) => item.Step).length !== 2)
          throw new Error('SQL statements were skipped');
        if (result.result[0].Step.values.rows[0][0] !== "'你好'")
          throw new Error('Unicode result changed');
        await sql(
          'CREATE TABLE persistent(value); INSERT INTO persistent VALUES (7); BEGIN; INSERT INTO persistent VALUES (8);',
        );
        const transaction = await request('DownloadDb');
        if (!transaction.DownloadDb.Err)
          throw new Error('Export accepted an uncommitted transaction');
        await sql('ROLLBACK;');
        const exported = unwrap(await request('DownloadDb'));
        unwrap(await request({ LoadDb: { data: exported.data } }));
        const imported = await sql('SELECT * FROM persistent;');
        if (imported.result[0].Step.values.rows[0][0] !== '7')
          throw new Error('Database transfer changed rows');
        if (engine === 'sqlite3mc') {
          unwrap(
            await request({
              Open: { filename: 'encrypted.db', persist: false },
            }),
          );
          await sql(
            "PRAGMA key='smoke-key'; CREATE TABLE secret(value); INSERT INTO secret VALUES (456);",
          );
          const encrypted = unwrap(await request('DownloadDb'));
          if (
            new TextDecoder().decode(encrypted.data.slice(0, 16)) ===
            'SQLite format 3\0'
          ) {
            throw new Error('The encrypted database has a plaintext header');
          }
          unwrap(await request({ LoadDb: { data: encrypted.data } }));
          const decrypted = await sql(
            "PRAGMA key='smoke-key'; SELECT * FROM secret;",
          );
          const table = decrypted.result.find(
            (item) => item.Step?.values?.columns[0] === 'value',
          );
          if (table.Step.values.rows[0][0] !== '456') {
            throw new Error('Encrypted database transfer changed rows');
          }
        }
        unwrap(
          await request({
            Open: { filename: `smoke-${engine}.db`, persist: true },
          }),
        );
        await sql(
          'CREATE TABLE IF NOT EXISTS persisted(value); DELETE FROM persisted; INSERT INTO persisted VALUES (123);',
        );
        unwrap(
          await request({ Open: { filename: 'memory.db', persist: false } }),
        );
        unwrap(
          await request({
            Open: { filename: `smoke-${engine}.db`, persist: true },
          }),
        );
        const persistent = await sql('SELECT * FROM persisted;');
        if (persistent.result[0].Step.values.rows[0][0] !== '123')
          throw new Error('OPFS did not persist data');
        checked.push(engine);
      } finally {
        worker.terminate();
      }
    }
    return checked;
  });
  assert.deepEqual(workers, ['sqlite3', 'sqlite3mc']);
  assert.deepEqual(
    errors,
    [],
    'The app should load and run without browser errors',
  );
  console.log(
    'Browser checks passed: editor, execution, preferences, sharing, transfers, both workers, and OPFS.',
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
