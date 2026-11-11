import assert from 'node:assert/strict';

export async function checkWorkspaceMenuPosition(page) {
  const context = await page.context().browser().newContext();
  const origin = new URL(page.url());
  origin.search = '';
  origin.hash = '';
  await context.route('**/*', (route) =>
    route.request().url().startsWith(origin.origin)
      ? route.continue()
      : route.abort(),
  );
  const menuPage = await context.newPage();
  menuPage.setDefaultTimeout(20_000);
  const workspaceButton = menuPage.getByRole('button', { name: /^Workspace: / });
  const menu = menuPage.getByRole('dialog');
  const readOnly = menu.getByRole('checkbox', { name: 'Read only', exact: true });
  const freshDatabase = menu.getByRole('checkbox', {
    name: 'Fresh database per run', exact: true,
  });

  async function stableGeometry() {
    return menuPage.evaluate(async () => {
      await document.fonts.ready;
      const box = (element) => {
        const { x, y, width, height } = element.getBoundingClientRect();
        return { x, y, width, height };
      };
      const snapshot = () => ({
        reference: box([...document.querySelectorAll('#header button')]
          .find((button) => button.textContent.startsWith('Workspace: '))),
        dialog: box(document.querySelector('[role="dialog"]')),
        checkbox: box(document.querySelector('input[aria-label="Read only"]')),
        headerHeight: document.getElementById('header').getBoundingClientRect().height,
      });
      let previous;
      let sameFrames = 0;
      for (let frame = 0; frame < 120; frame++) {
        await new Promise(requestAnimationFrame);
        const current = snapshot();
        if (JSON.stringify(current) === JSON.stringify(previous)) {
          if (++sameFrames === 4) return current;
        } else {
          sameFrames = 0;
        }
        previous = current;
      }
      throw new Error('Workspace menu geometry did not settle');
    });
  }

  function expectStable(before, after, description) {
    for (const part of ['reference', 'dialog', 'checkbox']) {
      for (const property of ['x', 'y', 'width', 'height']) {
        assert.ok(
          Math.abs(after[part][property] - before[part][property]) <= 1,
          `${description}: ${part}.${property} moved from ${before[part][property]} to ${after[part][property]}`,
        );
      }
    }
    assert.ok(
      Math.abs(after.headerHeight - before.headerHeight) <= 1,
      `${description}: toggling Read only changed the header height`,
    );
  }

  function expectInsideViewport(geometry, viewport) {
    const box = geometry.dialog;
    assert.ok(box.x >= -1 && box.y >= -1, 'Workspace menu starts inside the viewport');
    assert.ok(
      box.x + box.width <= viewport.width + 1 && box.y + box.height <= viewport.height + 1,
      'Workspace menu ends inside the viewport',
    );
  }

  try {
    await menuPage.goto(origin.href);
    await menuPage.waitForFunction(() => globalThis.ace?.edit('ace_editor'));
    const viewports = [
      { width: 1440, height: 960 },
      { width: 390, height: 844 },
      { width: 844, height: 390 },
    ];
    for (const [index, viewport] of viewports.entries()) {
      await menuPage.setViewportSize(viewport);
      await workspaceButton.click();
      await readOnly.waitFor();
      await menuPage.waitForFunction(() =>
        !document.querySelector('input[aria-label="Read only"]').disabled,
      );
      if (index === 1) {
        await menu.getByRole('textbox', { name: 'Workspace name', exact: true })
          .fill('A long workspace name that must keep its menu in place');
        await menu.getByRole('button', { name: /^Rename workspace/ }).click();
      }
      await readOnly.scrollIntoViewIfNeeded();
      const before = await stableGeometry();
      expectInsideViewport(before, viewport);

      // Keep this first so the old UI reproduces its movement before checking
      // the newer Fresh database per run control.
      for (const checked of [true, false, true, false]) {
        await readOnly.setChecked(checked);
        const after = await stableGeometry();
        expectStable(before, after, `${viewport.width}x${viewport.height}, Read only=${checked}`);
      }

      await freshDatabase.check();
      assert.equal(await freshDatabase.isChecked(), true);
      await readOnly.check();
      assert.equal(await freshDatabase.isChecked(), true);
      assert.equal(await freshDatabase.isEnabled(), true);
      await freshDatabase.uncheck();
      assert.equal(await readOnly.isChecked(), true);
      await freshDatabase.check();
      assert.equal(await readOnly.isChecked(), true);
      await readOnly.uncheck();
      assert.equal(await freshDatabase.isChecked(), true);
      assert.equal(await freshDatabase.isEnabled(), true);
      await freshDatabase.uncheck();

      const resized = viewports[(index + 1) % viewports.length];
      await menuPage.setViewportSize(resized);
      expectInsideViewport(await stableGeometry(), resized);
      await menuPage.keyboard.press('Escape');
      await menu.waitFor({ state: 'hidden' });
    }
  } finally {
    await context.close();
  }
}
