import { evaluateExpression } from "./calculator.mjs";

const STORAGE_KEY = "web-calculator.history.v1";
const form = document.querySelector("#calculator-form");
const input = document.querySelector("#expression");
const message = document.querySelector("#message");
const historyList = document.querySelector("#history-list");
const emptyHistory = document.querySelector("#empty-history");
const clearHistory = document.querySelector("#clear-history");

function isEntry(entry) {
  return entry && typeof entry === "object" && !Array.isArray(entry) &&
    Object.keys(entry).sort().join() === "createdAt,expression,result" &&
    typeof entry.expression === "string" && entry.expression.length > 0 &&
    typeof entry.result === "string" && entry.result.length > 0 &&
    typeof entry.createdAt === "string" && !Number.isNaN(Date.parse(entry.createdAt));
}

function loadHistory() {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    return Array.isArray(value) && value.every(isEntry) ? value : [];
  } catch {
    return [];
  }
}

let history = loadHistory();

function saveHistory() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(history));
  } catch {
    // History still works for the current tab when storage is unavailable.
  }
}

function renderHistory() {
  historyList.replaceChildren(...history.map((entry) => {
    const item = document.createElement("li");
    const button = document.createElement("button");
    const result = document.createElement("strong");
    button.type = "button";
    button.className = "history-entry";
    button.append(document.createTextNode(entry.expression), result);
    result.textContent = `= ${entry.result}`;
    button.addEventListener("click", () => {
      input.value = entry.expression;
      input.focus();
    });
    item.append(button);
    return item;
  }));
  emptyHistory.hidden = history.length > 0;
  clearHistory.disabled = history.length === 0;
}

function show(text, error = false) {
  message.textContent = text;
  message.classList.toggle("error", error);
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  try {
    const expression = input.value;
    const result = evaluateExpression(expression);
    show(result);
    history.unshift({ expression, result, createdAt: new Date().toISOString() });
    saveHistory();
    renderHistory();
    input.select();
  } catch (error) {
    show(error instanceof Error ? error.message : "Не удалось вычислить выражение", true);
    input.focus();
  }
});

form.querySelector(".keys").addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button || button.type === "submit") return;
  if (button.dataset.action === "clear") input.value = "";
  else input.setRangeText(button.dataset.value, input.selectionStart, input.selectionEnd, "end");
  input.focus();
});

clearHistory.addEventListener("click", () => {
  if (!confirm("Удалить всю историю вычислений?")) return;
  history = [];
  try { localStorage.removeItem(STORAGE_KEY); } catch { /* memory fallback is already clear */ }
  renderHistory();
});

renderHistory();
