export const ROW_HEIGHT = 19;
export const HEADER_ROW_HEIGHT = 22;
export const PADDING_X = 12;
export const PADDING_Y = 6;
export const MIN_WIDTH = 170;

const VISIBILITY_SYMBOLS = {
  public: '+',
  private: '-',
  protected: '#',
};

export function visibilitySymbol(visibility) {
  return VISIBILITY_SYMBOLS[visibility] || '+';
}

export function stereotypeFor(kind) {
  if (kind === 'interface') return 'interface';
  if (kind === 'enum') return 'enumeration';
  if (kind === 'abstractClass') return 'abstract';
  if (kind === 'trait') return 'trait';
  return null;
}

export function methodSignature(method) {
  const params = method.params.map((p) => `${p.name}: ${p.typeName}`).join(', ');
  const returnType = method.returnType ? `: ${method.returnType}` : '';
  return `${visibilitySymbol(method.visibility)} ${method.name}(${params})${returnType}`;
}

export function fieldSignature(field) {
  const typeSuffix = field.typeName ? `: ${field.typeName}` : '';
  return `${visibilitySymbol(field.visibility)} ${field.name}${typeSuffix}`;
}

// Each member line carries its own source `line` (not just its display
// text) so the webview can offer per-field/per-method "jump to source",
// not just per-class.
export function classCompartments(cls) {
  if (cls.kind === 'enum') {
    return {
      header: [cls.name],
      sections: [cls.fields.map((f) => ({ text: f.name, line: f.line }))],
    };
  }
  return {
    header: [cls.name],
    sections: [
      cls.fields.map((f) => ({ text: fieldSignature(f), line: f.line })),
      cls.methods.map((m) => ({ text: methodSignature(m), line: m.line })),
    ],
  };
}
