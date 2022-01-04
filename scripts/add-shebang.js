const fs = require('fs');
const path = require('path');

const filename = path.resolve(__dirname, '..', 'dist', 'index.js');

const lines = fs.readFileSync(filename, 'utf8').split('\n').filter(Boolean);

fs.writeFileSync(filename, ['#!/usr/bin/env node', ...lines].join('\n'));

fs.chmodSync(filename, 0o755);
