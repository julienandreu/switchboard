import packageJSON from '../../../package.json';
import { name, version, description } from '../package-json';

describe('package-json', () => {
  it('use the "name" defined in the package.json', () => {
    const expectedName = packageJSON.name;
    expect.assertions(1);
    expect(name).toStrictEqual(expectedName);
  });
  it('use the "version" defined in the package.json', () => {
    const expectedVersion = packageJSON.version;
    expect.assertions(1);
    expect(version).toStrictEqual(expectedVersion);
  });
  it('use the "description" defined in the package.json', () => {
    const expectedDescription = packageJSON.description;
    expect.assertions(1);
    expect(description).toStrictEqual(expectedDescription);
  });
});
