const fs = require('node:fs/promises');
const path = require('node:path');

module.exports = {
  plugins: [
    require('postcss-simple-vars')(),
    require('postcss-mixins')(),
    require('postcss-preset-env')(),
    require('postcss-modules')({
      async getJSON(_, classes, outputFileName) {
        const mapPath = `${outputFileName}.map`;
        await fs.mkdir(path.dirname(mapPath), { recursive: true });
        const contents = JSON.stringify(classes);
        const previous = await fs.readFile(mapPath, 'utf8').catch(error => {
          if (error.code !== 'ENOENT') {
            throw error;
          }
        });
        if (contents !== previous) {
          await fs.writeFile(mapPath, contents);
        }
      },
    }),
    require('cssnano')({
      preset: 'default',
    }),
  ],
};
