import os from "os";

const getNetworkAddresses = () => {
  const networkInterfaces = os.networkInterfaces();

  console.dir(
    {
      src: `${__dirname}${__filename}`,
      networkInterfaces,
    },
    {depth: null, colors: true},
  );
};
