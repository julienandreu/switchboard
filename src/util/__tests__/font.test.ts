import { drawText } from '../font';

describe('drawText', () => {
  it('use figlet with the "Standard" font', () => {
    expect.assertions(1);
    expect(drawText('Font.DrawText.Test')).toMatchSnapshot();
  });
});
