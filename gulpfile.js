const gulp = require('gulp');
const ts = require('gulp-typescript');
const plumber = require('gulp-plumber');
const babel = require('gulp-babel');
const chmod = require('gulp-chmod');
const insert = require('gulp-insert');
const logger = require('fancy-log');
const webpack = require('webpack-stream');

const fs = require('fs');
const path = require('path');

const babelRc = JSON.parse(fs.readFileSync(path.join(__dirname, '.babelrc'), 'utf8'));

const webpackConfig = require('./webpack.config.js');

const tsProject = ts.createProject('tsconfig.json');
const { outDir } = tsProject.config.compilerOptions;

const shebangJS = '#!/usr/bin/env node';

gulp.task('ts-compile', () => {
  return tsProject
    .src()
    .pipe(
      plumber({
        errorHandler(err) {
          logger.error(err.stack);
        },
      }),
    )
    .pipe(tsProject())
    .js.pipe(babel(babelRc))
    .pipe(gulp.dest(outDir));
});

gulp.task('bundle', () => {
  return gulp
    .src(`${outDir}/index.js`)
    .pipe(webpack(webpackConfig))
    .pipe(chmod(0o755))
    .pipe(insert.prepend([shebangJS, '\n', '\n'].join('')))
    .pipe(gulp.dest('bin'));
});

gulp.task('build', gulp.series('ts-compile', 'bundle'));

gulp.task('watch', () => {
  gulp.watch('src/**/*', gulp.task('build'));
});

gulp.task('default', gulp.task('build'));
