const ts = require("typescript");

module.exports = {
  process(sourceText, sourcePath) {
    if (/\.tsx?$/.test(sourcePath)) {
      const result = ts.transpileModule(sourceText, {
        compilerOptions: {
          module: ts.ModuleKind.CommonJS,
          target: ts.ScriptTarget.ES2021,
          esModuleInterop: true,
        },
        fileName: sourcePath,
        reportDiagnostics: false,
      });
      return { code: result.outputText };
    }
    return { code: sourceText };
  },
};
