import { describe, it, expect } from "vitest";
import { identityLabel, fmtBytes, type ExeIdentity } from "./wow";

describe("identityLabel", () => {
  it("offiziell zeigt Version + Locale", () => {
    const id: ExeIdentity = { status: "official", version: "1.12.1", locale: "enUS" };
    expect(identityLabel(id)).toBe("✓ Offiziell 1.12.1 (enUS)");
  });

  it("modifiziert zeigt behauptete Version", () => {
    const id: ExeIdentity = { status: "modified", claims_version: "1.12.1" };
    expect(identityLabel(id)).toContain("Modifiziert");
    expect(identityLabel(id)).toContain("1.12.1");
  });

  it("unbekannter Build", () => {
    expect(identityLabel({ status: "unknown-build" })).toContain("Unbekannter Build");
  });

  it("kein WoW-Client", () => {
    expect(identityLabel({ status: "unknown" })).toContain("Kein erkennbarer");
  });
});

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
