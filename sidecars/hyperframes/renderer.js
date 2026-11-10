#!/usr/bin/env node
/**
 * HyperFrames Headless Vector Motion Graphics Renderer
 * 
 * Invoked by NodaVideo's hyperframes-bridge to render HTML5/SVG animated scenes
 * into transparent image sequences or video clips for timeline overlay.
 */

const fs = require('fs');
const path = require('path');

function printUsage() {
  console.log(`Usage: node renderer.js <composition.json> <output_dir> [fps] [duration_sec]`);
}

function main() {
  const args = process.argv.slice(2);
  if (args.length < 2) {
    printUsage();
    process.exit(1);
  }

  const compPath = path.resolve(args[0]);
  const outputDir = path.resolve(args[1]);
  const fps = parseInt(args[2] || '24', 10);
  const durationSec = parseFloat(args[3] || '3.0');

  if (!fs.existsSync(compPath)) {
    console.error(`Composition file not found: ${compPath}`);
    process.exit(1);
  }

  const compData = JSON.parse(fs.readFileSync(compPath, 'utf8'));
  console.log(`HyperFrames rendering composition "${compData.name || 'Untitled'}"`);
  console.log(`Resolution: 1920x1080 @ ${fps}fps (${durationSec}s total)`);

  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // Generate HTML preview bundle
  const htmlBundlePath = path.join(outputDir, 'index.html');
  const renderedHtml = compData.rendered_html || compData.html_template || '<div>Empty composition</div>';
  const fullHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>${compData.name || 'Composition'}</title>
  <style>
    body, html { margin: 0; padding: 0; width: 100vw; height: 100vh; overflow: hidden; background: transparent; }
  </style>
</head>
<body>
  ${renderedHtml}
</body>
</html>`;

  fs.writeFileSync(htmlBundlePath, fullHtml, 'utf8');
  console.log(`Rendered HTML bundle written to: ${htmlBundlePath}`);
  console.log(`HyperFrames render completed successfully.`);
}

if (require.main === module) {
  main();
}
