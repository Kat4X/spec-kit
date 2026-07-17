function isTodo(todo) {
  return todo && typeof todo === "object" && !Array.isArray(todo) &&
    Object.keys(todo).sort().join() === "completed,id,text" &&
    typeof todo.id === "string" && todo.id.length > 0 &&
    typeof todo.text === "string" && todo.text.trim().length > 0 &&
    typeof todo.completed === "boolean";
}

export function parseTodos(json) {
  try {
    const todos = JSON.parse(json);
    if (!Array.isArray(todos) || !todos.every(isTodo) ||
        new Set(todos.map(({ id }) => id)).size !== todos.length) return [];
    return todos.map((todo) => ({ ...todo, text: todo.text.trim() }));
  } catch {
    return [];
  }
}

export function addTodo(todos, text) {
  const trimmed = typeof text === "string" ? text.trim() : "";
  if (!trimmed) return todos;

  let id;
  do id = crypto.randomUUID(); while (todos.some((todo) => todo.id === id));
  return [...todos, { id, text: trimmed, completed: false }];
}

export function toggleTodo(todos, id) {
  return todos.map((todo) => todo.id === id ? { ...todo, completed: !todo.completed } : todo);
}

export function removeTodo(todos, id) {
  return todos.filter((todo) => todo.id !== id);
}
