import assert from 'node:assert/strict';

export async function checkSharing(page) {
  const browser = page.context().browser();
  const base = new URL(page.url());
  base.search = '?campaign=share-check&campaign=preserved';
  base.hash = '#shared-example';
  const context = await browser.newContext();

  async function open(context, href) {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    await page.goto(href);
    await page.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    return page;
  }

  async function edit(page, sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await page.waitForFunction(
      (sql) => JSON.parse(localStorage.getItem('config'))?.sql === sql,
      sql,
    );
  }

  async function closeMenu(page) {
    await page.keyboard.press('Escape');
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
  }

  function assertConsumed(href) {
    const url = new URL(href);
    for (const name of ['code', 'engine', 'keep_ctx']) {
      assert.equal(url.searchParams.has(name), false, `${name} was consumed`);
    }
    assert.deepEqual(url.searchParams.getAll('campaign'), [
      'share-check',
      'preserved',
    ]);
    assert.equal(url.hash, '#shared-example');
  }

  try {
    const source = await open(context, base.href);
    const engines = [
      [
        'SQLite', 'sqlite3', 'SQLite', false,
        "SELECT '你好 + & # ?' AS shared;", 'shared',
      ],
      [
        'SQLite3 Multiple Ciphers', 'sqlite3mc', 'MultipleCiphers', true,
        "SELECT sqlite3mc_config('cipher') AS cipher;", 'cipher',
      ],
      [
        'SQLCipher', 'sqlcipher', 'SQLCipher', false,
        'PRAGMA cipher_version;', 'cipher_version',
      ],
    ];

    for (const [label, engine, storedEngine, keepContext, sql, column] of engines) {
      await source
        .getByRole('button', { name: 'Advanced options', exact: true })
        .click();
      await source
        .getByRole('combobox', { name: 'SQLite Engine', exact: true })
        .selectOption(label);
      await closeMenu(source);
      const mode = keepContext ? 'Keep Context' : 'Drop Context';
      if (!(await source.getByRole('button', { name: mode, exact: true }).count())) {
        await source.getByRole('button', {
          name: keepContext ? 'Drop Context' : 'Keep Context', exact: true,
        }).click();
        await source.getByRole('dialog').getByRole('button', {
          name: new RegExp(`^${mode}`),
        }).click();
        await closeMenu(source);
      }
      await edit(source, sql);
      await source.getByRole('button', { name: 'Share', exact: true }).click();
      const href = await source
        .getByRole('link', { name: 'Embedded code in link' })
        .getAttribute('href');
      const params = new URL(href).searchParams;
      assert.equal(params.get('code'), sql);
      assert.equal(params.get('engine'), engine);
      assert.equal(params.get('keep_ctx'), keepContext ? '1' : '0');

      const recipientContext = await browser.newContext();
      try {
        await recipientContext.addInitScript((config) => {
          if (!localStorage.getItem('config')) {
            localStorage.setItem('config', JSON.stringify(config));
          }
        }, {
          engine: storedEngine === 'SQLite' ? 'SQLCipher' : 'SQLite',
          keep_ctx: !keepContext,
          sql: 'SELECT 0 AS saved_draft;',
          theme: 'Dark',
        });
        const recipient = await open(recipientContext, href);
        assertConsumed(recipient.url());
        assert.equal(
          await recipient.getByRole('tabpanel').count(),
          0,
          'Opening a link does not run SQL',
        );
        await recipient.waitForFunction(
          ({ engine, keepContext, sql }) => {
            const config = JSON.parse(localStorage.getItem('config'));
            return config.engine === engine &&
              config.keep_ctx === keepContext && config.sql === sql;
          },
          { engine: storedEngine, keepContext, sql },
        );
        assert.equal(
          await recipient.evaluate(() =>
            JSON.parse(localStorage.getItem('config')).theme,
          ),
          'Dark',
        );
        await recipient.getByRole('button', { name: 'Run', exact: true }).click();
        await recipient
          .getByRole('columnheader', { name: column, exact: true })
          .waitFor();
        await recipient.getByText('Finished', { exact: true }).waitFor();

        const revised = 'SELECT 937 AS revised_after_share;';
        await edit(recipient, revised);
        await recipient.reload();
        await recipient.waitForFunction(
          (sql) => globalThis.ace?.edit('ace_editor').getValue() === sql,
          revised,
        );
        assertConsumed(recipient.url());
      } finally {
        await recipientContext.close();
      }
    }

    // Old links still use the recipient's preferences; an empty query is valid.
    const legacyContext = await browser.newContext();
    try {
      await legacyContext.addInitScript(() => {
        if (!localStorage.getItem('config')) {
          localStorage.setItem('config', JSON.stringify({
            engine: 'SQLCipher', keep_ctx: true, sql: 'SELECT 0;',
          }));
        }
      });
      const legacy = new URL(base);
      legacy.searchParams.set('code', 'SELECT 719 AS legacy_share;');
      const recipient = await open(legacyContext, legacy.href);
      await recipient.waitForFunction(() =>
        JSON.parse(localStorage.getItem('config'))?.sql === 'SELECT 719 AS legacy_share;',
      );
      assert.equal(
        await recipient
          .getByRole('button', { name: 'Keep Context', exact: true })
          .count(),
        1,
      );
      assertConsumed(recipient.url());
      legacy.searchParams.set('code', '');
      legacy.searchParams.set('engine', 'unsupported');
      legacy.searchParams.set('keep_ctx', 'unsupported');
      await recipient.goto(legacy.href);
      await recipient.waitForFunction(() => globalThis.ace?.edit('ace_editor').getValue() === '');
      await recipient.waitForFunction(() => {
        const config = JSON.parse(localStorage.getItem('config'));
        return config.sql === '' && config.engine === 'SQLCipher' && config.keep_ctx === true;
      });
      assertConsumed(recipient.url());
    } finally {
      await legacyContext.close();
    }

    // An untouched old tab must not replace work saved by a newer tab.
    const earlier = 'SELECT 929 AS earlier_draft;';
    const latest = 'SELECT 931 AS latest_draft;';
    await edit(source, earlier);
    const oldTab = await open(context, base.href);
    await oldTab.waitForFunction(
      (sql) => ace.edit('ace_editor').getValue() === sql,
      earlier,
    );
    await edit(source, latest);
    await source.close();
    await oldTab.close();
    const restored = await open(context, base.href);
    await restored.waitForFunction(
      (sql) => ace.edit('ace_editor').getValue() === sql,
      latest,
    );

    // Leaving before the debounce timer runs still flushes this tab's edits.
    const lastEdit = 'SELECT 953 AS saved_on_exit;';
    await restored.evaluate(
      (sql) => ace.edit('ace_editor').setValue(sql, -1),
      lastEdit,
    );
    await restored.close();
    const reopened = await open(context, base.href);
    await reopened.waitForFunction(
      (sql) => ace.edit('ace_editor').getValue() === sql,
      lastEdit,
    );
  } finally {
    await context.close();
  }
}
