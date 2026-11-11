import assert from 'node:assert/strict';

export async function checkResultPaging(page) {
  async function run(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
    await page.getByRole('button', { name: 'Run', exact: true }).click();
    await page.getByText('Finished', { exact: true }).waitFor();
  }

  await run(`WITH RECURSIVE numbers(n) AS (
    VALUES (1) UNION ALL SELECT n + 1 FROM numbers WHERE n < 201
  ) SELECT n, printf('row-%03d', n) AS label FROM numbers;`);
  const table = page.getByRole('table', { name: 'Statement #1', exact: true });
  const pages = page.getByRole('navigation', { name: 'Statement #1 pages' });
  const rows = table.locator('tbody tr');
  const previous = pages.getByRole('button', { name: 'Previous' });
  const next = pages.getByRole('button', { name: 'Next' });

  assert.equal(await rows.count(), 100);
  assert.equal(await rows.first().locator('td').first().textContent(), '1');
  assert.equal(await rows.last().locator('td').first().textContent(), '100');
  await pages.getByText('Rows 1–100 of 201', { exact: true }).waitFor();
  assert.equal(await previous.isDisabled(), true);
  await next.click();
  await pages.getByText('Rows 101–200 of 201', { exact: true }).waitFor();
  assert.equal(await rows.first().locator('td').first().textContent(), '101');
  await next.click();
  await pages.getByText('Rows 201–201 of 201', { exact: true }).waitFor();
  assert.equal(await rows.count(), 1);
  assert.equal(await rows.first().locator('td').first().textContent(), '201');
  assert.equal(await next.isDisabled(), true);
  await previous.click();

  // Opening another output tab or collapsing the panel preserves its table and page.
  const originalTable = await table.elementHandle();
  await page.getByRole('button', { name: 'Share', exact: true }).click();
  assert.equal(await originalTable.evaluate((element) => element.isConnected), true);
  await page.getByRole('tab', { name: 'Execution', exact: true }).click();
  await pages.getByText('Rows 101–200 of 201', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Collapse output' }).click();
  assert.equal(await originalTable.evaluate((element) => element.isConnected), true);
  await page.getByRole('tab', { name: 'Execution', exact: true }).click();
  await pages.getByText('Rows 101–200 of 201', { exact: true }).waitFor();
  assert.equal(await table.evaluate((element, original) => element === original, originalTable), true);
  await originalTable.dispose();

  await run(`WITH RECURSIVE numbers(n) AS (
    VALUES (1) UNION ALL SELECT n + 1 FROM numbers WHERE n < 10000
  ) SELECT n FROM numbers;`);
  await pages.getByText('Rows 1–100 of 10000', { exact: true }).waitFor();
  assert.equal(await rows.count(), 100, 'Large results keep only one page in the DOM');
}

export async function checkBatchFailures(page) {
  async function edit(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
  }

  async function inspectFailure(value) {
    await page.getByRole('tab', { name: 'Status', selected: true }).waitFor();
    await page.getByRole('tab', { name: 'Execution', exact: true }).click();
    await page.getByRole('cell', { name: value, exact: true }).waitFor();
    assert.equal(await page.getByText('Finished', { exact: true }).count(), 0);
    assert.equal(await page.getByRole('button', { name: 'Run', exact: true }).isEnabled(), true);
  }

  await edit(`CREATE TABLE batch_progress(value INTEGER PRIMARY KEY);
    INSERT INTO batch_progress VALUES (841);
    SELECT value FROM batch_progress;
    SELECT FROM;
    INSERT INTO batch_progress VALUES (842);`);
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await inspectFailure('841');

  // Earlier statements remain committed; statements after the error are skipped.
  await edit('SELECT count(*) AS rows_kept FROM batch_progress;');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.getByText('Finished', { exact: true }).waitFor();
  await page.getByRole('cell', { name: '1', exact: true }).waitFor();

  // Constraint errors during sqlite3_step preserve the completed SELECT too.
  await edit(`SELECT 843 AS before_constraint;
    INSERT INTO batch_progress VALUES (841);
    SELECT 844 AS skipped_after_constraint;`);
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await inspectFailure('843');
  assert.equal(await page.getByRole('cell', { name: '844', exact: true }).count(), 0);

  // A failed Embed must preserve the whole document, including statements not executed.
  const sql = 'SELECT 845 AS before_error; SELECT FROM; SELECT 846 AS after_error;';
  await edit(sql);
  await page.getByRole('button', { name: 'Tools', exact: true }).click();
  await page.getByRole('button', { name: /^Embed Query Result/ }).click();
  await inspectFailure('845');
  assert.equal(await page.evaluate(() => ace.edit('ace_editor').getValue()), sql);
}
