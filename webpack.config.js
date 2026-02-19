const { name } = require('./package.json');

module.exports = {
  mode: 'production',
  output: {
    filename: name,
  },
  target: 'node',
  stats: {
    warningsFilter: [/critical dependency:/i],
  }
};
