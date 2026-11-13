// Renders every page in pages/ to a standalone HTML file in ../pages/.
// Run with: deno task build

import type { Markup } from "./jsx/jsx-runtime.ts";
import { COLOR, FONT_MONO, FONT_SANS } from "./theme.ts";
import { compile } from "tailwindcss";
import { DESIGN_PAGES, DesignNavigation } from "./components/sidebar-nav.tsx";

const pagesDirectory = new URL("./pages/", import.meta.url);
const distDirectory = new URL("../pages/", import.meta.url);
await Deno.mkdir(distDirectory, { recursive: true });

const theme = await Deno.readTextFile(new URL(import.meta.resolve("tailwindcss/theme.css")));
const preflight = await Deno.readTextFile(new URL(import.meta.resolve("tailwindcss/preflight.css")));
const styles = await Deno.readTextFile(new URL("./styles.css", import.meta.url));
const colors = Object.entries(COLOR).map(([name, value]) => {
  const cssName = name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
  return `--color-${cssName}:${value};`;
}).join("\n");
const stylesheet = `
@layer theme, base, components, utilities;
@layer theme { ${theme} }
@layer base { ${preflight} }
@theme { ${colors} --font-sans:${FONT_SANS}; --font-mono:${FONT_MONO}; }
${styles}
@tailwind utilities;
`;

const pageFiles = [...Deno.readDirSync(pagesDirectory)]
  .filter((entry) => entry.isFile && entry.name.endsWith(".tsx"))
  .map((entry) => entry.name)
  .sort();

const index = new Map(DESIGN_PAGES.map((page) => [page.file, page]));
const pageNames = new Set(pageFiles.map((fileName) => fileName.replace(/\.tsx$/, "")));
const navigationPages = [
  ...DESIGN_PAGES.filter((page) => pageNames.has(page.file)),
  ...[...pageNames].filter((name) => !index.has(name)).map((name) => ({
    file: name,
    label: name,
    group: "Other",
  })),
];

for (const fileName of pageFiles) {
  const name = fileName.replace(/\.tsx$/, "");
  const module = await import(new URL(fileName, pagesDirectory).href);
  const page: () => Markup = module.default;
  const markup = page().html;
  const navigation = DesignNavigation({ current: name, pages: navigationPages });
  const content = `${navigation.html}<div class="design-stage">${markup}</div>`;
  const classes = [...content.matchAll(/\bclass="([^"]*)"/g)].flatMap((match) =>
    match[1].replaceAll("&quot;", '"').replaceAll("&lt;", "<").replaceAll("&gt;", ">").replaceAll("&amp;", "&")
      .split(/\s+/)
  );
  const compiler = await compile(stylesheet);
  const css = compiler.build(classes);
  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${index.get(name)?.label ?? name} · NodaVideo design</title>
<style>${css}</style>
<style>body{margin:0;background:#fefefe}svg{display:block}</style>
</head>
<body>
${content}
</body>
</html>
`;
  // The output is static markup; JSX only runs here, at build time.
  if (/<script\b|\son[a-z]+=/i.test(html)) {
    throw new Error(`${name}: the built HTML must not contain JavaScript`);
  }
  await Deno.writeTextFile(new URL(`${name}.html`, distDirectory), html);
  console.log(`pages/${name}.html`);
}
