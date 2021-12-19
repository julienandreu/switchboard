import { Command } from 'commander';
import { addDescription, addVersion, createProgram } from '../program';

describe('program', () => {
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

  describe('addDescription', () => {
    it('should set the description', () => {
      const expectedDescription = 'description';
      const program = new Command();
      const programDescriptionMock = jest.spyOn(program, 'description').mockImplementation();
      addDescription(expectedDescription)(program);
      expect(programDescriptionMock).toBeCalledWith(expectedDescription);
      programDescriptionMock.mockRestore();
    });
    it('should return the program', () => {
      const expectedDescription = 'description';
      const program = new Command();
      const versionedProgram = addDescription(expectedDescription)(program);
      expect(versionedProgram).toBeInstanceOf(Command);
    });
  });
});
