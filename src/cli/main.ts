import { pipe } from 'fp-ts/function';
import { log } from 'fp-ts/lib/Console';
import { addDescription, addVersion, createProgram } from './program';
import { MainOptions } from '../interfaces';

export const main = ({ name, version, description }: MainOptions) =>
  pipe(name, createProgram, addVersion(version), addDescription(description), log)();
