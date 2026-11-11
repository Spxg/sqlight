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
    const checked = [];
    for (const engine of ['sqlite3', 'sqlite3mc', 'sqlcipher']) {
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
          throw new Error(`${engine}: ${JSON.stringify(response)}`);
        return result.Ok;
      };
      const run = (sql) => ({
        Run: { sql, embed: false, clear_on_prepare: false },
      });
      const sql = async (value) => unwrap(await request(run(value)));
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
            const error = wrongKey.Run.Err?.SQLite;
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
        checked.push(engine);
      } finally {
        worker.terminate();
      }
    }
    return checked;
  });
  assert.deepEqual(checked, ['sqlite3', 'sqlite3mc', 'sqlcipher']);
}
