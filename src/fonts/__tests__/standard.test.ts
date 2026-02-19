import { StandardFont } from '../standard';

describe('standard font', () => {
  it('output the same font than before', () => {
    expect.assertions(1);
    expect(StandardFont).toMatchSnapshot();
  });
});
