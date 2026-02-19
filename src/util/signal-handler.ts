import {progress} from "./echo";
import {getExecutionTime} from "./execution-time";

export const signalReceived = (process: NodeJS.Process, startTime: [number, number], event: NodeJS.Signals) => (): void => {
  progress.succeed(`Done after ${getExecutionTime(startTime)}s`);
  process.kill(process.pid, event);
  process.exit(0);
};

export const handleSignals = (process: NodeJS.Process, startTime: [number, number]): void => {
  const catchableSignals: NodeJS.Signals[] = ['SIGTERM', 'SIGINT', 'SIGUSR2'];
  catchableSignals.map(
    (event): NodeJS.EventEmitter => {
      return process.once(event, signalReceived(process, startTime, event));
    },
  );
};
