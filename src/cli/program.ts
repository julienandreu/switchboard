import { Command, createCommand } from 'commander';

export const createProgram = createCommand;

export const addVersion =
  (version: string) =>
  (program: Command): Command =>
    program.version(version);
