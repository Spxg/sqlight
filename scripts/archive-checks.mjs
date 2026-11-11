import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { unzipSync, zipSync } from 'fflate';
import {
  chooseFiles, closeFiles, downloadDatabase, downloadDatabases, openFiles,
} from './files-helpers.mjs';

const loggedDatabases = await readFile(
  new URL('./fixtures/logged-databases.zip', import.meta.url),
);
const fixture = unzipSync(loggedDatabases);

export async function checkArchives(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url());
  origin.search = '';
  origin.hash = '';
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin.origin)
      ? route.continue()
      : route.abort(),
  );
  const archivePage = await context.newPage();
  archivePage.setDefaultTimeout(20_000);

  async function settings() {
    return archivePage.evaluate(() => {
      const saved = JSON.parse(localStorage.getItem('config'));
      return Object.fromEntries([
        'workspace_id', 'workspace_name', 'database', 'engine',
        'sql', 'read_only', 'keep_ctx',
      ].map((key) => [key, saved[key]]));
    });
  }

  async function closeMenu() {
    await archivePage.keyboard.press('Escape');
    await archivePage.getByRole('dialog').waitFor({ state: 'hidden' });
  }

  async function workspaceMenu() {
    await archivePage.getByRole('button', { name: /^Workspace: / }).click();
    const menu = archivePage.getByRole('dialog');
    await menu.getByRole('checkbox', { name: 'Read only', exact: true }).waitFor();
    await archivePage.waitForFunction(() =>
      !document.querySelector('input[aria-label="Read only"]').disabled,
    );
    return menu;
  }

  async function setReadOnly(enabled) {
    const menu = await workspaceMenu();
    await menu.getByRole('checkbox', { name: 'Read only', exact: true })
      .setChecked(enabled);
    await closeMenu();
    await archivePage.waitForFunction(
      (enabled) => JSON.parse(localStorage.getItem('config'))?.read_only === enabled,
      enabled,
    );
  }

  async function selectDatabase(filename) {
    const menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Main database', exact: true })
      .selectOption(filename);
    await closeMenu();
    await archivePage.waitForFunction(
      (filename) => JSON.parse(localStorage.getItem('config'))?.database === filename,
      filename,
    );
  }

  async function databaseNames() {
    const menu = await workspaceMenu();
    const names = await menu.getByRole('combobox', { name: 'Main database', exact: true })
      .locator('option').evaluateAll((options) =>
        options.map((option) => option.value).sort(),
      );
    await closeMenu();
    return names;
  }

  async function edit(sql) {
    await archivePage.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await archivePage.waitForFunction(
      (sql) => JSON.parse(localStorage.getItem('config'))?.sql === sql,
      sql,
    );
  }

  async function run(sql) {
    await edit(sql);
    await archivePage.getByRole('button', { name: 'Run', exact: true }).click();
    await archivePage.getByText('Finished', { exact: true }).waitFor();
  }

  async function load(files, { replace = false, error } = {}) {
    const before = await settings();
    if (replace) archivePage.once('dialog', (dialog) => dialog.accept());
    await chooseFiles(archivePage, files);
    await archivePage.getByRole('status').filter({
      hasText: error ? /^Import failed\.$/ : /^Database imported\.$/,
    }).waitFor();
    if (error) {
      assert.match(await archivePage.getByRole('tabpanel').locator('summary').innerText(), error);
    }
    assert.deepEqual(await settings(), before, 'Import preserves this workspace and draft');
  }

  async function download(filename, format = 'zip') {
    const before = await settings();
    const result = filename === ''
      ? await downloadDatabases(archivePage)
      : await downloadDatabase(archivePage, filename, format);
    assert.deepEqual(await settings(), before, 'Download preserves this workspace and draft');
    return result;
  }

  function expectEntries(archive, expected) {
    const entries = unzipSync(archive);
    assert.deepEqual(Object.keys(entries).sort(), Object.keys(expected).sort());
    for (const [name, data] of Object.entries(expected)) {
      assert.deepEqual(Buffer.from(entries[name]), Buffer.from(data), `${name} keeps its bytes`);
    }
  }

  try {
    await archivePage.goto(origin.href);
    await archivePage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await edit('SELECT 1169 AS unopened_default;');
    const emptyFiles = await openFiles(archivePage);
    assert.equal(await emptyFiles.getByRole('region', { name: /^Database / }).count(), 0);
    await closeFiles(archivePage);
    await load([['empty.db', new Uint8Array(0)]]);
    const unopened = await download('');
    expectEntries(unopened.data, { 'empty.db': new Uint8Array(0) });
    await load([['unopened.zip', unopened.data]], { replace: true });
    await run('CREATE TABLE anchor(value INTEGER); INSERT INTO anchor VALUES (1173);');
    await edit('SELECT 1171 AS archive_draft;');
    await setReadOnly(true);
    const main = (await settings()).database;
    const anchor = await download(main, 'database');
    assert.equal(anchor.name, main);
    const anchorGroup = unzipSync((await download(main)).data);
    assert.deepEqual(Buffer.from(anchorGroup[main]), anchor.data);
    assert.ok(Object.keys(anchorGroup).every((name) =>
      name === main || name === `${main}-journal`,
    ));
    const journal = anchorGroup[`${main}-journal`];
    assert.ok(!journal || journal.length === 0 || journal[0] === 0,
      'A retained journal from the committed anchor is cold');

    const nested = Object.fromEntries(Object.entries(fixture).map(([name, data]) =>
      [`backup/${name}`, data],
    ));
    nested['__MACOSX/._wal.db'] = new Uint8Array([1, 2, 3]);
    nested['backup/.DS_Store'] = new Uint8Array([4, 5, 6]);
    await load([
      ['backup.zip', zipSync(nested)],
      ['extra.db', anchor.data],
    ]);
    assert.deepEqual(await databaseNames(), [main, 'empty.db', 'extra.db', 'hot.db', 'wal.db'].sort());
    const all = await download('');
    assert.equal(all.name, 'databases.zip');
    expectEntries(all.data, {
      ...fixture, ...anchorGroup, 'empty.db': new Uint8Array(0), 'extra.db': anchor.data,
    });

    const wal = await download('wal.db');
    assert.equal(wal.name, 'wal.db.zip');
    expectEntries(wal.data, Object.fromEntries(Object.entries(fixture).filter(([name]) =>
      name.startsWith('wal.db'),
    )));
    const hot = await download('hot.db');
    assert.equal(hot.name, 'hot.db.zip');
    expectEntries(hot.data, Object.fromEntries(Object.entries(fixture).filter(([name]) =>
      name.startsWith('hot.db'),
    )));

    const beforeInvalid = await databaseNames();
    await load([['broken.zip', Buffer.from('PK\x03\x04broken')]], { error: /zip|archive|invalid/i });
    await load([['orphan.zip', zipSync({ 'orphan.db-wal': fixture['wal.db-wal'] })]], {
      error: /orphan/i,
    });
    await load([['duplicate.zip', zipSync({
      'first/collision.db': anchor.data,
      'second/collision.db': anchor.data,
    })]], { error: /collision/i });
    assert.deepEqual(await databaseNames(), beforeInvalid, 'Rejected uploads do not add files');

    // Read-only mode cannot roll back a genuinely hot journal.
    await selectDatabase('hot.db');
    await edit('SELECT count(*), sum(value) FROM records;');
    await archivePage.getByRole('button', { name: 'Run', exact: true }).click();
    await archivePage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await archivePage.getByText(/This workspace is read only\./).waitFor();
    expectEntries((await download('hot.db')).data, unzipSync(hot.data));
    await setReadOnly(false);
    await run('SELECT count(*) AS recovered_rows, sum(value) AS recovered_sum FROM records;');
    assert.deepEqual(await archivePage.locator('tbody td').allTextContents(), ['80', '80']);
    const recovered = await download('hot.db', 'database');
    assert.equal(recovered.data.subarray(0, 16).toString(), 'SQLite format 3\0');

    // WAL records stay readable without checkpointing or changing read-only mode.
    await selectDatabase('wal.db');
    await setReadOnly(true);
    await run('SELECT count(*) AS wal_rows, sum(value) AS wal_sum FROM records;');
    assert.deepEqual(await archivePage.locator('tbody td').allTextContents(), ['2', '83']);
    expectEntries((await download('wal.db')).data, unzipSync(wal.data));

    // Replacing a group with a plain file removes its previous WAL and SHM.
    await load([['wal.db', anchor.data]], { replace: true });
    expectEntries((await download('wal.db')).data, { 'wal.db': anchor.data });
    await run('SELECT value FROM anchor;');
    await archivePage.getByRole('cell', { name: '1173', exact: true }).waitFor();
    await load([['wal.db.zip', wal.data]], { replace: true });
    expectEntries((await download('wal.db')).data, unzipSync(wal.data));
    await run(`ATTACH 'hot.db' AS hot;
      SELECT (SELECT sum(value) FROM main.records) AS wal_sum,
        (SELECT sum(value) FROM hot.records) AS recovered_sum;
      DETACH hot;`);
    assert.deepEqual(await archivePage.locator('tbody td').allTextContents(), ['83', '80']);
  } finally {
    await context.close();
  }
}
