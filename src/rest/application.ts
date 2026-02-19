import bodyParser from 'body-parser';
import express from 'express';
import morgan from 'morgan';
import {ConfiguredRoutes} from "../cli/config.interface";
import {echo$} from "../util/echo";

export const createRESTApplication = (routes: ConfiguredRoutes): express.Express => {
  const application: express.Express = express();

  application.use(bodyParser.json());
  application.use(bodyParser.raw());
  application.use(bodyParser.text());
  application.use(bodyParser.urlencoded({extended: true}));

  application.use(morgan(
    ':date[clf] :method :url :status :res[content-length] ":user-agent"',
    {
      stream: echo$
    }
  ));

  application.get('/', (req, res) => {
    res.json({
      status: 'ok'
    }).end();
  });

  return application;
};
