import assert from 'node:assert/strict';

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
        const downloaded = unwrap(await request({ DownloadFile: database.filename }));
        if (engine !== 'sqlite3') encryptedHeader(downloaded.data);
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

        await open(true);
        const companion = `${database.filename}-shared.db`;
        unwrap(await request({
          LoadDbs: { files: [{ filename: companion, data: downloaded.data }], replace: false },
        }));
        await sql(key + 'BEGIN; SELECT value FROM protected;');
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

        unwrap(await request({
          Open: { filename: 'empty-readonly.db', read_only: true },
        }));
        const empty = await sql('PRAGMA integrity_check;');
        if (value(empty, 'integrity_check') !== "'ok'")
          throw new Error(`${engine}: Read-only workspace initialized an invalid database`);
        const initialized = unwrap(await request('DownloadDb')).data;
        if (new TextDecoder().decode(initialized.slice(0, 16)) !== 'SQLite format 3\0')
          throw new Error(`${engine}: Empty read-only workspace was not initialized`);
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
          unwrap(await peer.request({ Open: { filename: 'download-restored.db' } }));
          unwrap(await peer.request({ LoadDb: { data: downloaded.data } }));
          const restored = unwrap(await peer.request(run(key + `PRAGMA locking_mode=EXCLUSIVE;
            SELECT value FROM downloaded;`)));
          if (restored.error || value(restored, 'value') !== '456')
            throw new Error(`${engine}: WAL download did not include committed data`);
          unwrap(await peer.request('Close'));
        } finally {
          peer.terminate();
        }
      }

      try {
        unwrap(
          await request({ Open: { filename: 'smoke.db' } }),
        );
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
            INSERT INTO secret VALUES (456);`,
          );
          const encrypted = unwrap(await request('DownloadDb'));
          encryptedHeader(encrypted.data);
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
          ]) {
            const failed = await request({ LoadDbs: { files, replace: false } });
            if (!failed.LoadDbs.Err)
              throw new Error(`${engine}: Batch upload accepted invalid or conflicting files`);
          }
          const names = unwrap(await request('ListDbs'));
          if (!names.includes(aName) || !names.includes(bName)
            || names.includes('duplicate.db') || names.includes('not-written.db')) {
            throw new Error(`${engine}: Batch validation left a partial import`);
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
          unwrap(await request({ CloseWorkspace: 'inactive-workspace' }));
          await sql('SELECT 1;');
          unwrap(await request({ CloseWorkspace: selected.workspace }));
          unwrap(await request({
            Open: { filename: aName, workspace: `${prefix}-third`, read_only: true },
          }));
          if (value(await sql('SELECT value FROM shared_data;'), 'value') !== '2')
            throw new Error(`${engine}: Closing a workspace deleted a shared database`);
          unwrap(await request('Close'));
        }
        await checkDownloadWal();
        checked.push(engine);
      } finally {
        worker.terminate();
      }
    }
    return checked;
  });
  assert.deepEqual(checked, ['sqlite3', 'sqlite3mc', 'sqlcipher']);
}
