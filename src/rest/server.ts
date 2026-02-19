import http from 'http';
import fp from 'lodash/fp';
import {createRESTApplication} from './application';

export const createRESTServer = fp.compose(http.createServer, createRESTApplication);
