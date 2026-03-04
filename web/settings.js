const invoke = window.__TAURI__.core.invoke;

const shortcutInput = document.getElementById("shortcut");
const recordShortcutButton = document.getElementById("recordShortcut");
const alwaysOnTopCheckbox = document.getElementById("alwaysOnTop");
const launchAtLoginCheckbox = document.getElementById("launchAtLogin");
const statusElement = document.getElementById("status");
let isRecordingShortcut = false;

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);
const SPECIAL_KEY_BY_CODE = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Escape: "Esc",
};

function setStatus(message, isError = false) {
  statusElement.textContent = message;
  statusElement.classList.toggle("error", isError);
}

function normalizeShortcut(input) {
  return input.trim().replace(/\s+/g, "");
}

function getPrimaryKey(event) {
  const code = event.code;
  if (typeof code !== "string" || code.length === 0) {
    return null;
  }
  if (code.startsWith("Key") && code.length === 4) {
    return code.slice(3).toUpperCase();
  }
  if (code.startsWith("Digit") && code.length === 6) {
    return code.slice(5);
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) {
    return code;
  }
  return SPECIAL_KEY_BY_CODE[code] ?? null;
}

function getModifierParts(event) {
  const parts = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }
  if (event.metaKey) {
    parts.push(navigator.userAgent.includes("Mac") ? "Command" : "Super");
  }
  return parts;
}

function buildShortcutFromEvent(event) {
  if (MODIFIER_KEYS.has(event.key)) {
    return null;
  }
  const key = getPrimaryKey(event);
  if (!key) {
    return null;
  }
  const modifiers = getModifierParts(event);
  if (modifiers.length === 0) {
    return null;
  }
  return [...modifiers, key].join("+");
}

function setRecordingState(recording) {
  isRecordingShortcut = recording;
  if (recording) {
    recordShortcutButton.textContent = "Listening...";
    recordShortcutButton.classList.add("recording");
  } else {
    recordShortcutButton.textContent = "Record Shortcut";
    recordShortcutButton.classList.remove("recording");
  }
}

async function loadSettings() {
  try {
    const settings = await invoke("get_settings");
    shortcutInput.value = settings.shortcut ?? "";
    alwaysOnTopCheckbox.checked = Boolean(settings.alwaysOnTop);
    launchAtLoginCheckbox.checked = Boolean(settings.launchAtLogin);
    setStatus("Loaded.");
  } catch (error) {
    setStatus(`Failed to load settings: ${String(error)}`, true);
  }
}

async function saveSettings() {
  const shortcut = normalizeShortcut(shortcutInput.value);
  if (!shortcut) {
    setStatus("Shortcut cannot be empty.", true);
    return;
  }

  try {
    const settings = await invoke("save_settings", {
      payload: {
        shortcut,
        alwaysOnTop: alwaysOnTopCheckbox.checked,
        launchAtLogin: launchAtLoginCheckbox.checked,
      },
    });
    shortcutInput.value = settings.shortcut;
    alwaysOnTopCheckbox.checked = Boolean(settings.alwaysOnTop);
    launchAtLoginCheckbox.checked = Boolean(settings.launchAtLogin);
    setStatus("Saved. Changes are active now.");
  } catch (error) {
    setStatus(`Save failed: ${String(error)}`, true);
  }
}

document.getElementById("save").addEventListener("click", saveSettings);
recordShortcutButton.addEventListener("click", () => {
  if (isRecordingShortcut) {
    setRecordingState(false);
    setStatus("Shortcut recording canceled.");
    return;
  }
  setRecordingState(true);
  setStatus("Listening... press your shortcut combination now.");
});

window.addEventListener("keydown", (event) => {
  if (!isRecordingShortcut) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();

  const shortcut = buildShortcutFromEvent(event);
  if (!shortcut) {
    setStatus(
      "Use at least one modifier (Ctrl/Alt/Shift/Command) + key (A-Z, 0-9, F1-F24, Space, Enter, Tab, Esc).",
      true
    );
    return;
  }

  shortcutInput.value = shortcut;
  setRecordingState(false);
  setStatus(`Captured: ${shortcut}`);
});

document.getElementById("toggleMain").addEventListener("click", async () => {
  try {
    await invoke("toggle_main_window_cmd");
  } catch (error) {
    setStatus(`Unable to toggle overlay: ${String(error)}`, true);
  }
});

document.getElementById("goHome").addEventListener("click", async () => {
  try {
    await invoke("open_main_home_cmd");
  } catch (error) {
    setStatus(`Unable to open grok.com: ${String(error)}`, true);
  }
});

window.addEventListener("focus", loadSettings);
loadSettings();
