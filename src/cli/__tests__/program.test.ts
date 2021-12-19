import { Command } from 'commander';
import { addVersion, createProgram } from '../program';

describe('createProgram', () => {
  it('should create a new empty program', () => {
    const program = createProgram();
    expect(program).toBeInstanceOf(Command);
  });
});

describe('addVersion', () => {
  it('should set the version as option', () => {
    const expectedVersion = '1.0.0-test';
    const program = new Command();
    const programVersionMock = jest.spyOn(program, 'version').mockImplementation();
    addVersion(expectedVersion)(program);
    expect(programVersionMock).toBeCalledWith(expectedVersion);
    programVersionMock.mockRestore();
  });
  it('should return the program', () => {
    const expectedVersion = '1.0.0-test';
    const program = new Command();
    const versionedProgram = addVersion(expectedVersion)(program);
    expect(versionedProgram).toBeInstanceOf(Command);
  });
});
