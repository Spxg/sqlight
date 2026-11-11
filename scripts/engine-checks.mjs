import assert from 'node:assert/strict';
import { chooseFiles, closeFiles, databaseGroup, openFiles } from './files-helpers.mjs';

export async function checkEngineSelection(page) {
  const engineSelect = page.getByRole('combobox', {
    name: 'SQLite Engine',
    exact: true,
  });

  async function selectEngine(value, savedValue) {
    await page
      .getByRole('button', { name: /^Workspace: / })
      .click();
    await engineSelect.selectOption(value);
    assert.equal(await engineSelect.inputValue(), value);
    await page.keyboard.press('Escape');
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
    await page.waitForFunction(
      (engine) => JSON.parse(localStorage.getItem('config'))?.engine === engine,
      savedValue,
    );
    const filename = await page.evaluate(() => JSON.parse(localStorage.getItem('config')).database);
    const files = await openFiles(page);
    if (!await databaseGroup(files, filename).count()) {
      await chooseFiles(page, [[filename, new Uint8Array()]]);
      await page.getByText('Database imported.', { exact: true }).waitFor();
    } else {
      await closeFiles(page);
    }
  }

  async function run(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await page.getByRole('button', { name: 'Run', exact: true }).click();
    await page
      .getByRole('tabpanel')
      .getByText('Finished', { exact: true })
      .waitFor();
  }

  // Existing installations stored the encryption choice as a boolean.
  await page.evaluate(() => sessionStorage.setItem('legacy-engine-check', '1'));
  await page.addInitScript(() => {
    if (sessionStorage.getItem('legacy-engine-check') !== '1') return;
    const config = JSON.parse(localStorage.getItem('config'));
    delete config.engine;
    config.multiple_ciphers = true;
    localStorage.setItem('config', JSON.stringify(config));
    sessionStorage.removeItem('legacy-engine-check');
  });
  await page.reload();
  await page
    .getByRole('button', { name: /^Workspace: / })
    .click();
  assert.equal(await engineSelect.inputValue(), 'SQLite3 Multiple Ciphers');
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'hidden' });

  await selectEngine('SQLCipher', 'SQLCipher');
  await run('PRAGMA cipher_version; SELECT vec_version() AS vec;');
  await page
    .getByRole('columnheader', { name: 'cipher_version', exact: true })
    .waitFor();
  assert.match(await page.locator('tbody').first().innerText(), /\d+\.\d+\./);

  await page.reload();
  await page.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
  await page
    .getByRole('button', { name: /^Workspace: / })
    .click();
  assert.equal(await engineSelect.inputValue(), 'SQLCipher');
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'hidden' });
  await run('PRAGMA cipher_version; SELECT 83 AS sqlcipher_after_reload;');
  await page
    .getByRole('columnheader', { name: 'cipher_version', exact: true })
    .waitFor();
  await page.getByRole('cell', { name: '83', exact: true }).waitFor();

  await selectEngine('SQLite', 'SQLite');
  await run("PRAGMA cipher_version; PRAGMA cipher; SELECT 'SQLite' AS engine;");
  assert.deepEqual(await page.getByRole('columnheader').allTextContents(), [
    'engine',
  ]);
  await page.getByRole('cell', { name: "'SQLite'", exact: true }).waitFor();

  await selectEngine('SQLite3 Multiple Ciphers', 'MultipleCiphers');
  await run("SELECT sqlite3mc_config('cipher') AS multiple_ciphers;");
  await page
    .getByRole('columnheader', { name: 'multiple_ciphers', exact: true })
    .waitFor();
  assert.match(await page.locator('tbody').innerText(), /^'[a-z0-9]+'$/i);

  // Switching back also exercises an already cached worker session.
  await selectEngine('SQLCipher', 'SQLCipher');
  await run('PRAGMA cipher_version;');
  await page
    .getByRole('columnheader', { name: 'cipher_version', exact: true })
    .waitFor();
  await selectEngine('SQLite', 'SQLite');
}

export async function checkWorkers(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url());
  origin.search = '';
  origin.hash = '';
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin.origin)
      ? route.continue()
      : route.abort(),
  );
  try {
    const workerPage = await context.newPage();
    await workerPage.goto(origin.href);
    await checkWorkerContext(workerPage);
  } finally {
    await context.close();
  }
}

async function checkWorkerContext(page) {
  const checked = await page.evaluate(async () => {
    async function startWorker(engine) {
      const worker = new Worker(`./${engine}_loader.js`, { type: 'module' });
      const pending = [];
      worker.onmessage = (event) => pending.shift()?.resolve(event.data);
      worker.onerror = (event) => {
        const error = new Error(`${engine}: Worker error: ${event.message}`);
        for (const response of pending.splice(0)) response.reject(error);
      };
      const receive = (label) =>
        new Promise((resolve, reject) => {
          const timeout = setTimeout(() => {
            pending.splice(pending.indexOf(response), 1);
            reject(new Error(`${engine}: Timed out waiting for ${label}`));
          }, 30_000);
          const response = {
            resolve(value) {
              clearTimeout(timeout);
              resolve(value);
            },
            reject(error) {
              clearTimeout(timeout);
              reject(error);
            },
          };
          pending.push(response);
        });
      const request = async (value) => {
        const label = typeof value === 'string' ? value : Object.keys(value)[0];
        const response = receive(label);
        worker.postMessage(value);
        return response;
      };
      try {
        const ready = await receive('Ready');
        if (ready !== 'Ready')
          throw new Error(
            `${engine}: Unexpected worker ready: ${JSON.stringify(ready)}`,
          );
      } catch (error) {
        worker.terminate();
        throw error;
      }
      return { request, terminate: () => worker.terminate() };
    }

    const checked = [];
    for (const engine of ['sqlite3', 'sqlite3mc', 'sqlcipher']) {
      const worker = await startWorker(engine);
      const request = worker.request;
      const unwrap = (response) => {
        const result = Object.values(response)[0];
        if (!Object.hasOwn(result, 'Ok'))
          throw new Error(`${engine}: ${JSON.stringify(response)}`);
        return result.Ok;
      };
      const run = (sql) => ({
        Run: { sql, embed: false, clear_on_prepare: false },
      });
      const sql = async (value) => {
        const result = unwrap(await request(run(value)));
        if (result.error)
          throw new Error(`${engine}: ${JSON.stringify(result.error)}`);
        return result;
      };
      const table = (result, column) => {
        const values = result.result.find((item) =>
          item.Step?.values?.columns.includes(column),
        )?.Step.values;
        if (!values) throw new Error(`${engine}: Missing column ${column}`);
        return values;
      };
      const value = (result, column) => {
        const values = table(result, column);
        return values.rows[0][values.columns.indexOf(column)];
      };
      const encryptedHeader = (data) => {
        if (
          new TextDecoder().decode(data.slice(0, 16)) === 'SQLite format 3\0'
        ) {
          throw new Error(`${engine}: Encrypted database has a plaintext header`);
        }
      };
      const sqliteErrorCode = (response) => {
        const error = response.Run.Ok?.error ?? response.Run.Err?.SQLite;
        return error && Object.values(error)[0].code;
      };

      async function checkReadOnly() {
        const database = { filename: `readonly-${engine}.db` };
        const open = async (read_only) => unwrap(
          await request({ Open: { ...database, read_only } }),
        );
        const key = engine === 'sqlite3' ? '' : "PRAGMA key='readonly-key'; ";
        await open(false);
        await sql(key + `PRAGMA journal_mode=PERSIST;
          CREATE TABLE protected(value);
          INSERT INTO protected VALUES (321);`);
        await open(true);
        await sql(key + 'SELECT value FROM protected;');
        unwrap(await request({
          Open: { ...database, read_only: true, workspace: 'readonly-other' },
        }));
        if (value(await sql('SELECT value FROM protected;'), 'value') !== '321')
          throw new Error(`${engine}: Workspace selection reset the shared cipher connection`);

        for (const statement of [
          'INSERT INTO protected VALUES (999);',
          'CREATE TABLE forbidden(value);',
          'PRAGMA query_only=OFF; UPDATE protected SET value=999;',
        ]) {
          const failed = await request(run(statement));
          if ((sqliteErrorCode(failed) & 0xff) !== 8)
            throw new Error(`${engine}: Read-only execution allowed ${statement}`);
        }

        await sql('PRAGMA query_only=ON; PRAGMA cache_size=-321;');
        const created = `created sp ace?#%你好-${engine}.db`;
        if (unwrap(await request({ CreateDb: created })) !== created)
          throw new Error(`${engine}: CreateDb changed the requested filename`);
        const createdFile = unwrap(await request({ DownloadFile: created }));
        if (createdFile.filename !== created || createdFile.data.length !== 0) {
          throw new Error(`${engine}: CreateDb did not leave the first database page for SQL`);
        }
        for (const filename of [created, database.filename]) {
          const duplicate = await request({ CreateDb: filename });
          if (!duplicate.CreateDb.Err?.CreateDb)
            throw new Error(`${engine}: CreateDb accepted an existing database`);
        }
        const createdAgain = unwrap(await request({ DownloadFile: created }));
        if (createdAgain.data.length !== createdFile.data.length
          || createdAgain.data.some((byte, index) => byte !== createdFile.data[index])) {
          throw new Error(`${engine}: Duplicate creation changed the existing database`);
        }
        const downloaded = unwrap(await request({ DownloadFile: database.filename }));
        if (engine !== 'sqlite3') encryptedHeader(downloaded.data);
        const bundle = unwrap(await request({ DownloadBundle: [database.filename] }));
        if (bundle.filename !== `${database.filename}.zip`
          || !bundle.files.some((file) => file.filename === `${database.filename}-journal`)) {
          throw new Error(`${engine}: Bundle download omitted the retained journal`);
        }
        const bundledMain = bundle.files.find((file) => file.filename === database.filename).data;
        if (bundledMain.length !== downloaded.data.length
          || bundledMain.some((byte, index) => byte !== downloaded.data[index])) {
          throw new Error(`${engine}: Bundle download changed the encrypted database bytes`);
        }
        const snapshot = unwrap(await request('ListFiles'));
        if (snapshot.connection?.filename !== database.filename || !snapshot.connection.read_only
          || snapshot.in_transaction || !snapshot.files.find((file) => file.filename === database.filename)?.in_use) {
          throw new Error(`${engine}: File snapshot did not describe the active read-only connection`);
        }
        for (const file of bundle.files) {
          const listed = snapshot.files.find((entry) => entry.filename === file.filename);
          if (!listed?.in_use || listed.size !== file.data.length)
            throw new Error(`${engine}: File snapshot omitted a log's size or connection usage`);
        }
        const blockedDelete = await request({ DeleteFiles: [created, database.filename] });
        if (!blockedDelete.DeleteFiles.Err?.FilesInUse
          || !unwrap(await request('ListDbs')).includes(created)) {
          throw new Error(`${engine}: Deleting an active file partially deleted another database`);
        }
        const missingDelete = await request({ DeleteFiles: [created, 'missing-delete.db'] });
        if (missingDelete.DeleteFiles.Err?.MissingDatabase !== 'missing-delete.db'
          || !unwrap(await request('ListDbs')).includes(created)) {
          throw new Error(`${engine}: DeleteFiles did not validate all missing files before deletion`);
        }
        unwrap(await request({ CreateDb: 'unrelated-delete.db' }));
        unwrap(await request({ DeleteFiles: ['unrelated-delete.db'] }));
        const preserved = await sql(`PRAGMA query_only;
          PRAGMA cache_size; SELECT value FROM protected;`);
        if (value(preserved, 'query_only') !== '1'
          || value(preserved, 'cache_size') !== '-321'
          || value(preserved, 'value') !== '321') {
          throw new Error(`${engine}: Read-only export reset the connection`);
        }
        const repeated = unwrap(await request({ DownloadFile: database.filename }));
        if (downloaded.data.length !== repeated.data.length
          || downloaded.data.some((byte, index) => byte !== repeated.data[index])) {
          throw new Error(`${engine}: Read-only export changed database bytes`);
        }

        // The retained PERSIST journal is cold but must remain untouched.
        unwrap(await request({
          Open: { ...database, filename: `${database.filename}-journal`, read_only: true },
        }));
        const journal = unwrap(await request('DownloadDb')).data;
        if (!journal.length || journal[0] !== 0)
          throw new Error(`${engine}: Read-only export removed the journal`);
        const bundledJournal = bundle.files.find((file) => file.filename === `${database.filename}-journal`).data;
        if (journal.length !== bundledJournal.length
          || journal.some((byte, index) => byte !== bundledJournal[index])) {
          throw new Error(`${engine}: Bundle download changed the journal`);
        }

        await open(true);
        const companion = `${database.filename}-shared.db`;
        unwrap(await request({
          LoadDbs: { files: [{ filename: companion, data: downloaded.data }], replace: false },
        }));
        await sql(key + 'BEGIN; SELECT value FROM protected;');
        const blockedCreateName = `readonly-blocked-${engine}.db`;
        const blockedCreate = await request({ CreateDb: blockedCreateName });
        if (unwrap(blockedCreate) !== blockedCreateName
          || !unwrap(await request('ListDbs')).includes(blockedCreateName)
          || value(await sql('SELECT value FROM protected;'), 'value') !== '321') {
          throw new Error(`${engine}: CreateDb changed the current transaction or key`);
        }
        const duringTransaction = unwrap(await request('ListFiles'));
        if (!duringTransaction.in_transaction
          || (await request({ DeleteFiles: [blockedCreateName] })).DeleteFiles.Err !== 'ActiveTransaction'
          || !unwrap(await request('ListDbs')).includes(blockedCreateName)) {
          throw new Error(`${engine}: Files management discarded the current transaction`);
        }
        const reset = unwrap(await request({
          Run: {
            sql: `SELECT count(*) AS remaining FROM sqlite_schema;
              PRAGMA compile_options; SELECT sqlite_version() AS version;`,
            embed: false,
            clear_on_prepare: true,
          },
        }));
        if (reset.error || value(reset, 'remaining') !== '0'
          || !table(reset, 'compile_options').rows.length
          || !/^'\d+\./.test(value(reset, 'version'))) {
          throw new Error(`${engine}: Fresh read-only mode did not provide an empty readable database`);
        }
        if ((sqliteErrorCode(await request(run('PRAGMA query_only=OFF; CREATE TABLE forbidden(value);')))
          & 0xff) !== 8) {
          throw new Error(`${engine}: Fresh read-only mode allowed writing`);
        }
        const retainedCompanion = unwrap(await request({ DownloadFile: companion })).data;
        if (retainedCompanion.length !== downloaded.data.length
          || retainedCompanion.some((byte, index) => byte !== downloaded.data[index])) {
          throw new Error(`${engine}: Fresh read-only mode changed another shared database`);
        }

        unwrap(await request({ LoadDb: { data: downloaded.data } }));
        if (value(await sql(key + 'SELECT value FROM protected;'), 'value') !== '321')
          throw new Error(`${engine}: Read-only database import changed rows`);
        if ((sqliteErrorCode(await request(run('INSERT INTO protected VALUES (999);')))
          & 0xff) !== 8) {
          throw new Error(`${engine}: Import reopened a read-only database for writing`);
        }

        await open(false);
        const writable = await sql(key + `INSERT INTO protected VALUES (456);
          SELECT count(*) AS count FROM protected;`);
        if (value(writable, 'count') !== '2')
          throw new Error(`${engine}: Switching back to read-write failed`);
        await sql('PRAGMA query_only=ON; PRAGMA cache_size=-654;');
        const writableDownload = unwrap(await request({ DownloadFile: database.filename }));
        if (engine !== 'sqlite3') encryptedHeader(writableDownload.data);
        const writablePreserved = await sql(`PRAGMA query_only;
          PRAGMA cache_size; SELECT count(*) AS count FROM protected;`);
        if (value(writablePreserved, 'query_only') !== '1'
          || value(writablePreserved, 'cache_size') !== '-654'
          || value(writablePreserved, 'count') !== '2') {
          throw new Error(`${engine}: Writable file download reset the connection or key`);
        }

        unwrap(await request({ CreateDb: 'empty-readonly.db' }));
        unwrap(await request({
          Open: { filename: 'empty-readonly.db', read_only: true },
        }));
        const empty = await sql('PRAGMA integrity_check;');
        if (value(empty, 'integrity_check') !== "'ok'")
          throw new Error(`${engine}: Read-only workspace initialized an invalid database`);
        const initialized = unwrap(await request('DownloadDb')).data;
        if (initialized.length && new TextDecoder().decode(initialized.slice(0, 16)) !== 'SQLite format 3\0')
          throw new Error(`${engine}: Empty read-only workspace was not initialized`);
        unwrap(await request({ Open: { filename: created, read_only: true } }));
        if (value(await sql('SELECT count(*) AS count FROM sqlite_schema;'), 'count') !== '0')
          throw new Error(`${engine}: Newly created database did not have an empty schema`);
        if ((sqliteErrorCode(await request(run('CREATE TABLE forbidden(value);'))) & 0xff) !== 8)
          throw new Error(`${engine}: Selecting a new database lost the read-only setting`);
        if (engine !== 'sqlite3') {
          unwrap(await request({ Open: { filename: created } }));
          await sql(`PRAGMA key='new-database-key'; CREATE TABLE keyed_new(value);
            INSERT INTO keyed_new VALUES (123);`);
          encryptedHeader(unwrap(await request({ DownloadFile: created })).data);
        }
      }

      async function checkDownloadWal() {
        const main = 'download-main.db';
        const target = 'download-wal.db';
        const key = engine === 'sqlite3' ? '' : "PRAGMA key='download-key'; ";
        const attachKey = engine === 'sqlite3' ? '' : "KEY 'download-key'";
        unwrap(await request({ Open: { filename: main } }));
        await sql(key + `CREATE TABLE connection_marker(value);
          INSERT INTO connection_marker VALUES (123);
          ATTACH '${target}' AS "wal""target" ${attachKey};
          PRAGMA "wal""target".locking_mode=EXCLUSIVE;
          PRAGMA "wal""target".journal_mode=WAL;
          CREATE TABLE "wal""target".downloaded(value);
          INSERT INTO "wal""target".downloaded VALUES (456);
          PRAGMA query_only=ON; PRAGMA cache_size=-444;`);
        const before = unwrap(await request({ DownloadFile: `${target}-wal` }));
        if (!before.data.length)
          throw new Error(`${engine}: WAL download regression did not create a WAL`);
        const bundle = unwrap(await request({ DownloadBundle: [target] }));
        const bundledWal = bundle.files.find((file) => file.filename === `${target}-wal`)?.data;
        if (!bundledWal || bundledWal.length !== before.data.length
          || bundledWal.some((byte, index) => byte !== before.data[index])) {
          throw new Error(`${engine}: Bundle download changed or checkpointed the WAL`);
        }
        const all = unwrap(await request({ DownloadBundle: [main, target] }));
        if (all.filename !== 'databases.zip' || !all.files.some((file) => file.filename === main))
          throw new Error(`${engine}: Multiple database bundle omitted a main file`);
        const downloaded = unwrap(await request({ DownloadFile: target }));
        if (downloaded.filename !== target)
          throw new Error(`${engine}: WAL download changed the selected filename`);
        if (engine !== 'sqlite3') encryptedHeader(downloaded.data);
        const after = unwrap(await request({ DownloadFile: `${target}-wal` }));
        if (after.data.length)
          throw new Error(`${engine}: WAL download did not checkpoint the selected database`);
        const preserved = await sql(`PRAGMA query_only; PRAGMA cache_size;
          SELECT value AS marker FROM connection_marker;
          SELECT value FROM "wal""target".downloaded;`);
        if (value(preserved, 'query_only') !== '1'
          || value(preserved, 'cache_size') !== '-444'
          || value(preserved, 'marker') !== '123'
          || value(preserved, 'value') !== '456') {
          throw new Error(`${engine}: WAL download changed the current connection or ATTACH`);
        }
        unwrap(await request('Close'));
        const peer = await startWorker(engine);
        try {
          unwrap(await peer.request({ CreateDb: 'download-reader.db' }));
          unwrap(await peer.request({ Open: { filename: 'download-reader.db', read_only: true } }));
          const restoredName = 'download-restored.db';
          unwrap(await peer.request({ LoadDbs: {
            files: bundle.files.map((file) => ({
              filename: restoredName + file.filename.slice(target.length), data: file.data,
            })).reverse(),
            replace: false,
          } }));
          unwrap(await peer.request({ Open: { filename: restoredName, read_only: true } }));
          const restored = unwrap(await peer.request(run(key + 'SELECT value FROM downloaded;')));
          if (restored.error || value(restored, 'value') !== '456')
            throw new Error(`${engine}: Read-only bundle restore lost the WAL data`);
          if ((sqliteErrorCode(await peer.request(run('INSERT INTO downloaded VALUES (999);'))) & 0xff) !== 8)
            throw new Error(`${engine}: WAL bundle restore lost the read-only setting`);
          const restoredBundle = unwrap(await peer.request({ DownloadBundle: [restoredName] }));
          const restoredWal = restoredBundle.files.find((file) => file.filename === `${restoredName}-wal`)?.data;
          if (!restoredWal || restoredWal.length !== bundledWal.length
            || restoredWal.some((byte, index) => byte !== bundledWal[index])) {
            throw new Error(`${engine}: Read-only WAL recovery changed the archived WAL`);
          }
          unwrap(await peer.request('Close'));
        } finally {
          peer.terminate();
        }
      }

      async function checkFilesystem() {
        const disconnected = unwrap(await request('ListFiles'));
        if (disconnected.connection || disconnected.in_transaction)
          throw new Error(`${engine}: Files listing opened an SQL connection`);
        const group = ['fs-delete-group.db', 'fs-delete-group.db-journal',
          'fs-delete-group.db-wal', 'fs-delete-group.db-shm'];
        unwrap(await request({ LoadDbs: {
          files: group.map((filename) => ({ filename, data: new Uint8Array() })), replace: false,
        } }));
        if (unwrap(await request('ListFiles')).connection)
          throw new Error(`${engine}: Uploading files opened an SQL connection`);
        const archived = unwrap(await request({ DownloadBundle: [group[0]] }));
        if (archived.files.length !== 4)
          throw new Error(`${engine}: Files archive omitted an empty sidecar`);
        const deleted = unwrap(await request({ DeleteFiles: [group[2]] }));
        if (deleted.length !== 4 || unwrap(await request('ListFiles')).files.some((file) => group.includes(file.filename)))
          throw new Error(`${engine}: Deleting a sidecar did not delete its complete database group`);
        unwrap(await request({ Open: { filename: 'fs-orphan-source.db' } }));
        await sql(`ATTACH 'fs-orphan.db-wal' AS orphan;
          CREATE TABLE orphan.marker(value); DETACH orphan;`);
        unwrap(await request('Close'));
        const orphan = unwrap(await request({ DownloadBundle: ['fs-orphan.db-wal'] }));
        if (orphan.files.length !== 1 || orphan.files[0].filename !== 'fs-orphan.db-wal')
          throw new Error(`${engine}: Orphan file could not be archived independently`);
        if (unwrap(await request('ListFiles')).connection)
          throw new Error(`${engine}: Downloading an orphan opened an SQL connection`);
        unwrap(await request({ DeleteFiles: ['fs-orphan.db-wal'] }));
        for (const suffix of ['-journal', '-wal', '-shm']) {
          const name = `fs-orphan-${suffix.slice(1)}.db`;
          unwrap(await request({ Open: { filename: 'fs-orphan-source.db' } }));
          await sql(`ATTACH '${name}${suffix}' AS orphan;
            CREATE TABLE orphan.marker(value); DETACH orphan;`);
          unwrap(await request('Close'));
          const blocked = await request({ Open: { filename: name } });
          if (!blocked.Open.Err?.CreateDb || unwrap(await request('ListDbs')).includes(name))
            throw new Error(`${engine}: Opening a missing main ignored its orphan ${suffix}`);
          unwrap(await request({ DeleteFiles: [`${name}${suffix}`] }));
        }
        const readOnlyName = 'auto-readonly.db';
        unwrap(await request({ Open: { filename: readOnlyName, read_only: true } }));
        const empty = await sql(`SELECT count(*) AS count FROM sqlite_schema;
          PRAGMA integrity_check; SELECT sqlite_version() AS version;`);
        if (value(empty, 'count') !== '0' || value(empty, 'integrity_check') !== "'ok'")
          throw new Error(`${engine}: Read-only Open did not initialize a missing empty database`);
        const readOnlyData = unwrap(await request({ DownloadFile: readOnlyName })).data;
        if (new TextDecoder().decode(readOnlyData.slice(0, 16)) !== 'SQLite format 3\0')
          throw new Error(`${engine}: Read-only Open left the new database uninitialized`);
        if ((sqliteErrorCode(await request(run('CREATE TABLE forbidden(value);'))) & 0xff) !== 8)
          throw new Error(`${engine}: Automatically created read-only database allowed writing`);
        unwrap(await request('Close'));
        unwrap(await request({ DeleteFiles: [readOnlyName] }));

        unwrap(await request({ Open: { filename: 'default.db' } }));
        if (unwrap(await request({ DownloadFile: 'default.db' })).data.length !== 0)
          throw new Error(`${engine}: Automatic Open wrote plaintext pages before the first key`);
        unwrap(await request('Close'));
        unwrap(await request({ DeleteFiles: ['default.db'] }));
        if ((await request({ DownloadFile: 'default.db' })).DownloadDb.Err?.MissingDatabase !== 'default.db'
          || (await request({ DownloadBundle: ['default.db'] })).DownloadBundle.Err?.MissingDatabase !== 'default.db'
          || unwrap(await request('ListFiles')).files.some((file) => file.filename === 'default.db')) {
          throw new Error(`${engine}: A filesystem operation recreated a deleted database`);
        }
        unwrap(await request('Close'));
        const peer = await startWorker(engine);
        try {
          const snapshot = unwrap(await peer.request('ListFiles'));
          if (snapshot.connection || snapshot.files.some((file) => file.filename === 'default.db'))
            throw new Error(`${engine}: Listing files after restart recreated a missing database`);
          unwrap(await peer.request({ Open: { filename: 'default.db' } }));
          if (unwrap(await peer.request({ DownloadFile: 'default.db' })).data.length !== 0)
            throw new Error(`${engine}: Open could not recreate a deleted database after restart`);
          unwrap(await peer.request('Close'));
        } finally {
          peer.terminate();
        }
      }

      try {
        const previousFiles = unwrap(await request('ListFiles'));
        if (checked.length) {
          if (!previousFiles.files.some((file) => file.filename === 'cross-engine.db'))
            throw new Error(`${engine}: Backend switch did not expose the shared filesystem`);
          unwrap(await request({ Open: { filename: 'cross-engine.db', read_only: true } }));
          const shared = await sql('SELECT value FROM cross_engine;');
          if (value(shared, 'value') !== '83')
            throw new Error(`${engine}: Backend switch changed the shared database contents`);
          unwrap(await request('Close'));
          unwrap(await request({ DeleteFiles: previousFiles.files.map((file) => file.filename) }));
        }
        const initialFiles = unwrap(await request('ListFiles'));
        if (initialFiles.files.length || initialFiles.connection)
          throw new Error(`${engine}: Listing a new filesystem created a database or connection`);
        unwrap(
          await request({ Open: { filename: 'smoke.db' } }),
        );
        const untouched = unwrap(await request({ DownloadBundle: unwrap(await request('ListDbs')) }));
        if (untouched.files.length !== 1 || untouched.files[0].data.length !== 0)
          throw new Error(`${engine}: Opening an unwritten database unexpectedly wrote its pages`);
        unwrap(await request({ LoadDbs: {
          files: [{ filename: 'empty-import.db', data: untouched.files[0].data }], replace: false,
        } }));
        const emptyAll = unwrap(await request({ DownloadBundle: unwrap(await request('ListDbs')) }));
        if (emptyAll.filename !== 'databases.zip' || emptyAll.files.length !== 2
          || emptyAll.files.some((file) => file.data.length !== 0)) {
          throw new Error(`${engine}: All-databases bundle did not preserve empty databases`);
        }
        unwrap(await request({ Open: { filename: 'empty-import.db', read_only: true } }));
        const emptyImported = await sql('SELECT count(*) AS count FROM sqlite_schema; PRAGMA integrity_check;');
        if (value(emptyImported, 'count') !== '0' || value(emptyImported, 'integrity_check') !== "'ok'")
          throw new Error(`${engine}: Empty bundle import was not a readable SQLite database`);
        unwrap(await request({ Open: { filename: 'smoke.db' } }));
        if (engine === 'sqlcipher') {
          const version = await sql('PRAGMA cipher_version;');
          if (!/^'\d+\.\d+\./.test(value(version, 'cipher_version')))
            throw new Error('SQLCipher did not report its version');
        }
        const result = await sql(
          `; -- comment
          SELECT '你好' AS unicode, vec_version() AS vec;
          ; SELECT 2 AS second;`,
        );
        if (result.result.filter((item) => item.Step).length !== 2)
          throw new Error(`${engine}: SQL statements were skipped`);
        if (value(result, 'unicode') !== "'你好'")
          throw new Error(`${engine}: Unicode result changed`);
        if (!/^'v?\d+\./.test(value(result, 'vec')))
          throw new Error(`${engine}: sqlite-vec did not report its version`);
        await sql(
          `CREATE TABLE persistent(value);
          INSERT INTO persistent VALUES (7);
          BEGIN;
          INSERT INTO persistent VALUES (8);`,
        );
        const transaction = await request('DownloadDb');
        if (!transaction.DownloadDb.Err)
          throw new Error(`${engine}: Export accepted an uncommitted transaction`);
        const transactionCreate = await request({ CreateDb: 'blocked-create.db' });
        if (unwrap(transactionCreate) !== 'blocked-create.db'
          || !unwrap(await request('ListDbs')).includes('blocked-create.db')) {
          throw new Error(`${engine}: CreateDb could not create an unrelated database during a transaction`);
        }
        const unsafeClose = await request('Close');
        if (unsafeClose.Close.Err !== 'ActiveTransaction')
          throw new Error(`${engine}: Close discarded an active transaction`);
        for (const options of [
          { filename: 'smoke.db', read_only: true },
          { filename: 'other.db' },
        ]) {
          const switched = await request({ Open: options });
          if (switched.Open.Err !== 'ActiveTransaction')
            throw new Error(`${engine}: Database switch discarded an open transaction`);
          if (value(await sql('SELECT count(*) AS count FROM persistent;'), 'count') !== '2')
            throw new Error(`${engine}: Failed switch changed the current transaction`);
        }
        if (unwrap(await request('ListDbs')).includes('other.db'))
          throw new Error(`${engine}: A rejected switch created its missing database`);
        await sql('ROLLBACK;');
        const exported = unwrap(await request('DownloadDb'));
        unwrap(await request({ LoadDb: { data: exported.data } }));
        const imported = await sql('SELECT * FROM persistent;');
        if (value(imported, 'value') !== '7')
          throw new Error(`${engine}: Database transfer changed rows`);
        await sql('BEGIN; INSERT INTO persistent VALUES (9);');
        const importTransaction = await request({ LoadDb: { data: exported.data } });
        if (importTransaction.LoadDb.Err !== 'ActiveTransaction')
          throw new Error(`${engine}: Import discarded an open transaction`);
        await sql('ROLLBACK;');

        for (const size of [6, 513]) {
          const invalid = await request({
            LoadDb: { data: new Uint8Array(size) },
          });
          if (!invalid.LoadDb.Err)
            throw new Error(`${engine}: Import accepted an invalid ${size}-byte file`);
          if (value(await sql('SELECT value FROM persistent;'), 'value') !== '7')
            throw new Error(`${engine}: Invalid import replaced the current database`);
        }

        if (engine !== 'sqlite3') {
          unwrap(
            await request({
              Open: { filename: 'encrypted.db' },
            }),
          );
          await sql(
            `PRAGMA key='smoke-key';
            CREATE TABLE secret(value);
            INSERT INTO secret VALUES (456);
            CREATE TEMP TABLE legacy_download_marker(value);
            INSERT INTO legacy_download_marker VALUES (654);
            PRAGMA cache_size=-876;`,
          );
          const encrypted = unwrap(await request('DownloadDb'));
          encryptedHeader(encrypted.data);
          const preserved = await sql(`SELECT value FROM secret;
            SELECT value AS marker FROM legacy_download_marker; PRAGMA cache_size;`);
          if (value(preserved, 'value') !== '456' || value(preserved, 'marker') !== '654'
            || value(preserved, 'cache_size') !== '-876') {
            throw new Error(`${engine}: Legacy download reset the connection or encryption key`);
          }
          unwrap(await request({ LoadDb: { data: encrypted.data } }));
          if (engine === 'sqlcipher') {
            // PRAGMA key alone succeeds even with a wrong passphrase. Reading
            // the schema forces SQLCipher to decrypt and validate the first page.
            const wrongKey = await request(
              run("PRAGMA key='wrong-key'; SELECT count(*) FROM sqlite_master;"),
            );
            const error = wrongKey.Run.Ok?.error ?? wrongKey.Run.Err?.SQLite;
            if (!error || Object.values(error)[0].code !== 26)
              throw new Error(`Wrong key did not fail: ${JSON.stringify(wrongKey)}`);
            unwrap(await request({ LoadDb: { data: encrypted.data } }));
          }
          const decrypted = await sql(
            "PRAGMA key='smoke-key'; SELECT value, vec_version() AS vec FROM secret;",
          );
          if (value(decrypted, 'value') !== '456')
            throw new Error(`${engine}: Encrypted database transfer changed rows`);
          if (!/^'v?\d+\./.test(value(decrypted, 'vec')))
            throw new Error(`${engine}: sqlite-vec failed on an encrypted database`);
        }
        unwrap(
          await request({
            Open: { filename: `smoke-${engine}.db` },
          }),
        );
        const key = engine === 'sqlcipher' ? "PRAGMA key='opfs-key'; " : '';
        await sql(
          key +
            `CREATE TABLE IF NOT EXISTS persisted(value);
            DELETE FROM persisted;
            INSERT INTO persisted VALUES (123);`,
        );
        unwrap(await request('Close'));
        unwrap(
          await request({
            Open: { filename: `smoke-${engine}.db` },
          }),
        );
        const persistent = await sql(
          key + 'SELECT value, vec_version() AS vec FROM persisted;',
        );
        if (value(persistent, 'value') !== '123')
          throw new Error(`${engine}: OPFS did not persist data`);
        if (!/^'v?\d+\./.test(value(persistent, 'vec')))
          throw new Error(`${engine}: sqlite-vec failed after reopening OPFS`);
        if (engine === 'sqlcipher') {
          const encrypted = unwrap(await request('DownloadDb'));
          encryptedHeader(encrypted.data);
          unwrap(await request({ LoadDb: { data: encrypted.data } }));
          const restored = await sql(
            key + 'SELECT value, vec_version() AS vec FROM persisted;',
          );
          if (value(restored, 'value') !== '123')
            throw new Error('SQLCipher: Encrypted OPFS transfer changed rows');
          if (!/^'v?\d+\./.test(value(restored, 'vec')))
            throw new Error('SQLCipher: sqlite-vec failed after OPFS import');
        }

        const baseline = unwrap(await request('DownloadDb'));
        for (const mode of ['TRUNCATE', 'PERSIST']) {
          const journal = await sql(
            key + `PRAGMA journal_mode=${mode};
            INSERT INTO persisted VALUES (456);`,
          );
          if (value(journal, 'journal_mode') !== `'${mode.toLowerCase()}'`)
            throw new Error(`${engine}: Failed to enable ${mode} journaling`);

          const downloaded = unwrap(await request('DownloadDb'));
          if (!downloaded.data.length)
            throw new Error(`${engine}: ${mode} journal prevented OPFS export`);

          await sql(key + `PRAGMA journal_mode=${mode};
            INSERT INTO persisted VALUES (678);`);
          // Retained journals must not block replacing the main database.
          unwrap(await request({ LoadDb: { data: baseline.data } }));
          const imported = table(
            await sql(key + 'SELECT value FROM persisted;'),
            'value',
          );
          if (imported.rows.length !== 1 || imported.rows[0][0] !== '123')
            throw new Error(`${engine}: ${mode} journal prevented OPFS import`);

          await sql(`PRAGMA journal_mode=${mode};
            INSERT INTO persisted VALUES (789);`);
          const reset = unwrap(
            await request({
              Run: {
                sql: key + `SELECT count(*) AS remaining
                  FROM sqlite_master WHERE name='persisted';`,
                embed: false,
                clear_on_prepare: true,
              },
            }),
          );
          if (value(reset, 'remaining') !== '0')
            throw new Error(`${engine}: ${mode} journal survived dropping context`);
          unwrap(await request({ LoadDb: { data: baseline.data } }));
        }

        unwrap(await request('Close'));
        const peer = await startWorker(engine);
        try {
          const persistent = {
            Open: { filename: `smoke-${engine}.db` },
          };
          unwrap(await peer.request(persistent));
          const shared = unwrap(
            await peer.request(run(key + 'SELECT value FROM persisted;')),
          );
          if (value(shared, 'value') !== '123')
            throw new Error(`${engine}: OPFS data changed after releasing locks`);

          const blocked = await request(persistent);
          if (blocked.Open.Err !== 'OpfsSAHPoolOpened')
            throw new Error(`${engine}: OPFS allowed two workers to hold locks`);
          unwrap(await peer.request('Close'));
          unwrap(await request(persistent));
          const resumed = await sql(key + 'SELECT value FROM persisted;');
          if (value(resumed, 'value') !== '123')
            throw new Error(`${engine}: OPFS resume did not restore the database`);
        } finally {
          peer.terminate();
        }

        // A nonempty WAL must survive a refused export, even after SQLite closes.
        await sql(`ATTACH 'smoke-${engine}.db-wal' AS retained;
          CREATE TABLE retained.marker(value);
          INSERT INTO retained.marker VALUES (999);
          DETACH retained;`);
        const unsafeExport = await request('DownloadDb');
        if (!unsafeExport.DownloadDb.Err)
          throw new Error(`${engine}: Export discarded a nonempty WAL`);
        unwrap(await request({
          Open: { filename: `smoke-${engine}.db`, read_only: true },
        }));
        const unsafeReadOnlyExport = await request('DownloadDb');
        if (!unsafeReadOnlyExport.DownloadDb.Err)
          throw new Error(`${engine}: Read-only export ignored a nonempty WAL`);
        const unsafeReadOnlyFile = await request({ DownloadFile: `smoke-${engine}.db` });
        if (!unsafeReadOnlyFile.DownloadDb.Err)
          throw new Error(`${engine}: Read-only file download ignored a nonempty WAL`);
        unwrap(await request({
          Open: { filename: `smoke-${engine}.db-wal` },
        }));
        const retained = await sql(key + 'SELECT value FROM marker;');
        if (value(retained, 'value') !== '999')
          throw new Error(`${engine}: Failed export changed WAL contents`);
        unwrap(await request({
          Open: { filename: `smoke-${engine}.db` },
        }));
        unwrap(await request({
          Run: { sql: key + 'SELECT 1;', embed: false, clear_on_prepare: true },
        }));
        unwrap(await request({ LoadDb: { data: baseline.data } }));
        await checkReadOnly();
        unwrap(await request('Close'));

        {
          const prefix = 'shared';
          const source = { filename: `${prefix}-source.db`, workspace: `${prefix}-source` };
          const aName = `${prefix}-a.db`;
          const bName = `${prefix}-b.db`;
          unwrap(await request({ Open: source }));
          await sql(`CREATE TABLE shared_data(value);
            INSERT INTO shared_data VALUES (1);`);
          for (const suffix of ['-journal', '-wal', '-shm']) {
            const reserved = `reserved-${suffix.slice(1)}.db`;
            await sql(`ATTACH '${reserved}${suffix}' AS occupied;
              CREATE TABLE occupied.marker(value); DETACH occupied;`);
            const before = unwrap(await request({ DownloadFile: `${reserved}${suffix}` })).data;
            if (!(await request({ CreateDb: reserved })).CreateDb.Err?.CreateDb)
              throw new Error(`${engine}: CreateDb ignored an existing ${suffix} file`);
            const after = unwrap(await request({ DownloadFile: `${reserved}${suffix}` })).data;
            if (before.length !== after.length || before.some((byte, index) => byte !== after[index]))
              throw new Error(`${engine}: CreateDb changed an existing ${suffix} file`);
          }
          const beforeInvalidNames = unwrap(await request('ListDbs'));
          for (const filename of ['', '.', '..', 'bad/name.db', 'bad\\name.db', 'bad\0name.db',
            'bad.db-journal', 'bad.db-wal', 'bad.db-shm']) {
            if (!(await request({ CreateDb: filename })).CreateDb.Err)
              throw new Error(`${engine}: CreateDb accepted an invalid filename`);
          }
          if (JSON.stringify(unwrap(await request('ListDbs'))) !== JSON.stringify(beforeInvalidNames))
            throw new Error(`${engine}: Invalid CreateDb requests left partial files`);
          const first = unwrap(await request('DownloadDb')).data;
          await sql('UPDATE shared_data SET value=2;');
          const second = unwrap(await request('DownloadDb')).data;

          await sql(`CREATE TEMP TABLE download_marker(value);
            INSERT INTO download_marker VALUES (123);
            PRAGMA query_only=ON; PRAGMA cache_size=-777;`);
          const loaded = unwrap(await request({
            LoadDbs: {
              files: [{ filename: aName, data: first }, { filename: bName, data: second }],
              replace: false,
            },
          }));
          if (JSON.stringify(loaded) !== JSON.stringify([aName, bName]))
            throw new Error(`${engine}: Batch upload did not preserve database names`);
          const legacy = unwrap(await request('DownloadDb'));
          if (legacy.filename !== source.filename || legacy.data.length !== second.length
            || legacy.data.some((byte, index) => byte !== second[index])) {
            throw new Error(`${engine}: Legacy download changed the current database`);
          }
          const downloadedB = unwrap(await request({ DownloadFile: bName }));
          if (downloadedB.filename !== bName || downloadedB.data.length !== second.length
            || downloadedB.data.some((byte, index) => byte !== second[index])) {
            throw new Error(`${engine}: Downloading another file changed its name or bytes`);
          }
          const retained = await sql(`PRAGMA cache_size; PRAGMA query_only;
            PRAGMA database_list; SELECT value AS marker FROM download_marker;`);
          if (value(retained, 'cache_size') !== '-777' || value(retained, 'query_only') !== '1'
            || value(retained, 'marker') !== '123'
            || !table(retained, 'file').rows.some((row) => row.includes(`'${source.filename}'`))) {
            throw new Error(`${engine}: Upload or download reset or changed the current connection`);
          }
          await sql('PRAGMA query_only=OFF;');

          for (const files of [
            [{ filename: aName, data: first }],
            [{ filename: 'duplicate.db', data: first }, { filename: 'duplicate.db', data: second }],
            [{ filename: 'not-written.db', data: first }, { filename: 'invalid.db', data: new Uint8Array(6) }],
            [{ filename: 'bad/name.db', data: first }],
            [{ filename: 'orphan.db-wal', data: new Uint8Array() }],
            [{ filename: 'orphan-good.db', data: first }, { filename: 'absent.db-journal', data: new Uint8Array() }],
          ]) {
            const failed = await request({ LoadDbs: { files, replace: false } });
            if (!failed.LoadDbs.Err)
              throw new Error(`${engine}: Batch upload accepted invalid or conflicting files`);
          }
          const names = unwrap(await request('ListDbs'));
          if (!names.includes(aName) || !names.includes(bName)
            || names.includes('duplicate.db') || names.includes('not-written.db') || names.includes('orphan-good.db')) {
            throw new Error(`${engine}: Batch validation left a partial import`);
          }
          const longMain = `${'名'.repeat(163)}.db`;
          const oversizedMain = `${'名'.repeat(166)}.db`;
          for (const files of [
            [{ filename: aName, data: second }, { filename: oversizedMain, data: first }],
            [{ filename: aName, data: second }, { filename: longMain, data: first },
              { filename: `${longMain}-journal`, data: new Uint8Array() }],
          ]) {
            const rejected = await request({ LoadDbs: { files, replace: true } });
            if (!/499 UTF-8 bytes/.test(rejected.LoadDbs.Err?.LoadDb ?? ''))
              throw new Error(`${engine}: Import did not preflight the transfer filename limit`);
            const retained = unwrap(await request({ DownloadFile: aName })).data;
            if (retained.length !== first.length || retained.some((byte, index) => byte !== first[index])
              || unwrap(await request('ListDbs')).includes(longMain)) {
              throw new Error(`${engine}: A later oversized filename partially replaced the import batch`);
            }
          }
          unwrap(await request({ Open: { filename: longMain } }));
          await sql(`PRAGMA journal_mode=PERSIST;
            CREATE TABLE long_name(value); INSERT INTO long_name VALUES (42);`);
          const longSnapshot = unwrap(await request('ListFiles'));
          const longJournal = longSnapshot.files.find((file) => file.filename === `${longMain}-journal`);
          if (!longJournal?.size)
            throw new Error(`${engine}: File listing could not read a long generated journal name`);
          const longBundle = unwrap(await request({ DownloadBundle: [longMain] }));
          const journal = longBundle.files.find((file) => file.filename === `${longMain}-journal`);
          if (journal?.data.length !== longJournal.size)
            throw new Error(`${engine}: Bundle download omitted a long generated journal`);
          unwrap(await request({ Open: source }));
          const journalMagic = new Uint8Array([0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]);
          const superName = new TextEncoder().encode('/tmp/目录/main.db-mj1234569ab');
          const superJournal = new Uint8Array(512 + superName.length + 16);
          superJournal.set(journalMagic);
          superJournal.set(superName, 512);
          const footer = new DataView(superJournal.buffer, 512 + superName.length);
          footer.setUint32(0, superName.length);
          footer.setUint32(4, superName.reduce((sum, byte) => (sum + (byte < 128 ? byte : byte - 256)) >>> 0, 0));
          superJournal.set(journalMagic, superJournal.length - 8);
          const unsupported = await request({ LoadDbs: {
            files: [{ filename: 'super-ref.db', data: first },
              { filename: 'super-ref.db-journal', data: superJournal }],
            replace: false,
          } });
          if (!/super-journal/.test(unsupported.LoadDbs.Err?.LoadDb ?? '')
            || unwrap(await request('ListDbs')).includes('super-ref.db')) {
            throw new Error(`${engine}: Import accepted a journal missing its super-journal`);
          }

          const specialName = `${prefix} sp ace?#%你好.db`;
          unwrap(await request({
            LoadDbs: { files: [{ filename: specialName, data: first }], replace: false },
          }));
          unwrap(await request({
            Open: { filename: specialName, workspace: `${prefix}-special` },
          }));
          if (value(await sql('SELECT value FROM shared_data;'), 'value') !== '1')
            throw new Error(`${engine}: SQLite URI changed the uploaded filename`);

          const joined = { filename: `${prefix}-join.db`, workspace: `${prefix}-first` };
          unwrap(await request({ Open: joined }));
          const joinedResult = await sql(`ATTACH '${aName}' AS a;
            ATTACH '${bName}' AS b;
            SELECT a.shared_data.value + b.shared_data.value AS total
            FROM a.shared_data CROSS JOIN b.shared_data;`);
          if (value(joinedResult, 'total') !== '3')
            throw new Error(`${engine}: ATTACH could not join the uploaded databases`);

          await sql('BEGIN; INSERT INTO a.shared_data VALUES (9);');
          const selected = { ...joined, workspace: `${prefix}-second` };
          unwrap(await request({ Open: selected }));
          if (value(await sql('SELECT count(*) AS count FROM a.shared_data;'), 'count') !== '2')
            throw new Error(`${engine}: Workspace selection discarded the shared transaction`);
          const busyDownload = await request({ DownloadFile: bName });
          if (!busyDownload.DownloadDb.Err)
            throw new Error(`${engine}: Download accepted a mounted file during a transaction`);
          if (!(await request({ DownloadBundle: [bName] })).DownloadBundle.Err)
            throw new Error(`${engine}: Bundle download accepted a mounted database during a transaction`);
          const unrelatedBundle = unwrap(await request({ DownloadBundle: [specialName] }));
          if (unrelatedBundle.files.length !== 1 || unrelatedBundle.files[0].filename !== specialName)
            throw new Error(`${engine}: Bundle download rejected an unrelated database during a transaction`);
          const unrelatedDownload = unwrap(await request({ DownloadFile: specialName }));
          if (unrelatedDownload.filename !== specialName || unrelatedDownload.data.length !== first.length
            || unrelatedDownload.data.some((byte, index) => byte !== first[index])) {
            throw new Error(`${engine}: Downloading an unmounted file changed its bytes`);
          }
          if (value(await sql('SELECT count(*) AS count FROM a.shared_data;'), 'count') !== '2')
            throw new Error(`${engine}: File download discarded the current transaction or ATTACH`);
          const blocked = await request({ Open: { filename: bName, workspace: 'blocked' } });
          if (blocked.Open.Err !== 'ActiveTransaction')
            throw new Error(`${engine}: Changing databases discarded an open transaction`);
          const busyImport = await request({
            LoadDbs: { files: [{ filename: 'not-uploaded.db', data: first }], replace: false },
          });
          if (busyImport.LoadDbs.Err !== 'ActiveTransaction')
            throw new Error(`${engine}: Upload accepted an open transaction`);
          const duringTransaction = unwrap(await request('ListDbs'));
          if (duringTransaction.some((name) => /-(journal|wal|shm)$/.test(name)))
            throw new Error(`${engine}: File selection listed database sidecars`);
          await sql('ROLLBACK;');

          // Replacing an attached file closes the connection before removing it.
          unwrap(await request({
            LoadDbs: { files: [{ filename: aName, data: second }], replace: true },
          }));
          const replaced = await sql(`ATTACH '${aName}' AS a;
            SELECT value FROM a.shared_data;`);
          if (value(replaced, 'value') !== '2')
            throw new Error(`${engine}: Replacing an attached database used the old file`);

          unwrap(await request({
            Run: { sql: 'SELECT 1;', embed: false, clear_on_prepare: true },
          }));
          const remaining = unwrap(await request('ListDbs'));
          if (!remaining.includes(aName) || !remaining.includes(bName))
            throw new Error(`${engine}: Dropping context deleted other databases`);
          unwrap(await request('Close'));
          unwrap(await request({
            Open: { filename: aName, workspace: `${prefix}-third`, read_only: true },
          }));
          if (value(await sql('SELECT value FROM shared_data;'), 'value') !== '2')
            throw new Error(`${engine}: Closing a workspace deleted a shared database`);

          const rawMain = first.slice();
          rawMain[18] = 2;
          rawMain[19] = 2;
          const rawGroup = [
            { filename: 'raw-group.db-shm', data: new Uint8Array([1, 3, 5]) },
            { filename: 'raw-group.db-wal', data: new Uint8Array() },
            { filename: 'raw-group.db-journal', data: new Uint8Array([0, 99]) },
            { filename: 'raw-group.db', data: rawMain },
          ];
          const rawLoaded = unwrap(await request({ LoadDbs: { files: rawGroup, replace: false } }));
          if (JSON.stringify(rawLoaded) !== JSON.stringify(['raw-group.db']))
            throw new Error(`${engine}: Bundle upload returned sidecars as databases`);
          const rawBundle = unwrap(await request({ DownloadBundle: ['raw-group.db'] }));
          for (const original of rawGroup) {
            const copied = rawBundle.files.find((file) => file.filename === original.filename)?.data;
            if (!copied || copied.length !== original.data.length
              || copied.some((byte, index) => byte !== original.data[index])) {
              throw new Error(`${engine}: Bundle upload changed a header, empty WAL, journal, or SHM`);
            }
          }
          if (unwrap(await request('ListDbs')).some((name) => /-(journal|wal|shm)$/.test(name)))
            throw new Error(`${engine}: Bundle sidecars appeared in the database list`);
          unwrap(await request({ Open: { filename: 'raw-group.db', read_only: true } }));
          const rawReadable = await sql('PRAGMA locking_mode; SELECT value FROM shared_data;');
          if (value(rawReadable, 'locking_mode') !== "'exclusive'" || value(rawReadable, 'value') !== '1')
            throw new Error(`${engine}: Raw WAL header did not open read-only in exclusive mode`);
          unwrap(await request('Close'));
        }
        await checkDownloadWal();
        await checkFilesystem();
        unwrap(await request({ Open: { filename: 'cross-engine.db' } }));
        await sql('CREATE TABLE cross_engine(value); INSERT INTO cross_engine VALUES (83);');
        unwrap(await request('Close'));
        checked.push(engine);
      } finally {
        worker.terminate();
      }
    }
    const sqlite = await startWorker('sqlite3');
    try {
      const snapshot = await sqlite.request('ListFiles');
      const files = snapshot.ListFiles.Ok?.files;
      if (!files?.some((file) => file.filename === 'encrypted.db'))
        throw new Error('SQLite: Cipher backend files were not shared');
      const bundle = await sqlite.request({ DownloadBundle: ['encrypted.db'] });
      const encrypted = bundle.DownloadBundle.Ok?.files.find((file) => file.filename === 'encrypted.db');
      if (!encrypted || new TextDecoder().decode(encrypted.data.slice(0, 16)) === 'SQLite format 3\0')
        throw new Error('SQLite: Shared filesystem changed encrypted database bytes');
      const imported = await sqlite.request({ LoadDbs: {
        files: [{ filename: 'cross-engine-cipher.db', data: encrypted.data }], replace: false,
      } });
      if (!imported.LoadDbs.Ok?.includes('cross-engine-cipher.db'))
        throw new Error('SQLite: Files upload rejected another backend\'s encrypted database');
      const copy = await sqlite.request({ DownloadFile: 'cross-engine-cipher.db' });
      const data = copy.DownloadDb.Ok?.data;
      if (!data || data.length !== encrypted.data.length
        || data.some((byte, index) => byte !== encrypted.data[index])) {
        throw new Error('SQLite: Shared encrypted file upload changed its contents');
      }
      if ((await sqlite.request('Close')).Close.Err)
        throw new Error('SQLite: Shared filesystem did not release its worker');
    } finally {
      sqlite.terminate();
    }
    return checked;
  });
  assert.deepEqual(checked, ['sqlite3', 'sqlite3mc', 'sqlcipher']);
}
