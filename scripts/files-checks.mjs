import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { unzipSync } from 'fflate';
import {
  chooseFiles, closeFiles, databaseGroup, downloadDatabase,
  openFiles, waitForFilesIdle,
} from './files-helpers.mjs';

export async function checkFiles(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url());
  origin.search = '';
  origin.hash = '';
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin.origin)
      ? route.continue()
      : route.abort(),
  );
  const filesPage = await context.newPage();
  filesPage.setDefaultTimeout(20_000);

  async function config() {
    return filesPage.evaluate(() => JSON.parse(localStorage.getItem('config')));
  }

  async function edit(sql) {
    await filesPage.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await filesPage.waitForFunction(
      (sql) => JSON.parse(localStorage.getItem('config'))?.sql === sql,
      sql,
    );
  }

  async function run(sql) {
    await edit(sql);
    await filesPage.getByRole('button', { name: 'Run', exact: true }).click();
    await filesPage.getByText('Finished', { exact: true }).waitFor();
  }

  async function workspaceMenu() {
    await filesPage.getByRole('button', { name: /^Workspace: / }).click();
    const menu = filesPage.getByRole('dialog');
    await menu.getByRole('checkbox', { name: 'Read only', exact: true }).waitFor();
    await filesPage.waitForFunction(() =>
      !document.querySelector('input[aria-label="Read only"]').disabled,
    );
    return menu;
  }

  async function closeWorkspace() {
    await filesPage.keyboard.press('Escape');
    await filesPage.getByRole('dialog').waitFor({ state: 'hidden' });
  }

  async function createFile(filename) {
    const before = await config();
    const drawer = await openFiles(filesPage);
    await drawer.getByRole('textbox', { name: 'New database name', exact: true }).fill(filename);
    await drawer.getByRole('button', { name: 'Create database', exact: true }).click();
    await databaseGroup(drawer, filename).waitFor();
    await waitForFilesIdle(filesPage, drawer);
    assert.deepEqual(await config(), before, 'Filesystem creation preserves the workspace');
    await closeFiles(filesPage);
  }

  async function upload(files) {
    const before = await config();
    await chooseFiles(filesPage, files);
    const status = filesPage.getByRole('status').filter({
      hasText: /^(Database imported\.|Import failed\.)$/,
    });
    await status.waitFor();
    assert.equal(await status.textContent(), 'Database imported.',
      await filesPage.getByRole('tabpanel').innerText());
    assert.deepEqual(await config(), before, 'Filesystem upload preserves the workspace');
  }

  async function disconnect() {
    const drawer = await openFiles(filesPage);
    await drawer.getByRole('button', { name: 'Disconnect', exact: true }).click();
    await waitForFilesIdle(filesPage, drawer);
    assert.equal(await drawer.getByRole('button', {
      name: 'Disconnect', exact: true,
    }).isDisabled(), true);
    await closeFiles(filesPage);
  }

  async function deleteFile(filename) {
    const before = await config();
    const drawer = await openFiles(filesPage);
    filesPage.once('dialog', async (dialog) => {
      assert.ok(dialog.message().includes(filename));
      await dialog.accept();
    });
    await databaseGroup(drawer, filename)
      .getByRole('button', { name: 'Delete', exact: true }).click();
    await databaseGroup(drawer, filename).waitFor({ state: 'hidden' });
    await waitForFilesIdle(filesPage, drawer);
    const after = await config();
    for (const field of ['workspace_id', 'database', 'engine', 'sql', 'read_only', 'keep_ctx']) {
      assert.equal(after[field], before[field], `Deleting a file preserves ${field}`);
    }
    await closeFiles(filesPage);
  }

  async function useFile(filename) {
    const menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Main database', exact: true })
      .selectOption(filename);
    await closeWorkspace();
    await filesPage.waitForFunction(
      (filename) => JSON.parse(localStorage.getItem('config'))?.database === filename,
      filename,
    );
  }

  async function expectCreatedOnRun(filename, embed = false) {
    const sql = 'SELECT count(*) AS recreated_tables FROM sqlite_schema;';
    await edit(sql);
    const before = await config();
    const menu = await workspaceMenu();
    const selected = menu.getByRole('combobox', { name: 'Main database', exact: true });
    assert.equal(await selected.inputValue(), filename);
    assert.equal(await selected.locator('option:checked').textContent(), `${filename} (Created on run)`);
    await closeWorkspace();
    if (embed) {
      await filesPage.getByRole('button', { name: 'Tools', exact: true }).click();
      await filesPage.getByRole('button', { name: /^Embed Query Result/ }).click();
      await filesPage.waitForFunction((sql) => {
        const value = ace.edit('ace_editor').getValue();
        return value.startsWith('-- R:') && value.endsWith(sql);
      }, sql);
      assert.match(await filesPage.evaluate(() => ace.edit('ace_editor').getValue()),
        /-- R: \|\s*0\s*\|/);
      await filesPage.waitForFunction(() =>
        JSON.parse(localStorage.getItem('config'))?.sql === ace.edit('ace_editor').getValue(),
      );
    } else {
      await filesPage.getByRole('button', { name: 'Run', exact: true }).click();
      await filesPage.getByText('Finished', { exact: true }).waitFor();
      await filesPage.getByRole('cell', { name: '0', exact: true }).waitFor();
      assert.equal(await filesPage.evaluate(() => ace.edit('ace_editor').getValue()), sql);
    }
    const drawer = await openFiles(filesPage);
    assert.equal(await databaseGroup(drawer, filename).count(), 1);
    await closeFiles(filesPage);
    const after = await config();
    for (const field of ['workspace_id', 'database', 'engine', 'read_only', 'keep_ctx']) {
      assert.equal(after[field], before[field], `Automatic creation preserves ${field}`);
    }
  }

  try {
    await filesPage.goto(origin.href);
    await filesPage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await edit('SELECT 1191 AS files_draft;');
    const original = await config();
    let drawer = await openFiles(filesPage);
    await drawer.getByText('No database files yet. Create a database or upload files.', {
      exact: true,
    }).waitFor();
    assert.equal(await drawer.getByRole('region', { name: /^Database / }).count(), 0);
    await closeFiles(filesPage);
    assert.deepEqual(await config(), original);
    await createFile('spare.db');
    const empty = await downloadDatabase(filesPage, 'spare.db');
    assert.equal(empty.data.length, 0);
    drawer = await openFiles(filesPage);
    assert.equal(await databaseGroup(drawer, 'default.db').count(), 0);
    await closeFiles(filesPage);

    let menu = await workspaceMenu();
    await menu.getByRole('checkbox', { name: 'Fresh database per run', exact: true }).uncheck();
    await closeWorkspace();
    await run('CREATE TABLE active(value INTEGER); INSERT INTO active VALUES (1);');
    const validDatabase = (await downloadDatabase(filesPage, 'default.db')).data;
    assert.equal(validDatabase.subarray(0, 16).toString(), 'SQLite format 3\0');

    // Selecting a workspace's main file does not open another connection.
    menu = await workspaceMenu();
    await menu.getByRole('button', { name: /^New workspace/ }).click();
    await closeWorkspace();
    menu = await workspaceMenu();
    await menu.getByRole('textbox', { name: 'Workspace name', exact: true })
      .fill('Second file view');
    await menu.getByRole('button', { name: /^Rename workspace/ }).click();
    await closeWorkspace();
    menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'SQLite Engine', exact: true }).selectOption('SQLCipher');
    await closeWorkspace();
    await useFile('spare.db');
    drawer = await openFiles(filesPage);
    assert.match(await databaseGroup(drawer, 'default.db').innerText(), /Used by:.*Default/);
    assert.match(await databaseGroup(drawer, 'spare.db').innerText(), /Used by:.*Second file view/);
    assert.equal(await databaseGroup(drawer, 'default.db').getByRole('button', {
      name: 'Delete', exact: true,
    }).isDisabled(), true);
    assert.equal(await databaseGroup(drawer, 'spare.db').getByRole('button', {
      name: 'Delete', exact: true,
    }).isEnabled(), true);
    await closeFiles(filesPage);
    menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Workspace', exact: true }).selectOption('default');
    await closeWorkspace();

    await run('BEGIN; INSERT INTO active VALUES (2);');
    drawer = await openFiles(filesPage);
    assert.equal(await databaseGroup(drawer, 'spare.db').getByRole('button', {
      name: 'Delete', exact: true,
    }).isDisabled(), true, 'Transactions protect even unrelated files');
    await drawer.getByRole('button', { name: 'Disconnect', exact: true }).click();
    await drawer.getByRole('alert').filter({ hasText: 'Commit or roll back' }).waitFor();
    await closeFiles(filesPage);
    await run('ROLLBACK; SELECT count(*) AS kept_rows FROM active;');
    await filesPage.getByRole('cell', { name: '1', exact: true }).waitFor();
    await run("ATTACH 'spare.db' AS side; SELECT 1193 AS attached;");
    drawer = await openFiles(filesPage);
    for (const filename of ['default.db', 'spare.db']) {
      assert.equal(await databaseGroup(drawer, filename).getByRole('button', {
        name: 'Delete', exact: true,
      }).isDisabled(), true, `${filename} is mounted in the live connection`);
    }
    await closeFiles(filesPage);
    await disconnect();
    await deleteFile('spare.db');
    await deleteFile('default.db');
    await expectCreatedOnRun('default.db');
    await disconnect();
    await deleteFile('default.db');
    await expectCreatedOnRun('default.db', true);
    await disconnect();
    await deleteFile('default.db');

    // A missing default must not hide or discard another connection's transaction.
    await createFile('connection-guard.db');
    menu = await workspaceMenu();
    await menu.getByRole('button', { name: /^New workspace/ }).click();
    await closeWorkspace();
    await useFile('connection-guard.db');
    await run(`CREATE TABLE connection_guard(value INTEGER);
      INSERT INTO connection_guard VALUES (1);
      BEGIN; INSERT INTO connection_guard VALUES (2);`);
    menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Workspace', exact: true }).selectOption('default');
    await menu.getByRole('combobox', { name: 'SQLite Engine', exact: true }).selectOption('SQLCipher');
    await closeWorkspace();
    await edit('SELECT 1195 AS blocked_switch;');
    await filesPage.getByRole('button', { name: 'Run', exact: true }).click();
    await filesPage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await filesPage.getByRole('tabpanel').getByText(/Run COMMIT or ROLLBACK/).waitFor();
    const restored = await config();
    assert.equal(restored.database, 'connection-guard.db');
    assert.equal(restored.engine, 'SQLite');
    assert.equal(restored.keep_ctx, true);
    await run('ROLLBACK; SELECT count(*) AS kept_rows FROM connection_guard;');
    await filesPage.getByRole('cell', { name: '1', exact: true }).waitFor();
    await disconnect();
    await deleteFile('connection-guard.db');

    // Corrupt databases and hot journals can be managed without opening SQLite.
    const logs = await readFile(new URL('./fixtures/logged-databases.zip', import.meta.url));
    const corrupt = Buffer.from(validDatabase);
    corrupt[100] = 0xff; // Keep the SQLite header valid, but corrupt page one's B-tree.
    await upload([['logs.zip', logs], ['corrupt.db', corrupt]]);
    drawer = await openFiles(filesPage);
    const hot = databaseGroup(drawer, 'hot.db');
    await hot.locator('summary').click();
    await hot.getByText('hot.db-journal', { exact: true }).waitFor();
    assert.match(await hot.innerText(), /\d+.*(B|KiB|KB)/);
    await drawer.getByRole('button', { name: 'Refresh', exact: true }).click();
    await waitForFilesIdle(filesPage, drawer);
    assert.equal(await hot.locator('details').evaluate((element) => element.open), true,
      'Refreshing file metadata preserves expanded logs');
    await closeFiles(filesPage);
    const hotArchive = unzipSync((await downloadDatabase(filesPage, 'hot.db', 'zip')).data);
    const originals = unzipSync(logs);
    assert.deepEqual(Buffer.from(hotArchive['hot.db-journal']), Buffer.from(originals['hot.db-journal']));
    await useFile('corrupt.db');
    await edit('SELECT name FROM sqlite_schema;');
    await filesPage.getByRole('button', { name: 'Run', exact: true }).click();
    await filesPage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await filesPage.getByRole('tabpanel').getByText(/disk image is malformed/i).waitFor();
    assert.deepEqual((await downloadDatabase(filesPage, 'corrupt.db')).data, corrupt);
    await disconnect();
    await deleteFile('corrupt.db');

    // Deleting an unrelated file preserves an encrypted connection and its key.
    menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'SQLite Engine', exact: true }).selectOption('SQLCipher');
    await closeWorkspace();
    await upload([['keyed.db', new Uint8Array(0)], ['unrelated.db', new Uint8Array(0)]]);
    await useFile('keyed.db');
    await run(`PRAGMA key='files-test-key';
      CREATE TABLE secrets(value INTEGER);
      INSERT INTO secrets VALUES (1197);`);
    const keyed = await downloadDatabase(filesPage, 'keyed.db');
    await deleteFile('unrelated.db');
    await run('SELECT value FROM secrets;');
    await filesPage.getByRole('cell', { name: '1197', exact: true }).waitFor();
    await disconnect();
    await edit("PRAGMA key='wrong-key'; SELECT value FROM secrets;");
    await filesPage.getByRole('button', { name: 'Run', exact: true }).click();
    await filesPage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await filesPage.getByText(/Check the SQLite engine, encryption key/).waitFor();
    assert.deepEqual((await downloadDatabase(filesPage, 'keyed.db')).data, keyed.data);
    await disconnect();
    await deleteFile('keyed.db');
  } finally {
    await context.close();
  }
}
