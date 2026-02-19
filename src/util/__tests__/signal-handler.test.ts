import { handleSignals, signalReceived } from '../signal-handler';
import logger = require('fancy-log');

describe('signalReceived', () => {
  it('stop the process', () => {
    expect.assertions(1);
    const processExitMock = jest.spyOn(process, 'exit').mockImplementation();
    const customEvent = 'TEST_EVENT';
    signalReceived(process)(customEvent);
    expect(processExitMock).toHaveBeenCalledTimes(1);
    processExitMock.mockClear();
  });
  it('log an error with the event name', () => {
    expect.assertions(2);
    const loggerErrorMock = jest.spyOn(logger, 'error').mockImplementation();
    const customEvent = 'TEST_EVENT';
    signalReceived(process)(customEvent);
    expect(loggerErrorMock).toHaveBeenCalledTimes(1);
    expect(loggerErrorMock).toHaveBeenCalledWith(`Process terminated on ${customEvent}`);
    loggerErrorMock.mockClear();
  });
});

describe('handleSignals', () => {
  it('attach event listener on SIGTERM then SIGINT', () => {
    expect.assertions(3);
    const runningProcess = process;
    const processOnMock = jest.spyOn(runningProcess, 'on').mockImplementation();
    handleSignals(runningProcess);
    expect(processOnMock).toHaveBeenCalledTimes(2);
    expect(processOnMock.mock.calls[0][0]).toStrictEqual('SIGTERM');
    expect(processOnMock.mock.calls[1][0]).toStrictEqual('SIGINT');
    processOnMock.mockClear();
  });
});
