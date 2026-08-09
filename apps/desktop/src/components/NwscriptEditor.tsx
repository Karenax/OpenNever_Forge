import Editor, { type Monaco } from "@monaco-editor/react";

type NwscriptEditorProps = {
  value: string;
  readOnly: boolean;
  onChange: (value: string) => void;
};

export default function NwscriptEditor({ value, readOnly, onChange }: NwscriptEditorProps) {
  return (
    <Editor
      height="430px"
      theme="opennever-dark"
      language="nwscript"
      value={value}
      onChange={(nextValue) => onChange(nextValue ?? "")}
      beforeMount={configureNwscriptMonaco}
      options={{
        readOnly,
        domReadOnly: readOnly,
        automaticLayout: true,
        minimap: { enabled: false },
        fontFamily: "Cascadia Code, Consolas, monospace",
        fontSize: 12,
        scrollBeyondLastLine: false,
        renderWhitespace: "selection",
      }}
    />
  );
}

function configureNwscriptMonaco(monaco: Monaco) {
  if (!monaco.languages.getLanguages().some((language: { id: string }) => language.id === "nwscript")) {
    monaco.languages.register({ id: "nwscript" });
  }
  monaco.languages.setMonarchTokensProvider("nwscript", {
    keywords: ["break", "case", "const", "continue", "default", "do", "else", "for", "if", "return", "struct", "switch", "while"],
    typeKeywords: ["void", "int", "float", "string", "object", "vector", "location", "effect", "event", "itemproperty", "talent", "sqlquery", "json"],
    tokenizer: {
      root: [
        [/[a-zA-Z_]\w*/, { cases: { "@keywords": "keyword", "@typeKeywords": "type", "@default": "identifier" } }],
        [/\/\/.*$/, "comment"],
        [/\/\*/, "comment", "@comment"],
        [/"([^"\\]|\\.)*$/, "string.invalid"],
        [/"/, "string", "@string"],
        [/#\s*include/, "keyword.directive"],
        [/[{}()[\]]/, "@brackets"],
        [/[0-9]+(\.[0-9]+)?/, "number"],
      ],
      comment: [[/[^/*]+/, "comment"], [/\*\//, "comment", "@pop"], [/[/*]/, "comment"]],
      string: [[/[^\\"]+/, "string"], [/\\./, "string.escape"], [/"/, "string", "@pop"]],
    },
  });
  monaco.editor.defineTheme("opennever-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "D59B55" },
      { token: "type", foreground: "73B4E8" },
      { token: "comment", foreground: "66798A" },
    ],
    colors: {
      "editor.background": "#0D1217",
      "editor.lineHighlightBackground": "#151C23",
    },
  });
}
