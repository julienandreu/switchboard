import figlet from 'figlet';
import { StandardFont } from '../fonts/standard';

figlet.parseFont('Standard', StandardFont);

export const drawText = (text: string): string => {
  return figlet.textSync(text, 'Standard');
};
