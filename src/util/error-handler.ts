import {progress} from "./echo";
import {getExecutionTime} from "./execution-time";

export const errorReceived = (process: NodeJS.Process, startTime: [number, number]) => (): void => {
  progress.succeed(`Done after ${getExecutionTime(startTime)}s`);
  process.exit(0);
};

export const handleErrors = (process: NodeJS.Process, startTime: [number, number]): void => {
  const catchableErrors: string[] = ['unhandledRejection', 'uncaughtException'];
  catchableErrors.map(
    (event): NodeJS.EventEmitter => {
      return process.on(event, errorReceived(process, startTime));
    },
  );
};
