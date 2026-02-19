import { convertHrtimeExtractSeconds, getExecutionTime } from '../execution-time';

describe('convertHrtimeExtractSeconds', () => {
  it('return the diff in seconds from a start hrtime', () => {
    expect.assertions(1);
    const convertedHrtime = {
      seconds: 1.234,
      milliseconds: 5.678,
      nanoseconds: 9.012,
    };
    const seconds: string = convertHrtimeExtractSeconds(convertedHrtime);
    const expectedSeconds = '1.23';
    expect(seconds).toStrictEqual(expectedSeconds);
  });
});

describe('getExecutionTime', () => {
  it('chains functions process.hrtime > convertHrtime > convertHrtimeExtractSeconds > parseFloat', async (): Promise<void> => {
    expect.assertions(1);
    const startTime = jest.fn().mockImplementation();
    const expectedExecutionTime = 210;
    await new Promise<void>((resolve) => {
      setTimeout(() => {
        const executionTime = getExecutionTime(startTime);
        expect(executionTime).toBeCloseTo(expectedExecutionTime / 1000, 1);
        resolve();
      }, expectedExecutionTime);
    });
  });
});
