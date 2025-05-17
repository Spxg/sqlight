module.exports = {
  plugins: [
    require('postcss-simple-vars')(),
    require('postcss-preset-env')(),
    require('postcss-mixins')(),
    require("postcss-modules")({
      getJSON: function(_, json, outputFileName) {
        var path = require("path");
        var fs = require("fs");
        var jsonPath = path.resolve(outputFileName + ".map");
        var dir = path.dirname(jsonPath);
        if (!fs.existsSync(dir)) {
          fs.mkdirSync(dir, { recursive: true });
        }
        fs.writeFileSync(jsonPath, JSON.stringify(json));
      },
    }),
    require('cssnano')({
      reset: 'default',
    }),
  ]
};
