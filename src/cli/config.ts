import fp from 'lodash/fp';
import {resolve} from 'path';
import {readFileSync} from 'fs';
import {load} from 'js-yaml';

export const getRoutes = fp.compose(load, readFileSync, resolve);
