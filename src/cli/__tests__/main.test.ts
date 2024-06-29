import { main } from '../main';
import * as P from '../program';
import * as F from 'fp-ts/function';
import { log } from 'fp-ts/lib/Console';

describe('main', () => {
  it('should match definition and pipe flow', () => {
    const expectedOptions = {
      name: 'name',
      version: 'version',
      description: 'description',
    };
    const pipeMock = jest.spyOn(F, 'pipe');
    const addVersionMock = jest.spyOn(P, 'addVersion');
    const addDescriptionMock = jest.spyOn(P, 'addDescription');
    main(expectedOptions);
    expect(pipeMock).toBeCalledWith(
      expectedOptions.name,
      P.createProgram,
      expect.any(Function), // addVersion
      expect.any(Function), // addDescription
      expect.any(Function), // parseArguments
      log,
    );
    expect(addVersionMock).toHaveBeenCalledWith(expectedOptions.version);
    expect(addDescriptionMock).toHaveBeenCalledWith(expectedOptions.description);
    pipeMock.mockRestore();
    addVersionMock.mockRestore();
    addDescriptionMock.mockRestore();
  });
});
