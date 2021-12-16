import { main } from '../main';

describe('main', () => {
  it('should console log "Hello Switchboard"', () => {
    const consoleLogMock = jest.spyOn(console, 'log').mockImplementation();
    main();
    expect(console.log).toBeCalledWith('Hello Switchboard');
    consoleLogMock.mockRestore();
  });
});
