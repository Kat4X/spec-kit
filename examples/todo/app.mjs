import { addTodo, parseTodos, removeTodo, toggleTodo } from "./todo.mjs";

const STORAGE_KEY = "spec-kit.todo.v1";
const form = document.querySelector("#todo-form");
const input = document.querySelector("#todo-text");
const list = document.querySelector("#todo-list");
const emptyState = document.querySelector("#empty-state");

function loadTodos() {
  try {
    return parseTodos(localStorage.getItem(STORAGE_KEY) ?? "[]");
  } catch {
    return [];
  }
}

let todos = loadTodos();

function saveTodos() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(todos));
  } catch {
    // The current tab keeps working when storage is unavailable.
  }
}

function render() {
  list.replaceChildren(...todos.map((todo) => {
    const item = document.createElement("li");
    const toggle = document.createElement("input");
    const text = document.createElement("span");
    const remove = document.createElement("button");

    item.classList.toggle("completed", todo.completed);
    toggle.type = "checkbox";
    toggle.checked = todo.completed;
    toggle.setAttribute("aria-label", todo.completed
      ? `Вернуть задачу «${todo.text}» в работу`
      : `Отметить задачу «${todo.text}» выполненной`);
    toggle.addEventListener("change", () => update(toggleTodo(todos, todo.id)));
    text.className = "todo-text";
    text.textContent = todo.text;
    remove.type = "button";
    remove.className = "delete";
    remove.textContent = "Удалить";
    remove.setAttribute("aria-label", `Удалить задачу «${todo.text}»`);
    remove.addEventListener("click", () => update(removeTodo(todos, todo.id)));
    item.append(toggle, text, remove);
    return item;
  }));
  emptyState.hidden = todos.length > 0;
}

function update(nextTodos) {
  todos = nextTodos;
  saveTodos();
  render();
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const nextTodos = addTodo(todos, input.value);
  if (nextTodos === todos) return;
  input.value = "";
  update(nextTodos);
  input.focus();
});

render();
