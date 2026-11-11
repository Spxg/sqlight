import { readFile } from 'node:fs/promises';

export async function waitForFilesIdle(page, drawer) {
  const refresh = drawer.getByRole('button', { name: 'Refresh', exact: true });
  await refresh.waitFor();
  await page.waitForFunction((button) => !button.disabled, await refresh.elementHandle());
}

export async function openFiles(page) {
  const drawer = page.getByRole('dialog', { name: 'Files', exact: true });
  if (!await drawer.isVisible()) {
    await page.getByRole('button', { name: 'Files', exact: true }).click();
  }
  await drawer.waitFor();
  await drawer.getByRole('button', { name: 'Refresh', exact: true }).click();
  await waitForFilesIdle(page, drawer);
  return drawer;
}

export async function closeFiles(page) {
  const drawer = page.getByRole('dialog', { name: 'Files', exact: true });
  if (await drawer.isVisible()) {
    await drawer.getByRole('button', { name: 'Close Files', exact: true }).click();
    await drawer.waitFor({ state: 'hidden' });
  }
}

export function databaseGroup(drawer, filename) {
  return drawer.getByRole('region', { name: `Database ${filename}`, exact: true });
}

export async function chooseFiles(page, files) {
  const drawer = await openFiles(page);
  const opened = page.waitForEvent('filechooser');
  await drawer.getByRole('button', { name: 'Upload', exact: true }).click();
  await (await opened).setFiles(files.map(([name, buffer]) => ({
    name,
    mimeType: name.endsWith('.zip') ? 'application/zip' : 'application/vnd.sqlite3',
    buffer: Buffer.from(buffer),
  })));
  await closeFiles(page);
}

export async function downloadDatabase(page, filename, format = 'database') {
  const drawer = await openFiles(page);
  await drawer.getByRole('checkbox', { name: 'Select all files', exact: true }).uncheck();
  await drawer.getByRole('combobox', { name: 'Download format', exact: true })
    .selectOption(format);
  const downloaded = page.waitForEvent('download');
  await databaseGroup(drawer, filename)
    .getByRole('button', { name: 'Download', exact: true }).click();
  const file = await downloaded;
  const result = { name: file.suggestedFilename(), data: await readFile(await file.path()) };
  await closeFiles(page);
  return result;
}

export async function downloadDatabases(page, filenames) {
  const drawer = await openFiles(page);
  await drawer.getByRole('combobox', { name: 'Download format', exact: true })
    .selectOption('zip');
  const all = drawer.getByRole('checkbox', { name: 'Select all files', exact: true });
  await all.uncheck();
  if (filenames) {
    for (const filename of filenames) {
      await drawer.getByRole('checkbox', { name: `Select ${filename}`, exact: true }).check();
    }
  } else {
    await all.check();
  }
  const downloaded = page.waitForEvent('download');
  await drawer.getByRole('button', { name: 'Download selected', exact: true }).click();
  const file = await downloaded;
  const result = { name: file.suggestedFilename(), data: await readFile(await file.path()) };
  await closeFiles(page);
  return result;
}
