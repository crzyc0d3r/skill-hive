import { ESLint } from "eslint";
import tseslint from "typescript-eslint";

const rawPalette =
  "\\b(?:bg|text|border|ring|from|to|via|fill|stroke)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\\d{2,3}\\b";

const eslint = new ESLint({
  overrideConfigFile: true,
  overrideConfig: [
    {
      files: ["scripts/fixtures/**/*.{ts,tsx}"],
      languageOptions: {
        parser: tseslint.parser,
        parserOptions: { ecmaFeatures: { jsx: true }, sourceType: "module" },
      },
      rules: {
        "no-restricted-syntax": [
          "error",
          {
            selector: `JSXAttribute[name.name='className'] Literal[value=/${rawPalette}/]`,
            message: "Raw palette classes are rejected.",
          },
        ],
      },
    },
  ],
});

const file = "scripts/fixtures/bad-palette.tsx";
const results = await eslint.lintFiles([file]);
const errors = results.flatMap((result) => result.messages.filter((message) => message.severity === 2));
if (errors.length === 0) {
  console.error(`${file} was expected to fail the raw-palette lint`);
  process.exit(1);
}
console.log(`${file} rejected (${errors.length} error${errors.length === 1 ? "" : "s"})`);
for (const error of errors) console.log(`  ${error.line}:${error.column} ${error.message}`);
