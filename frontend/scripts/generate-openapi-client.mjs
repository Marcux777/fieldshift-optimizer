// AI-assisted (OpenBot agent)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const openapiPath = path.resolve(__dirname, '../../backend/crates/fieldshift-api/openapi.json');
const outPath = path.resolve(__dirname, '../src/lib/api/openapi.gen.ts');

const spec = JSON.parse(fs.readFileSync(openapiPath, 'utf8'));

function schemaToTs(schema, indent = '') {
  if (!schema || Object.keys(schema).length === 0) {
    return 'unknown';
  }
  if (schema.$ref) {
    const refName = schema.$ref.split('/').pop();
    return schema.nullable ? `${refName} | null` : refName;
  }
  let base = 'unknown';
  if (schema.enum) {
    base = schema.enum.map((v) => JSON.stringify(v)).join(' | ');
  } else if (schema.oneOf) {
    base = schema.oneOf.map((s) => schemaToTs(s, indent)).join(' | ');
  } else if (schema.type === 'string') {
    base = 'string';
  } else if (schema.type === 'number' || schema.type === 'integer') {
    base = 'number';
  } else if (schema.type === 'boolean') {
    base = 'boolean';
  } else if (schema.type === 'array') {
    const itemType = schemaToTs(schema.items, indent);
    base = `Array<${itemType}>`;
  } else if (schema.type === 'object') {
    if (schema.properties) {
      const req = new Set(schema.required ?? []);
      const lines = Object.entries(schema.properties).map(([k, v]) => {
        const opt = req.has(k) ? '' : '?';
        return `${indent}  readonly ${JSON.stringify(k)}${opt}: ${schemaToTs(v, indent + '  ')};`;
      });
      base = `{\n${lines.join('\n')}\n${indent}}`;
    } else if (schema.additionalProperties) {
      const valType = schemaToTs(schema.additionalProperties, indent);
      base = `Record<string, ${valType}>`;
    } else {
      base = 'Record<string, unknown>';
    }
  }
  return schema.nullable ? `${base} | null` : base;
}

const out = [
  '// AI-assisted (OpenBot agent)',
  '// AUTO-GENERATED FROM backend/crates/fieldshift-api/openapi.json — DO NOT EDIT MANUALLY.',
  `// OpenAPI title: ${spec.info.title} (v${spec.info.version})`,
  ''
];

for (const [name, schema] of Object.entries(spec.components.schemas)) {
  out.push(`export type ${name} = ${schemaToTs(schema)};\n`);
}

const paretoSchema =
  spec.paths['/v1/pareto'].post.responses['200'].content['application/json'].schema;
out.push(`export type ParetoResponse = ${schemaToTs(paretoSchema)};\n`);

out.push(`export interface OpenApiClientOptions {
  readonly baseUrl?: string;
  readonly fetchFn?: typeof fetch;
}

export class OpenApiHttpError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(params: { readonly status: number; readonly code: string; readonly message: string }) {
    super(params.message);
    this.name = 'OpenApiHttpError';
    this.status = params.status;
    this.code = params.code;
  }
}

export interface GeneratedOpenApiClient {
  health(): Promise<Health>;
  meta(): Promise<Meta>;
  catalog(): Promise<Catalog>;
  plans(body: Inputs): Promise<Plans>;
  run(body: Inputs): Promise<RunResult>;
  evaluate(body: Inputs): Promise<EvalReport>;
  pareto(body: Inputs): Promise<ParetoResponse>;
  provenance(): Promise<Provenance>;
  limitations(): Promise<Limitations>;
}
`);

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, out.join('\n'));
console.log(`Generated ${path.relative(process.cwd(), outPath)}`);
