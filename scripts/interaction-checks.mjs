import assert from 'node:assert/strict';
import { closeFiles, openFiles, waitForFilesIdle } from './files-helpers.mjs';

export async function checkWorkerRecovery(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url()).origin;
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin) ? route.continue() : route.abort(),
  );
  await context.addInitScript(() => {
    const NativeWorker = globalThis.Worker;
    const state = { created: 0, failSnapshot: false, requests: [] };
    globalThis.__workerRecoveryCheck = state;
    globalThis.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args);
        state.created++;
      }

      postMessage(request, ...rest) {
        state.requests.push(typeof request === 'string' ? request : Object.keys(request)[0]);
        if (request === 'ListFiles' && state.failSnapshot) {
          state.failSnapshot = false;
          queueMicrotask(() => this.dispatchEvent(new ErrorEvent('error', {
            message: 'snapshot failure fixture',
          })));
          return;
        }
        super.postMessage(request, ...rest);
      }
    };
  });
  const recoveryPage = await context.newPage();
  recoveryPage.setDefaultTimeout(20_000);

  async function run(sql) {
    await recoveryPage.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await recoveryPage.getByRole('button', { name: 'Run', exact: true }).click();
  }

  try {
    await recoveryPage.goto(origin);
    await recoveryPage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    await recoveryPage.evaluate(() => { globalThis.__workerRecoveryCheck.failSnapshot = true; });
    await run(`CREATE TABLE recovery_probe(value INTEGER);
      INSERT INTO recovery_probe VALUES (2713);
      SELECT value FROM recovery_probe;`);
    await recoveryPage.getByText('Worker communication failed: snapshot failure fixture', {
      exact: true,
    }).waitFor();
    assert.equal(await recoveryPage.getByRole('button', { name: 'Run', exact: true }).isEnabled(), true);
    await recoveryPage.getByRole('tab', { name: 'Execution', exact: true }).click();
    await recoveryPage.getByRole('cell', { name: '2713', exact: true }).waitFor();
    await recoveryPage.getByText('Finished', { exact: true }).waitFor();

    await recoveryPage.getByRole('button', { name: /^Workspace: / }).click();
    await recoveryPage.getByRole('checkbox', { name: 'Fresh database per run', exact: true }).uncheck();
    await recoveryPage.keyboard.press('Escape');
    await recoveryPage.getByRole('dialog').waitFor({ state: 'hidden' });
    for (let attempt = 0; attempt < 2; attempt++) {
      await run('SELECT value FROM recovery_probe;');
      await recoveryPage.getByRole('cell', { name: '2713', exact: true }).waitFor();
      await recoveryPage.getByText('Finished', { exact: true }).waitFor();
    }
    const state = await recoveryPage.evaluate(() => globalThis.__workerRecoveryCheck);
    assert.equal(state.created, 2, 'A snapshot transport error replaces the worker');
    assert.equal(state.requests.filter((request) => request === 'BootstrapDefault').length, 1,
      'Running an existing default database does not repeat initialization');

    // A failed explicit refresh must also discard stale connection/file badges.
    const database = await openFiles(recoveryPage);
    await database.getByText('Connected', { exact: true }).waitFor();
    await recoveryPage.evaluate(() => { globalThis.__workerRecoveryCheck.failSnapshot = true; });
    await database.getByRole('button', { name: 'Refresh', exact: true }).click();
    await waitForFilesIdle(recoveryPage, database);
    assert.equal(await database.getByRole('region').count(), 0);
    assert.equal(await database.getByRole('button', { name: 'Disconnect', exact: true }).isDisabled(), true);
    await closeFiles(recoveryPage);
    await run('SELECT value FROM recovery_probe;');
    await recoveryPage.getByRole('cell', { name: '2713', exact: true }).waitFor();
    await recoveryPage.getByText('Finished', { exact: true }).waitFor();
    assert.equal(await recoveryPage.evaluate(() => globalThis.__workerRecoveryCheck.created), 3);
  } finally {
    await context.close();
  }
}

export async function checkInteractions(page) {
  const viewport = page.viewportSize();
  const originalSql = await page.evaluate(() =>
    ace.edit('ace_editor').getValue(),
  );
  const runButton = page.getByRole('button', { name: 'Run', exact: true });

  // Hold an actual Embed request so editing during execution is deterministic.
  await page.evaluate(() => {
    const original = Worker.prototype.postMessage;
    const pending = [];
    Worker.prototype.postMessage = function (...args) {
      if (args[0]?.Run?.embed) {
        pending.push(() => original.apply(this, args));
        return;
      }
      return original.apply(this, args);
    };
    globalThis.__sqlightInteractionCheck = {
      pending,
      release() {
        if (pending.length !== 1) {
          throw new Error(
            `Expected one held Embed request, got ${pending.length}`,
          );
        }
        pending.shift()();
      },
      restore() {
        Worker.prototype.postMessage = original;
        for (const send of pending.splice(0)) send();
        delete globalThis.__sqlightInteractionCheck;
      },
    };
  });

  async function edit(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
  }

  try {
    await edit('SELECT 619 AS delayed_embed;');
    await page.getByRole('button', { name: 'Tools', exact: true }).click();
    await page.getByRole('button', { name: /^Embed Query Result/ }).click();
    await page.waitForFunction(
      () => globalThis.__sqlightInteractionCheck.pending.length === 1,
    );
    assert.equal(await runButton.isDisabled(), true);

    const newSql = 'SELECT 620 AS preserved_edits;';
    await edit(newSql);
    await page.evaluate(() => globalThis.__sqlightInteractionCheck.release());
    await page.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await page
      .getByText(
        'Query results were not embedded because the SQL changed during execution.',
        { exact: true },
      )
      .waitFor();
    assert.equal(
      await page.evaluate(() => ace.edit('ace_editor').getValue()),
      newSql,
      'Embed preserves edits made while the query was running',
    );
    await page.getByRole('tab', { name: 'Execution', exact: true }).click();
    await page.getByRole('cell', { name: '619', exact: true }).waitFor();
    await page.getByText('Finished', { exact: true }).waitFor();
    assert.equal(await runButton.isEnabled(), true);

    // The preserved document can be run normally after the Embed result arrives.
    await runButton.click();
    await page.getByRole('cell', { name: '620', exact: true }).waitFor();
    await page.getByText('Finished', { exact: true }).waitFor();

    // Embedding preserves source comments, Unicode, and literal result markers.
    const source = [
      '-- A query to share',
      '-- R: a hand-written note',
      '/* Keep this block comment',
      '-- R: inside a block comment',
      '*/',
      "SELECT '你好', 'first line",
      '-- R: inside a multiline string',
      "last line' AS message;",
      '',
      '-- Between the statements',
      '  SELECT 42 AS answer;',
      '',
      '-- Keep this explanation after the final query',
      '/* And this final note */',
      '',
    ].join('\n');
    await edit(source);
    async function embed() {
      await page.getByRole('button', { name: 'Tools', exact: true }).click();
      await page.getByRole('button', { name: /^Embed Query Result/ }).click();
      await page.waitForFunction(
        () => globalThis.__sqlightInteractionCheck.pending.length === 1,
      );
      await page.evaluate(() => globalThis.__sqlightInteractionCheck.release());
      await page.waitForFunction(
        () => !document.querySelector('#header button').disabled,
      );
      return page.evaluate(() => ace.edit('ace_editor').getValue());
    }
    const embedded = await embed();
    assert.ok(embedded.startsWith('-- R:'));
    assert.equal(
      embedded
        .split('\n')
        .filter((line) => /^-- R: [|+]/.test(line) === false)
        .join('\n'),
      source,
      'Embedding retains all original SQL, comments, indentation, and newlines',
    );
    assert.equal(
      await embed(),
      embedded,
      'Embedding again replaces generated result comments without duplicating them',
    );

    await edit('SELECT char(0) AS nul;');
    const nulResult = await embed();
    assert.equal(nulResult.includes('\0'), false);
    assert.ok(nulResult.includes('\\0'));
    assert.ok(nulResult.endsWith('SELECT char(0) AS nul;'));
    await runButton.click();
    await page.getByRole('columnheader', { name: 'nul', exact: true }).waitFor();
    await page.getByText('Finished', { exact: true }).waitFor();

    // Landscape phones must keep the entire popup inside the viewport.
    await page.setViewportSize({ width: 844, height: 390 });
    await page.getByRole('button', { name: 'Config', exact: true }).click();
    const menu = page.getByRole('dialog');
    await menu.waitFor();
    await page.waitForFunction(() => {
      const menu = document.querySelector('[role="dialog"]');
      const box = menu.getBoundingClientRect();
      return box.top >= 0 && box.bottom <= innerHeight;
    });
    const menuBox = await menu.boundingBox();
    assert.ok(menuBox.x >= 0 && menuBox.x + menuBox.width <= 845);
    assert.equal(
      await menu.evaluate(
        (element) => element.scrollHeight > element.clientHeight,
      ),
      true,
      'A tall popup scrolls instead of extending below the landscape viewport',
    );
    const orientation = page.getByRole('combobox', {
      name: 'Orientation',
      exact: true,
    });
    await orientation.scrollIntoViewIfNeeded();
    const lastControl = await orientation.boundingBox();
    assert.ok(lastControl.y >= 0 && lastControl.y + lastControl.height <= 390);
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollHeight <= innerHeight,
      ),
      true,
      'Opening the popup does not add page scrolling',
    );
  } finally {
    await page.evaluate(() => globalThis.__sqlightInteractionCheck.restore());
    await page.keyboard.press('Escape');
    await page.setViewportSize(viewport);
    await edit(originalSql);
  }
}
