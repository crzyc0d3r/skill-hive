import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

const rawPalette =
  "\\b(?:bg|text|border|ring|from|to|via|fill|stroke)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\\d{2,3}\\b";

export default tseslint.config(
  { ignores: ["dist", "src-tauri", "node_modules", "scripts/fixtures", ".claude", "automations", "graphify-out"] },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      "no-restricted-syntax": [
        "error",
        {
          selector: `JSXAttribute[name.name='className'] Literal[value=/${rawPalette}/]`,
          message:
            "Raw palette classes are rejected. Use semantic tokens such as bg-primary or text-muted-foreground.",
        },
        {
          selector: `JSXAttribute[name.name='className'] TemplateElement[value.raw=/${rawPalette}/]`,
          message:
            "Raw palette classes are rejected. Use semantic tokens such as bg-primary or text-muted-foreground.",
        },
      ],
    },
  },
);
