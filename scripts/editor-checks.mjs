import assert from 'node:assert/strict';

export async function checkEditor(page) {
  const editor = page.locator('#ace_editor');
  const runButton = page.getByRole('button', { name: 'Run', exact: true });
  const viewport = page.viewportSize();
  const originalCode = await page.evaluate(() =>
    ace.edit('ace_editor').getValue(),
  );

  async function closeMenu() {
    await page.keyboard.press('Escape');
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
  }

  async function configure(preferences) {
    await page.getByRole('button', { name: 'Config', exact: true }).click();
    for (const [name, value] of Object.entries(preferences)) {
      await page
        .getByRole('combobox', { name, exact: true })
        .selectOption(value);
    }
    await closeMenu();
  }

  async function selectSql(enabled) {
    await page
      .getByRole('button', { name: 'Advanced options', exact: true })
      .click();
    await page
      .getByRole('group', { name: 'Run Selected SQL', exact: true })
      .getByText(enabled ? 'On' : 'Off', { exact: true })
      .click();
    await closeMenu();
  }

  async function edit(sql) {
    await page.evaluate((sql) => ace.edit('ace_editor').setValue(sql, -1), sql);
  }

  async function finishedWith(value) {
    await page
      .getByRole('tabpanel')
      .getByRole('cell', { name: value, exact: true })
      .waitFor();
    await page
      .getByRole('tabpanel')
      .getByText('Finished', { exact: true })
      .waitFor();
    assert.equal(await runButton.isEnabled(), true);
  }

  async function assertMenuFits() {
    const menu = page.getByRole('dialog');
    await menu.waitFor();
    await menu.evaluate(
      (element) =>
        new Promise((resolve) => {
          requestAnimationFrame(() => requestAnimationFrame(resolve));
        }),
    );
    const box = await menu.boundingBox();
    const size = page.viewportSize();
    assert.ok(
      box.x >= 0 && box.x + box.width <= size.width + 1,
      'Mobile menus fit horizontally',
    );
    assert.ok(
      box.y >= 0 && box.y + box.height <= size.height + 1,
      'Mobile menus fit vertically',
    );
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
      true,
      'Opening a mobile menu does not introduce horizontal page scrolling',
    );
  }

  await page.getByRole('button', { name: 'Config', exact: true }).click();
  const originalKeybinding = await page
    .getByRole('combobox', { name: 'Keybinding', exact: true })
    .inputValue();
  const originalOrientation = await page
    .getByRole('combobox', { name: 'Orientation', exact: true })
    .inputValue();
  await closeMenu();
  await page
    .getByRole('button', { name: 'Advanced options', exact: true })
    .click();
  const originalRunSelected = await page
    .getByRole('group', { name: 'Run Selected SQL', exact: true })
    .getByRole('radio', { name: 'On', exact: true })
    .isChecked();
  await closeMenu();

  try {
    // Vim's global :w command must dispatch to this editor's Run command.
    await selectSql(false);
    await configure({ Keybinding: 'vim' });
    await page.waitForFunction(
      () =>
        ace.edit('ace_editor').getKeyboardHandler().$id === 'ace/keyboard/vim',
    );
    const vimSql = 'SELECT 73 AS vim_write;';
    await edit(vimSql);
    await editor.click();
    // Ace's click handler queues a second focus call; let it finish before : opens a dialog.
    await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 0)));
    await page.keyboard.press('Escape');
    await page.keyboard.press('Shift+Semicolon');
    const command = page.locator('.ace_dialog input');
    await command.waitFor();
    await command.press('w');
    await command.press('Enter');
    await finishedWith('73');
    assert.equal(
      await page.evaluate(() => ace.edit('ace_editor').getValue()),
      vimSql,
      'Vim :w executes SQL without inserting the command into the document',
    );

    await configure({ Keybinding: 'ace' });
    await page.waitForFunction(() => {
      const editor = ace.edit('ace_editor');
      return editor.getKeyboardHandler() === editor.commands;
    });

    // Keep the selection unchanged while testing both settings.
    await selectSql(true);
    await edit(
      "SELECT 'whole document' AS unselected;\nSELECT 'selected statement' AS selected;",
    );
    await page.evaluate(() => {
      const editor = ace.edit('ace_editor');
      editor.selection.setSelectionRange({
        start: { row: 1, column: 0 },
        end: { row: 1, column: editor.session.getLine(1).length },
      });
    });
    await runButton.click();
    await finishedWith("'selected statement'");
    assert.deepEqual(
      await page.locator('tbody td').allTextContents(),
      ["'selected statement'"],
      'Run Selected SQL executes only the highlighted statement',
    );

    await selectSql(false);
    assert.equal(
      await page.evaluate(() => ace.edit('ace_editor').getSelectedText()),
      "SELECT 'selected statement' AS selected;",
    );
    await runButton.click();
    await finishedWith("'whole document'");
    assert.deepEqual(
      await page.locator('tbody td').allTextContents(),
      ["'whole document'", "'selected statement'"],
      'Turning Run Selected SQL off executes the entire document',
    );

    // Switching axes recreates the splitter; dragging must resize Ace on both axes.
    await page.setViewportSize({ width: 1440, height: 960 });
    for (const [orientation, axis, distance] of [
      ['Vertical', 'width', 120],
      ['Horizontal', 'height', 80],
    ]) {
      await configure({ Orientation: orientation });
      const before = await editor.boundingBox();
      const gutter = await page
        .getByText('⣿', { exact: true })
        .locator('..')
        .boundingBox();
      const x = gutter.x + gutter.width / 2;
      const y = gutter.y + gutter.height / 2;
      await page.mouse.move(x, y);
      await page.mouse.down();
      await page.mouse.move(
        x + (axis === 'width' ? distance : 0),
        y + (axis === 'height' ? distance : 0),
        { steps: 8 },
      );
      await page.mouse.up();
      await page.waitForFunction(
        ({ axis, previous }) => {
          return (
            document.getElementById('ace_editor').getBoundingClientRect()[
              axis
            ] >
            previous + 40
          );
        },
        { axis, previous: before[axis] },
      );
      await page.waitForFunction(() => {
        const editor = ace.edit('ace_editor');
        return (
          Math.abs(editor.renderer.$size.width - editor.container.clientWidth) <
            2 &&
          Math.abs(
            editor.renderer.$size.height - editor.container.clientHeight,
          ) < 2
        );
      });
    }

    // Narrow screens stack the panes and keep every popup inside the viewport.
    await configure({ Orientation: 'Automatic' });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForFunction(() => {
      const editor = document
        .getElementById('ace_editor')
        .getBoundingClientRect();
      const output = document
        .querySelector('[role=tabpanel]')
        .getBoundingClientRect();
      return output.top >= editor.bottom;
    });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
      true,
      'The 390px layout has no horizontal page overflow',
    );
    for (const name of ['Config', 'Advanced options']) {
      const button = page.getByRole('button', { name, exact: true });
      const box = await button.boundingBox();
      assert.ok(
        box.x >= 0 && box.x + box.width <= 391,
        `${name} remains inside the mobile header`,
      );
      await button.click();
      await assertMenuFits();
      await closeMenu();
    }
  } finally {
    await closeMenu();
    await page.setViewportSize(viewport);
    await configure({
      Keybinding: originalKeybinding,
      Orientation: originalOrientation,
    });
    await selectSql(originalRunSelected);
    await edit(originalCode);
  }
}
