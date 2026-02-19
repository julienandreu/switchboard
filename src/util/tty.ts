export const isTTY = (process: NodeJS.Process): boolean => {
  return process.stdout.isTTY || false;
};
