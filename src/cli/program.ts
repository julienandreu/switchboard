import { Command, createCommand } from 'commander';

export const createProgram = createCommand;

export const addVersion =
  (version: string) =>
  (program: Command): Command =>
    program.version(version, '-v, --version');

export const addDescription =
  (description: string) =>
  (program: Command): Command =>
    program.description(description);

export const parseArguments = (program: Command): Command => program.parse(process.argv);
