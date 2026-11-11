import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

export async function checkWorkspaces(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url());
  origin.search = '';
  origin.hash = '';
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin.origin)
      ? route.continue()
      : route.abort(),
  );
  const workspacePage = await context.newPage();
  workspacePage.setDefaultTimeout(20_000);

  async function config(target = workspacePage) {
    return target.evaluate(() =>
      JSON.parse(localStorage.getItem('config')),
    );
  }

  async function closeMenu(target = workspacePage) {
    await target.keyboard.press('Escape');
    await target.getByRole('dialog').waitFor({ state: 'hidden' });
  }

  async function workspaceMenu(target = workspacePage) {
    await target.getByRole('button', { name: /^Workspace: / }).click();
    const menu = target.getByRole('dialog');
    await menu.getByRole('checkbox', { name: 'Read only', exact: true }).waitFor();
    await target.waitForFunction(() =>
      !document.querySelector('input[aria-label="Read only"]').disabled,
    );
    return menu;
  }

  async function selectWorkspace(id, target = workspacePage) {
    const menu = await workspaceMenu(target);
    const selected = menu.getByRole('combobox', { name: 'Workspace', exact: true });
    await selected.selectOption(id);
    assert.equal(await selected.inputValue(), id);
    await closeMenu(target);
    await target.waitForFunction((id) => {
      const saved = JSON.parse(localStorage.getItem('config'));
      return saved?.workspace_id === id ||
        saved?.workspaces.some((workspace) => workspace.id === id);
    }, id);
  }

  async function createWorkspace(name) {
    const previousConfig = await config();
    const previous = previousConfig?.workspace_id ?? 'default';
    const menu = await workspaceMenu();
    await menu.getByRole('button', { name: /^New workspace/ }).click();
    await closeMenu();
    await workspacePage.waitForFunction(
      (id) => JSON.parse(localStorage.getItem('config'))?.workspace_id !== id,
      previous,
    );
    const created = await config();
    assert.equal(created.engine, previousConfig.engine);
    assert.equal(created.database, previousConfig.database);
    assert.equal(created.keep_ctx, true);
    assert.equal(created.read_only, previousConfig.read_only);
    assert.equal(created.run_selected_sql, previousConfig.run_selected_sql);

    const renameMenu = await workspaceMenu();
    await renameMenu.getByRole('textbox', { name: 'Workspace name', exact: true })
      .fill(name);
    await renameMenu.getByRole('button', { name: /^Rename workspace/ })
      .click();
    await closeMenu();
    await workspacePage.waitForFunction(
      (name) => JSON.parse(localStorage.getItem('config'))?.workspace_name === name,
      name,
    );
    return created.workspace_id;
  }

  async function setFreshDatabase(enabled) {
    const menu = await workspaceMenu();
    await menu.getByRole('checkbox', {
      name: 'Fresh database per run', exact: true,
    }).setChecked(enabled);
    await closeMenu();
    await workspacePage.waitForFunction((enabled) => {
      const config = JSON.parse(localStorage.getItem('config'));
      return config?.keep_ctx === !enabled;
    }, enabled);
  }

  async function setReadOnly(enabled) {
    const menu = await workspaceMenu();
    await menu.getByRole('checkbox', { name: 'Read only', exact: true })
      .setChecked(enabled);
    await closeMenu();
    await workspacePage.waitForFunction(
      (enabled) => JSON.parse(localStorage.getItem('config'))?.read_only === enabled,
      enabled,
    );
  }

  async function selectEngine(engine) {
    const menu = await workspaceMenu();
    await menu.getByRole('combobox', {
      name: 'SQLite Engine', exact: true,
    }).selectOption(engine);
    await closeMenu();
    await workspacePage.waitForFunction((engine) => {
      const selected = engine === 'SQLite3 Multiple Ciphers'
        ? 'MultipleCiphers'
        : engine;
      return JSON.parse(localStorage.getItem('config'))?.engine === selected;
    }, engine);
  }

  async function edit(sql, target = workspacePage) {
    await target.evaluate(
      (sql) => ace.edit('ace_editor').setValue(sql, -1),
      sql,
    );
    await target.waitForFunction(
      (sql) => JSON.parse(localStorage.getItem('config'))?.sql === sql,
      sql,
    );
  }

  async function run(sql) {
    await edit(sql);
    await workspacePage.getByRole('button', { name: 'Run', exact: true }).click();
    await workspacePage.getByText('Finished', { exact: true }).waitFor();
  }

  async function expectValue(value) {
    await workspacePage.getByRole('cell', { name: value, exact: true }).waitFor();
  }

  async function download(filename) {
    await workspacePage.getByRole('button', { name: 'Database', exact: true })
      .click();
    if (filename) {
      await workspacePage.getByRole('combobox', { name: 'Download database', exact: true })
        .selectOption(filename);
    }
    const downloaded = workspacePage.waitForEvent('download');
    await workspacePage.getByRole('button', { name: /^Download/ }).click();
    const file = await downloaded;
    if (filename) assert.equal(file.suggestedFilename(), filename);
    return readFile(await file.path());
  }

  async function chooseDatabaseFiles(files) {
    await workspacePage.getByRole('button', { name: 'Database', exact: true })
      .click();
    const chooserOpened = workspacePage.waitForEvent('filechooser');
    await workspacePage.getByRole('button', { name: /^Load/ }).click();
    const chooser = await chooserOpened;
    await chooser.setFiles(
      files.map(([name, buffer]) => ({
        name,
        mimeType: 'application/vnd.sqlite3',
        buffer,
      })),
    );
  }

  async function importDatabases(files) {
    const before = await config();
    await chooseDatabaseFiles(files);
    const status = workspacePage.getByRole('status').filter({
      hasText: /^(Database imported\.|Import failed\.)$/,
    });
    await status.waitFor();
    if (await status.textContent() === 'Import failed.') {
      throw new Error(`Database import failed: ${await workspacePage
        .getByRole('tabpanel').innerText()}`);
    }
    const after = await config();
    for (const field of [
      'workspace_id', 'workspace_name', 'database', 'engine', 'read_only', 'keep_ctx', 'sql',
    ]) {
      assert.equal(after[field], before[field], `Import preserves ${field}`);
    }
  }

  async function selectDatabase(filename) {
    const menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Main database', exact: true })
      .selectOption(filename);
    await closeMenu();
    await workspacePage.waitForFunction(
      (filename) => JSON.parse(localStorage.getItem('config'))?.database === filename,
      filename,
    );
  }

  async function expectDatabase(filename) {
    const menu = await workspaceMenu();
    await menu.getByRole('combobox', { name: 'Main database', exact: true })
      .locator('option').filter({ hasText: filename }).waitFor({ state: 'attached' });
    await closeMenu();
  }

  async function expectSettingChangeRejected() {
    await workspacePage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await workspacePage.getByText(
      'Commit or roll back the current transaction before switching database settings',
      { exact: true },
    ).waitFor();
  }

  try {
    await workspacePage.goto(origin.href);
    await workspacePage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await setFreshDatabase(false);
    const initialDatabase = (await config()).database;
    assert.equal(initialDatabase, 'default.db');
    await setFreshDatabase(true);
    await run('CREATE TABLE fresh_run_probe(value INTEGER);');
    await run("SELECT count(*) FROM sqlite_schema WHERE name = 'fresh_run_probe';");
    await expectValue('0');
    await setFreshDatabase(false);
    await run(`CREATE TABLE customers(id INTEGER PRIMARY KEY, name TEXT);
      INSERT INTO customers VALUES (1, 'Alice'), (2, 'Bob');`);
    const customersDatabase = await download();

    await run(`ATTACH 'seed-orders.db' AS seed;
      CREATE TABLE seed.orders(customer_id INTEGER, total INTEGER);
      INSERT INTO seed.orders VALUES (1, 10), (1, 20), (2, 7);
      DETACH seed;`);
    const ordersWorkspace = await createWorkspace('Orders report');
    await setFreshDatabase(false);
    await selectDatabase('seed-orders.db');
    const ordersDatabase = await download();

    // Fresh databases can be queried through a read-only connection too.
    await setFreshDatabase(true);
    await setReadOnly(true);
    assert.equal((await config()).keep_ctx, false);
    await run(`PRAGMA compile_options;
      SELECT sqlite_version() AS version;
      SELECT count(*) AS fresh_tables FROM sqlite_schema;`);
    await workspacePage.getByRole('columnheader', {
      name: 'compile_options', exact: true,
    }).waitFor();
    await workspacePage.getByRole('columnheader', { name: 'version', exact: true })
      .waitFor();
    await workspacePage.getByRole('table', { name: 'Statement #3', exact: true })
      .getByRole('cell', { name: '0', exact: true }).waitFor();
    await edit(`ATTACH '${initialDatabase}' AS source;
      INSERT INTO source.customers VALUES (9, 'read-only write');`);
    await workspacePage.getByRole('button', { name: 'Run', exact: true }).click();
    await workspacePage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await workspacePage.getByText(/This workspace is read only\./).waitFor();
    await run('SELECT count(*) AS fresh_again FROM sqlite_schema;');
    await expectValue('0');
    assert.equal((await config()).read_only, true);
    workspacePage.once('dialog', (dialog) => dialog.accept());
    await importDatabases([['seed-orders.db', ordersDatabase]]);
    assert.equal((await config()).keep_ctx, false);
    await setFreshDatabase(false);
    await run('SELECT sum(total) AS restored_orders FROM orders;');
    await expectValue('37');
    await setReadOnly(false);

    // Import both files with their own names into the same engine.
    await importDatabases([
      ['a.db', customersDatabase],
      ['b.db', ordersDatabase],
    ]);
    await expectDatabase('a.db');
    await expectDatabase('b.db');
    await selectDatabase('a.db');

    const beforeDownload = await config();
    const independentlyDownloaded = await download('b.db');
    assert.deepEqual(independentlyDownloaded, ordersDatabase);
    const afterDownload = await config();
    for (const field of ['database', 'sql', 'engine', 'keep_ctx', 'read_only']) {
      assert.equal(afterDownload[field], beforeDownload[field], `${field} stays unchanged`);
    }
    await setReadOnly(true);
    const readOnlyMenu = await workspaceMenu();
    const freshDatabase = readOnlyMenu.getByRole('checkbox', {
      name: 'Fresh database per run', exact: true,
    });
    assert.equal(await freshDatabase.isEnabled(), true);
    assert.equal(await freshDatabase.isChecked(), false);
    await closeMenu();
    assert.equal((await config()).read_only, true);
    await setReadOnly(false);
    assert.equal((await config()).keep_ctx, true);
    await run('SELECT count(*) AS still_main_a FROM customers;');
    await expectValue('2');

    await run(`ATTACH 'b.db' AS b;
      SELECT c.name, sum(o.total) AS total
      FROM customers AS c JOIN b.orders AS o ON o.customer_id = c.id
      GROUP BY c.id ORDER BY c.id;`);
    assert.deepEqual(await workspacePage.locator('tbody td').allTextContents(), [
      "'Alice'", '30', "'Bob'", '7',
    ]);
    await run('DETACH b;');
    await selectDatabase('b.db');
    const ordersQuery = 'SELECT sum(total) AS total FROM orders; -- orders draft';
    await run(ordersQuery);
    await expectValue('37');

    const customersWorkspace = await createWorkspace('Customers report');
    await setFreshDatabase(false);
    await selectDatabase('a.db');
    const customersQuery = 'SELECT name FROM customers ORDER BY id; -- customers draft';
    await run(customersQuery);
    await expectValue("'Alice'");
    await run("INSERT INTO customers VALUES (3, 'Carol');");
    await edit(customersQuery);
    await selectWorkspace(ordersWorkspace);
    assert.equal((await config()).database, 'b.db');
    assert.equal(
      await workspacePage.evaluate(() => ace.edit('ace_editor').getValue()),
      ordersQuery,
    );
    await run(ordersQuery);
    await expectValue('37');

    const sameDatabaseWorkspace = await createWorkspace('Customers count');
    await setFreshDatabase(false);
    await selectDatabase('a.db');
    await run('SELECT count(*) AS shared_customers FROM customers;');
    await expectValue('3');
    await selectWorkspace(customersWorkspace);
    assert.equal((await config()).database, 'a.db');
    assert.equal(
      await workspacePage.evaluate(() => ace.edit('ace_editor').getValue()),
      customersQuery,
    );

    // Configurations on the same database reuse its live transaction.
    await run(`CREATE TABLE transaction_restore(value INTEGER);
      BEGIN;
      INSERT INTO transaction_restore VALUES (1);`);
    await selectWorkspace(sameDatabaseWorkspace);
    await run('SELECT count(*) AS in_same_connection FROM transaction_restore;');
    await expectValue('1');
    await selectDatabase('b.db');
    await edit('SELECT sum(total) FROM orders;');
    await workspacePage.getByRole('button', { name: 'Run', exact: true }).click();
    await expectSettingChangeRejected();
    await workspacePage.waitForFunction(() =>
      JSON.parse(localStorage.getItem('config'))?.database === 'a.db',
    );
    await run('ROLLBACK; SELECT count(*) AS rolled_back FROM transaction_restore;');
    await expectValue('0');

    // A rejected read-only change restores the connection's writable settings.
    await run('BEGIN; INSERT INTO transaction_restore VALUES (1);');
    await setReadOnly(true);
    await edit('SELECT count(*) FROM transaction_restore;');
    await workspacePage.getByRole('button', { name: 'Run', exact: true }).click();
    await expectSettingChangeRejected();
    const restoredMenu = await workspaceMenu();
    assert.equal(await restoredMenu.getByRole('checkbox', {
      name: 'Fresh database per run', exact: true,
    }).isChecked(), false);
    await closeMenu();
    await run('ROLLBACK; SELECT count(*) AS rolled_back FROM transaction_restore;');
    await expectValue('0');
    assert.equal((await config()).read_only, false);

    await setReadOnly(true);
    await run('SELECT count(*) AS unchanged FROM customers;');
    await expectValue('3');
    const readOnlyDownload = await download();
    assert.equal(readOnlyDownload.subarray(0, 16).toString(), 'SQLite format 3\0');
    await edit("INSERT INTO customers VALUES (4, 'must not be written');");
    await workspacePage.getByRole('button', { name: 'Run', exact: true }).click();
    await workspacePage.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await run('SELECT count(*) AS unchanged FROM customers;');
    await expectValue('3');
    await importDatabases([['c.db', ordersDatabase]]);
    await selectDatabase('c.db');
    await run('SELECT sum(total) AS imported_while_read_only FROM orders;');
    await expectValue('37');
    assert.equal((await config()).read_only, true);

    workspacePage.once('dialog', (dialog) => dialog.dismiss());
    await chooseDatabaseFiles([['a.db', customersDatabase]]);
    await selectDatabase('a.db');
    await run('SELECT count(*) AS cancelled_replacement FROM customers;');
    await expectValue('3');
    workspacePage.once('dialog', (dialog) => dialog.accept());
    await importDatabases([['a.db', customersDatabase]]);
    await run('SELECT count(*) AS confirmed_replacement FROM customers;');
    await expectValue('2');
    assert.equal((await config()).read_only, true);

    // Switching SQL configurations for one encrypted file keeps its key.
    const encryptedWorkspace = await createWorkspace('Encrypted query');
    await setReadOnly(false);
    await setFreshDatabase(false);
    await selectEngine('SQLCipher');
    await run(`PRAGMA key='shared-connection-key';
      CREATE TABLE encrypted_data(value INTEGER);
      INSERT INTO encrypted_data VALUES (41);`);
    const encryptedFilename = (await config()).database;
    const encryptedDownload = await download(encryptedFilename);
    assert.notEqual(encryptedDownload.subarray(0, 16).toString(), 'SQLite format 3\0');
    await run('SELECT value FROM encrypted_data;');
    await expectValue('41');
    const encryptedReport = await createWorkspace('Encrypted count');
    await selectEngine('SQLCipher');
    await setFreshDatabase(false);
    await selectDatabase(encryptedFilename);
    await run('SELECT count(*) AS encrypted_rows FROM encrypted_data;');
    await expectValue('1');
    await selectWorkspace(encryptedWorkspace);
    await run('SELECT value FROM encrypted_data;');
    await expectValue('41');
    await selectWorkspace(encryptedReport);
    await run('SELECT value + 1 AS encrypted_answer FROM encrypted_data;');
    await expectValue('42');

    const persistentCustomers = await createWorkspace('Persistent customers');
    await selectEngine('SQLite');
    await setFreshDatabase(false);
    await selectDatabase('a.db');
    await setReadOnly(true);
    const persistentQuery = 'SELECT name FROM customers ORDER BY id; -- persistent draft';
    await edit(persistentQuery);
    const persistentOrders = await createWorkspace('Persistent orders');
    await setReadOnly(false);
    await setFreshDatabase(false);
    await selectDatabase('b.db');
    await run(ordersQuery);
    await expectValue('37');
    await selectWorkspace(persistentCustomers);
    await workspacePage.reload();
    await workspacePage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    assert.equal((await config()).database, 'a.db');
    assert.equal((await config()).read_only, true);
    assert.equal(
      await workspacePage.evaluate(() => ace.edit('ace_editor').getValue()),
      persistentQuery,
    );
    await run(`ATTACH 'b.db' AS b;
      SELECT sum(o.total) AS joined_total
      FROM customers AS c JOIN b.orders AS o ON o.customer_id = c.id;`);
    await expectValue('37');
    await run('DETACH b;');

    // Deleting a configuration leaves shared files available to other reports.
    await selectWorkspace(persistentOrders);
    assert.equal((await config()).database, 'b.db');
    assert.equal((await config()).read_only, false);
    const menu = await workspaceMenu();
    workspacePage.once('dialog', (dialog) => dialog.accept());
    await menu.getByRole('button', { name: /^Delete workspace/ }).click();
    await closeMenu();
    await workspacePage.waitForFunction((removed) => {
      const config = JSON.parse(localStorage.getItem('config'));
      return config.workspace_id !== removed &&
        !config.workspaces.some((workspace) => workspace.id === removed);
    }, persistentOrders);
    await selectWorkspace(persistentCustomers);
    await expectDatabase('b.db');
    await selectDatabase('b.db');
    await run(ordersQuery);
    await expectValue('37');

    // Old single-database installations still open the existing default OPFS file.
    await selectWorkspace('default');
    await setFreshDatabase(false);
    await run(`ATTACH 'test.db' AS legacy;
      CREATE TABLE legacy.legacy_probe(value TEXT);
      INSERT INTO legacy.legacy_probe VALUES ('legacy database');
      DETACH legacy;`);
    const legacyQuery = 'SELECT value FROM legacy_probe; -- legacy installation';
    await edit(legacyQuery);
    await workspacePage.evaluate(() => {
      const config = JSON.parse(localStorage.getItem('config'));
      for (const name of [
        'workspace_id', 'workspace_name', 'workspaces',
        'deleted_workspace_ids', 'read_only', 'database',
      ]) {
        delete config[name];
      }
      config.vfs = 'OPFS';
      localStorage.setItem('config', JSON.stringify(config));
    });
    await workspacePage.reload();
    await workspacePage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    assert.equal(
      await workspacePage.getByRole('button', {
        name: /^Workspace: Default \(/,
      })
        .count(),
      1,
    );
    assert.equal(
      await workspacePage.evaluate(() => ace.edit('ace_editor').getValue()),
      legacyQuery,
    );
    await run(legacyQuery);
    await expectValue("'legacy database'");
    const onlyWorkspace = await workspaceMenu();
    assert.equal(await onlyWorkspace.getByRole('button', {
      name: /^Delete workspace/,
    }).isDisabled(), true);
    await closeMenu();

    // A shared example opened in another tab survives further edits to the source.
    await workspacePage.getByRole('button', { name: 'Share', exact: true }).click();
    const sharedHref = await workspacePage.getByRole('link', {
      name: 'Embedded code in link',
    }).getAttribute('href');
    const sharedPage = await context.newPage();
    await sharedPage.goto(sharedHref);
    await sharedPage.waitForFunction(
      (sql) => globalThis.ace?.edit('ace_editor').getValue() === sql,
      legacyQuery,
    );
    await sharedPage.getByRole('button', { name: /^Workspace: Shared \(/ }).waitFor();
    const sharedId = await sharedPage.evaluate((sql) => {
      const saved = JSON.parse(localStorage.getItem('config'));
      if (saved.workspace_name === 'Shared' && saved.sql === sql) return saved.workspace_id;
      return saved.workspaces.find((workspace) =>
        workspace.name === 'Shared' && workspace.sql === sql,
      )?.id;
    }, legacyQuery);
    assert.ok(sharedId, 'The shared draft is saved even if another tab saved last');
    await edit('SELECT 971 AS original_after_share;');
    const afterSourceEdit = await config();
    assert.equal(afterSourceEdit.workspace_id, 'default');
    assert.equal(afterSourceEdit.workspaces.some((workspace) =>
      workspace.id === sharedId && workspace.sql === legacyQuery,
    ), true);
    await workspacePage.reload();
    await workspacePage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await selectWorkspace(sharedId);

    const sharedMenu = await workspaceMenu();
    workspacePage.once('dialog', (dialog) => dialog.accept());
    await sharedMenu.getByRole('button', { name: /^Delete workspace/ }).click();
    await closeMenu();
    await workspacePage.waitForFunction((id) => {
      const config = JSON.parse(localStorage.getItem('config'));
      return config.deleted_workspace_ids?.includes(id);
    }, sharedId);

    // A tab that still has the deleted workspace open cannot bring it back.
    await sharedPage.evaluate(() =>
      ace.edit('ace_editor').setValue('SELECT 977 AS stale_deleted_draft;', -1),
    );
    await sharedPage.close();
    const afterStaleSave = await config();
    assert.notEqual(afterStaleSave.workspace_id, sharedId);
    assert.equal(afterStaleSave.workspaces.some((workspace) =>
      workspace.id === sharedId,
    ), false);
    assert.equal(afterStaleSave.sql, 'SELECT 971 AS original_after_share;');

    // Switching refreshes an inactive draft changed by another open tab.
    const concurrentId = await createWorkspace('Concurrent draft');
    await edit('SELECT 981 AS original_inactive_draft;');
    await selectWorkspace('default');
    const otherPage = await context.newPage();
    await otherPage.goto(origin.href);
    await otherPage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await selectWorkspace(concurrentId, otherPage);
    const latestDraft = 'SELECT 983 AS updated_in_another_tab;';
    await edit(latestDraft, otherPage);
    await selectWorkspace(concurrentId);
    assert.equal(
      await workspacePage.evaluate(() => ace.edit('ace_editor').getValue()),
      latestDraft,
    );
    await otherPage.close();
  } finally {
    await context.close();
  }
}
