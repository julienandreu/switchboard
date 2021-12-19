import { main } from './cli';

// eslint-disable-next-line @typescript-eslint/no-var-requires
const { name, version, description } = require('../package.json');

main({ name, version, description });
