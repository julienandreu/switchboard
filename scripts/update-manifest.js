const fs = require('fs');
const path = require('path');

const packageJson = path.resolve(__dirname, '..', 'package.json');

const { name, version, description } = JSON.parse(fs.readFileSync(packageJson, 'utf8'));

const manifestJson = path.resolve(__dirname, '..', 'src', 'assets', 'manifest.json');

fs.writeFileSync(manifestJson, JSON.stringify({ name, version, description }, null, 2));
