module.exports = {
  plugins: [
    require('postcss-simple-vars')(),
    require('postcss-preset-env')(),
    require('postcss-mixins')(),
    require("postcss-modules")({
      getJSON: function(_, json, outputFileName) {
        var path = require("path");
        var fs = require("fs");
        var jsonFileName = path.resolve(outputFileName + ".map");
        fs.writeFileSync(jsonFileName, JSON.stringify(json));
      },
    }),
    require('cssnano')({
      reset: 'default',
    }),
  ]
};
