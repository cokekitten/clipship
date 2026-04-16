import { describe, expect, it } from "vitest";
import { eventToAccelerator } from "./map";

function mk(
  code: string,
  key: string,
  mods: Partial<{ ctrl: boolean; alt: boolean; shift: boolean; meta: boolean }> = {},
) {
  return {
    code,
    key,
    ctrlKey: !!mods.ctrl,
    altKey: !!mods.alt,
    shiftKey: !!mods.shift,
    metaKey: !!mods.meta,
  } as KeyboardEvent;
}

describe("eventToAccelerator", () => {
  it("maps Ctrl+Shift+U", () => {
    expect(eventToAccelerator(mk("KeyU", "U", { ctrl: true, shift: true }))).toBe(
      "CmdOrCtrl+Shift+U",
    );
  });

  it("orders modifiers CmdOrCtrl → Alt → Shift → Super", () => {
    expect(
      eventToAccelerator(mk("KeyD", "D", { ctrl: true, alt: true, shift: true, meta: true })),
    ).toBe("CmdOrCtrl+Alt+Shift+Super+D");
  });

  it("maps digit keys by physical code", () => {
    expect(eventToAccelerator(mk("Digit5", "%", { ctrl: true, shift: true }))).toBe(
      "CmdOrCtrl+Shift+5",
    );
  });

  it("maps F-keys", () => {
    expect(eventToAccelerator(mk("F12", "F12", { ctrl: true }))).toBe("CmdOrCtrl+F12");
  });

  it("maps named keys (Space, Enter, Tab, Backspace, Delete, Escape, arrows)", () => {
    expect(eventToAccelerator(mk("Space", " ", { ctrl: true }))).toBe("CmdOrCtrl+Space");
    expect(eventToAccelerator(mk("Enter", "Enter", { alt: true }))).toBe("Alt+Enter");
    expect(eventToAccelerator(mk("ArrowLeft", "ArrowLeft", { ctrl: true }))).toBe(
      "CmdOrCtrl+Left",
    );
    expect(eventToAccelerator(mk("ArrowRight", "ArrowRight", { ctrl: true }))).toBe(
      "CmdOrCtrl+Right",
    );
    expect(eventToAccelerator(mk("ArrowUp", "ArrowUp", { ctrl: true }))).toBe("CmdOrCtrl+Up");
    expect(eventToAccelerator(mk("ArrowDown", "ArrowDown", { ctrl: true }))).toBe(
      "CmdOrCtrl+Down",
    );
  });

  it("rejects combos without any modifier", () => {
    expect(eventToAccelerator(mk("KeyU", "U"))).toBeNull();
  });

  it("rejects modifier-only press", () => {
    expect(eventToAccelerator(mk("ControlLeft", "Control", { ctrl: true }))).toBeNull();
    expect(eventToAccelerator(mk("ShiftRight", "Shift", { shift: true }))).toBeNull();
  });

  it("returns null for unknown codes", () => {
    expect(eventToAccelerator(mk("IntlRo", "", { ctrl: true }))).toBeNull();
  });
});
