import { copyFile, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import postcss from 'postcss';
import postcssConfig from '../postcss.config.js';

const root = fileURLToPath(new URL('..', import.meta.url));
process.chdir(root);

const sourceDir = 'assets/module.css';
const outputDir = 'assets/module.postcss';
const entries = await readdir(sourceDir, { recursive: true });

for (const entry of entries.sort()) {
  if (!entry.endsWith('.css')) {
    continue;
  }

  const from = path.join(sourceDir, entry);
  const to = path.join(outputDir, entry);
  const source = await readFile(from, 'utf8');
  const result = await postcss(postcssConfig.plugins).process(source, {
    from,
    to,
    map: false,
  });

  for (const warning of result.warnings()) {
    console.warn(warning.toString());
  }

  await mkdir(path.dirname(to), { recursive: true });
  await writeFile(to, result.css);
}

// Font hashes change when a dependency updates; only copy current assets.
await rm('assets/generated', { recursive: true, force: true });
await build({
  entryPoints: ['assets/vendor.js'],
  bundle: true,
  minify: true,
  format: 'iife',
  target: 'es2022',
  outfile: 'assets/generated/vendor.js',
  loader: { '.woff2': 'file' },
  assetNames: '[name]-[hash]',
  legalComments: 'linked',
  logLevel: 'info',
});

for (const font of ['open-sans', 'source-code-pro']) {
  await copyFile(
    `node_modules/@fontsource-variable/${font}/LICENSE`,
    `assets/generated/${font}-LICENSE.txt`,
  );
}
