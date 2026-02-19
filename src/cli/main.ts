import {flatten} from "array-flatten";
import chalk from 'chalk';
import CliTable3 from "cli-table3";
import {writeSync as copyToClipboard} from 'clipboardy';
import {program} from 'commander';
import debug from "debug";
import isDocker from "is-docker";
import loudRejection from 'loud-rejection';
import {AddressInfo} from "net";
import os from "os";
import {createRESTServer} from "../rest/server";
import {echo, progress} from "../util/echo";
import {handleErrors} from "../util/error-handler";
import {drawText} from '../util/font';
import {description, name, version} from '../util/package-json';
import {handleSignals} from '../util/signal-handler';
import {getRoutes} from "./config";
import {ConfiguredRoutes} from "./config.interface";
import {MainOptions} from './main.interface';

export const main = (): void => {
  // Get HRTime
  const startTime: [number, number] = process.hrtime();
  // Define debug namespace
  debug(`${name}:${process.pid}`);
  // Loud rejection
  loudRejection();
  // Handle signals
  handleSignals(process, startTime);
  // Handle errors
  handleErrors(process, startTime);
  // Define program params/
  program
    .description(description)
    .name(name)
    .version(version, '-v, --version')
    .requiredOption('-c, --config <path-to-yaml>', 'Path to YAML routing config file')
    .option('-s, --subscribe <server>', 'Server to subscribe')
    .parse(process.argv);

  echo(chalk.greenBright(drawText(name)));

  const options: MainOptions = program.opts();
  const routes: ConfiguredRoutes = getRoutes(options.config);

  const RESTServer = createRESTServer(routes);
  const port = process.env.PORT ?? 3000;

  RESTServer.listen(port, async () => {
      const server = RESTServer;
      const httpMode = 'http';

      interface ServerAddress {
        local: string[];
        network: string[];
      }

      const serverAddress: AddressInfo | string = server.address();
      const serverAddresses: ServerAddress = {
        local: [],
        network: [],
      };
      if (typeof serverAddress === 'string') {
        serverAddresses.local.push(serverAddress);
      } else if (typeof serverAddress === 'object' && serverAddress.port) {
        const address = serverAddress.address === '::' ? 'localhost' : serverAddress.address;
        serverAddresses.local.push(`${httpMode}://${address}:${port}`);

        interface NetworkInterfaceIP {
          address: string;
          netmask: string;
          mac: string;
          family: string;
          internal: boolean;
          cidr: string | null;
        }

        const networkInterfaces: NetworkInterfaceIP[] = flatten(Object.values(os.networkInterfaces())) as NetworkInterfaceIP[];
        const networkAddresses: string[] = networkInterfaces
          .filter(({family, internal}) => (family === 'IPv4' && !internal))
          .map(({address}) => {
            return `${httpMode}://${address}:${port}`;
          });
        serverAddresses.network.push(...networkAddresses);
      }

      const networkAddressesTable = new CliTable3({
        chars: {'mid': '', 'left-mid': '', 'mid-mid': '', 'right-mid': '', 'middle': '', 'bottom-mid': '', 'top-mid': ''},
        colWidths: [23, 33],
      });


      networkAddressesTable.push(
        [chalk.greenBright('Serving !'), ''],
        ['', ''],
      );

      serverAddresses.local.map((addr, index) => {
        const key = index === 0 ? '- Local:' : '';
        networkAddressesTable.push([key, addr]);
        return addr;
      });

      serverAddresses.network.map((addr, index) => {
        const key = index === 0 ? '- On your network:' : '';
        networkAddressesTable.push([key, addr]);
        return addr;
      });

      if (isDocker()) {
        const dockerNoticeTable = new CliTable3({
          chars: {
            'top': ''
            , 'top-mid': ''
            , 'top-left': ''
            , 'top-right': ''
            , 'bottom': ''
            , 'bottom-mid': ''
            , 'bottom-left': ''
            , 'bottom-right': ''
            , 'left': ''
            , 'left-mid': ''
            , 'right': ''
            , 'right-mid': ''
          },
          colWidths: [56],
          wordWrap: true,
        });
        dockerNoticeTable.push([
          chalk.grey(`You are using Docker, please expose port ${port} to make\n${name} accessible outside your container.`)
        ]);
        echo(dockerNoticeTable.toString());
      }

      echo(networkAddressesTable.toString());

      try {
        copyToClipboard(serverAddresses.local[0]);
        echo(chalk.grey('Copied local address to clipboard!\n'));
      } catch (clipboardCopyError) {
        debug.log(`Unable to copy in the clipboard \n${JSON.stringify(clipboardCopyError, null, 2)}`);
      }

      progress.start('Waiting for calls...');

    }
  );
};
