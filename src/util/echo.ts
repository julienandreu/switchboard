import ora from "ora";
import stream from "stream";

export const progress = ora();

export const echo = (content: unknown) => {
  progress.stop();
  console.log(content);
  progress.start();
};

export const echo$ = new stream.Writable({
  write: (chunk, encoding, next) => {
    echo(chunk.toString().replace(/\n$/, ''));
    next();
  }
});
