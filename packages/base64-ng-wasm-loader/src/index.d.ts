export type Base64Codec =
  | "standard"
  | "standard-no-pad"
  | "url-safe"
  | "url-safe-no-pad";

export const Codecs: Readonly<{
  STANDARD: "standard";
  STANDARD_NO_PAD: "standard-no-pad";
  URL_SAFE: "url-safe";
  URL_SAFE_NO_PAD: "url-safe-no-pad";
}>;

export class Base64NgError extends Error {
  readonly code: string;
  readonly index?: number;
  readonly required?: number;
  readonly limit?: number;
}

export interface Base64NgPosture {
  readonly capabilityEvidence: "embedded-probe-validated" | "embedded-probe-rejected";
  readonly selectedArtifact: "scalar" | "simd128";
  readonly selectionReason: string;
  readonly artifactPosture: "scalar" | "simd128";
  readonly artifactSha256: string;
  readonly abiVersion: 1;
  readonly maxInputLength: number;
  readonly maxOutputLength: number;
  readonly maximumMemoryPages: number;
  readonly maxArtifactBytes: number;
  readonly secretApi: false;
}

export interface Base64NgWasm {
  readonly posture: Base64NgPosture;
  encodedLength(input: Uint8Array, codec?: Base64Codec): number;
  decodedLength(input: Uint8Array, codec?: Base64Codec): number;
  encode(input: Uint8Array, codec?: Base64Codec): Uint8Array;
  decode(input: Uint8Array, codec?: Base64Codec): Uint8Array;
  encodeInto(input: Uint8Array, destination: Uint8Array, codec?: Base64Codec): number;
  decodeInto(input: Uint8Array, destination: Uint8Array, codec?: Base64Codec): number;
  decodeForgiving(input: Uint8Array): Uint8Array;
  dispose(): void;
}

export interface Base64NgOptions {
  artifact?: "auto" | "scalar" | "simd128";
  scalarArtifact?: URL | string | Uint8Array;
  simdArtifact?: URL | string | Uint8Array;
  scalarArtifactSha256?: string;
  simdArtifactSha256?: string;
  maxInputLength?: number;
  maxOutputLength?: number;
  maximumMemoryPages?: number;
}

export function createBase64Ng(options?: Base64NgOptions): Promise<Base64NgWasm>;
