import { describe, it, expect } from "vitest";
import { fmtBytes } from "./wow";

describe("fmtBytes", () => {
  it("Bytes unter 1 KB", () => {
    expect(fmtBytes(512)).toBe("512 B");
  });

  it("Kilobytes", () => {
    expect(fmtBytes(2048)).toBe("2.0 KB");
  });

  it("Megabytes", () => {
    expect(fmtBytes(4775986)).toBe("4.6 MB");
  });

  it("Grenzwert exakt 1024 ist KB", () => {
    expect(fmtBytes(1024)).toBe("1.0 KB");
  });
});
