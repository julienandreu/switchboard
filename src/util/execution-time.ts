import convertHrtime from 'convert-hrtime';
import fp from 'lodash/fp';

export const convertHrtimeExtractSeconds = fp.curry((convertedHrtime: convertHrtime.HRTime): string =>
  convertedHrtime.seconds.toFixed(2),
);

export const getExecutionTime = fp.compose(parseFloat, convertHrtimeExtractSeconds, convertHrtime, process.hrtime);
