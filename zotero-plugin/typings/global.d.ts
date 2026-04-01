/// <reference types="zotero-types" />

declare const ZoteroRAG: typeof import("../src/index");

declare const Components: any;
declare const Services: {
  scriptloader: {
    loadSubScript(url: string, scope?: object): void;
  };
  [key: string]: any;
};
declare const Cu: any;
declare const Cc: any;
declare const Ci: any;
declare const ChromeUtils: {
  importESModule(url: string): any;
  import(url: string): any;
};

declare const IOUtils: {
  read(path: string): Promise<Uint8Array>;
  writeUTF8(path: string, data: string): Promise<void>;
  exists(path: string): Promise<boolean>;
};

declare const PathUtils: {
  filename(path: string): string;
  join(...parts: string[]): string;
  parent(path: string): string;
};

declare const APP_SHUTDOWN: number;
