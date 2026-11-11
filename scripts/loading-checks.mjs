import assert from 'node:assert/strict';

export async function checkExecutionProgress(page) {
  const runButton = page.getByRole('button', { name: 'Run', exact: true });
  const progress = page.getByRole('status', {
    name: 'Running SQL',
    exact: true,
  });
  const execution = page.getByRole('tab', { name: 'Execution', exact: true });

  // Hold actual worker requests until the loading state has been inspected.
  await page.evaluate(() => {
    const original = Worker.prototype.postMessage;
    const pending = [];
    const requests = [];
    Worker.prototype.postMessage = function (...args) {
      const request = args[0];
      if (
        request &&
        typeof request === 'object' &&
        Object.hasOwn(request, 'Run')
      ) {
        requests.push(request.Run);
        pending.push(() => original.apply(this, args));
        return;
      }
      return original.apply(this, args);
    };
    globalThis.__sqlightLoadingCheck = {
      requests,
      release() {
        if (pending.length !== 1) {
          throw new Error(
            `Expected one held SQL request, got ${pending.length}`,
          );
        }
        pending.shift()();
      },
      restore() {
        Worker.prototype.postMessage = original;
        for (const send of pending.splice(0)) send();
        delete globalThis.__sqlightLoadingCheck;
      },
    };
  });

  async function edit(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
  }

  async function waitForRequest(count) {
    await page.waitForFunction(
      (count) => globalThis.__sqlightLoadingCheck.requests.length === count,
      count,
    );
  }

  async function release() {
    await page.evaluate(() => globalThis.__sqlightLoadingCheck.release());
  }

  async function assertRunning() {
    await progress.waitFor();
    assert.equal(await runButton.isDisabled(), true);
    const dots = progress.locator('span').filter({ hasText: /^⬤$/ });
    assert.equal(await dots.count(), 3);
    assert.deepEqual(
      await dots.evaluateAll((dots) =>
        dots.map((dot) => {
          const style = getComputedStyle(dot);
          return (
            style.animationName !== 'none' &&
            parseFloat(style.animationDuration) > 0 &&
            dot
              .getAnimations()
              .some((animation) => animation.playState === 'running')
          );
        }),
      ),
      [true, true, true],
      'All three progress dots have running CSS animations',
    );
  }

  try {
    await edit('SELECT 71 AS loading_success;');
    await runButton.click();
    await waitForRequest(1);
    await assertRunning();

    // Keyboard shortcuts must respect the same in-flight execution as Run.
    await page.locator('#ace_editor').click();
    await page.keyboard.press('Control+Enter');
    await page.keyboard.press('Control+Enter');
    assert.equal(
      await page.evaluate(
        () => globalThis.__sqlightLoadingCheck.requests.length,
      ),
      1,
    );
    await release();
    await page.getByRole('cell', { name: '71', exact: true }).waitFor();
    await page
      .getByRole('tabpanel')
      .getByText('Finished', { exact: true })
      .waitFor();
    await progress.waitFor({ state: 'hidden' });
    assert.equal(await runButton.isEnabled(), true);
    assert.equal(
      await page.evaluate(
        () => globalThis.__sqlightLoadingCheck.requests.length,
      ),
      1,
      'Repeated shortcuts did not queue another worker request',
    );

    await edit('SELECT FROM;');
    await runButton.click();
    await waitForRequest(2);
    await assertRunning();
    await release();
    await page.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    assert.equal(await runButton.isEnabled(), true);
    await execution.click();
    await progress.waitFor({ state: 'hidden' });

    // Embed also executes SQL, including when another output tab is selected.
    await page.getByRole('tab', { name: 'Status', exact: true }).click();
    await edit('SELECT 72 AS loading_embed;');
    await page.getByRole('button', { name: 'Tools', exact: true }).click();
    await page.getByRole('button', { name: /^Embed Query Result/ }).click();
    await waitForRequest(3);
    assert.equal(
      await page.evaluate(
        () => globalThis.__sqlightLoadingCheck.requests[2].embed,
      ),
      true,
    );
    await page.getByText('No errors.', { exact: true }).waitFor();
    await execution.click();
    await assertRunning();
    await release();
    await page.waitForFunction(() => {
      const sql = ace.edit('ace_editor').getValue();
      return sql.startsWith('-- R:') && sql.includes('72');
    });
    await progress.waitFor({ state: 'hidden' });
    assert.equal(await runButton.isEnabled(), true);
  } finally {
    await page.evaluate(() => globalThis.__sqlightLoadingCheck.restore());
  }
}
