import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor/editor/editor.api";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";

type MonacoWorkerEnvironment = typeof self & {
  MonacoEnvironment?: { getWorker: () => Worker };
};

(self as MonacoWorkerEnvironment).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

loader.config({ monaco });
