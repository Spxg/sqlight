import assert from 'node:assert/strict';

export async function checkEngineSelection(page) {
  const engineSelect = page.getByRole('combobox', {
    name: 'SQLite Engine',
    exact: true,
  });

  async function selectEngine(value, savedValue) {
    await page
      .getByRole('button', { name: 'Advanced options', exact: true })
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
    .getByRole('button', { name: 'Advanced options', exact: true })
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
    .getByRole('button', { name: 'Advanced options', exact: true })
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

      try {
        unwrap(
          await request({ Open: { filename: 'smoke.db', persist: false } }),
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
        await sql('ROLLBACK;');
        const exported = unwrap(await request('DownloadDb'));
        unwrap(await request({ LoadDb: { data: exported.data } }));
        const imported = await sql('SELECT * FROM persistent;');
        if (value(imported, 'value') !== '7')
          throw new Error(`${engine}: Database transfer changed rows`);

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
              Open: { filename: 'encrypted.db', persist: false },
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
            Open: { filename: `smoke-${engine}.db`, persist: true },
          }),
        );
        const key = engine === 'sqlcipher' ? "PRAGMA key='opfs-key'; " : '';
        await sql(
          key +
            `CREATE TABLE IF NOT EXISTS persisted(value);
            DELETE FROM persisted;
            INSERT INTO persisted VALUES (123);`,
        );
        unwrap(
          await request({ Open: { filename: 'memory.db', persist: false } }),
        );
        unwrap(
          await request({
            Open: { filename: `smoke-${engine}.db`, persist: true },
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

        unwrap(await request({
          Open: { filename: 'handoff-memory.db', persist: false },
        }));
        await sql(`CREATE TABLE cached(value);
          INSERT INTO cached VALUES (321);`);
        const peer = await startWorker(engine);
        try {
          const persistent = {
            Open: { filename: `smoke-${engine}.db`, persist: true },
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
          unwrap(await request({
            Open: { filename: 'handoff-memory.db', persist: false },
          }));
          if (value(await sql('SELECT value FROM cached;'), 'value') !== '321')
            throw new Error(`${engine}: Failed OPFS resume lost the Memory database`);

          unwrap(await peer.request({
            Open: { filename: 'peer-memory.db', persist: false },
          }));
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
          Open: { filename: `smoke-${engine}.db-wal`, persist: true },
        }));
        const retained = await sql(key + 'SELECT value FROM marker;');
        if (value(retained, 'value') !== '999')
          throw new Error(`${engine}: Failed export changed WAL contents`);
        unwrap(await request({
          Open: { filename: `smoke-${engine}.db`, persist: true },
        }));
        unwrap(await request({
          Run: { sql: key + 'SELECT 1;', embed: false, clear_on_prepare: true },
        }));
        unwrap(await request({ LoadDb: { data: baseline.data } }));
        checked.push(engine);
      } finally {
        worker.terminate();
      }
    }
    return checked;
  });
  assert.deepEqual(checked, ['sqlite3', 'sqlite3mc', 'sqlcipher']);
}
