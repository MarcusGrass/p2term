#!/bin/bash
set -e

wasm-pack build p2term-web --target web

# Keeping the path structure just for convenience of running local/remote without changing any html
mkdir -p ./dist/node_modules/@xterm/xterm/lib/
mkdir -p ./dist/node_modules/@xterm/xterm/css/
mkdir -p ./dist/pkg

cp ./p2term-web/node_modules/@xterm/xterm/lib/xterm.js ./dist/node_modules/@xterm/xterm/lib/xterm.js
cp ./p2term-web/node_modules/@xterm/xterm/css/xterm.css ./dist/node_modules/@xterm/xterm/css/xterm.css
cp ./p2term-web/index.html ./dist/index.html
cp ./p2term-web/pkg/p2term_web.js ./dist/pkg/
cp ./p2term-web/pkg/p2term_web_bg.wasm ./dist/pkg/

rsync -avP --delete ./dist/ gramar@mgrass.dev:/home/gramar/p2term/
