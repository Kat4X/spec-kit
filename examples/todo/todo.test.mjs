import assert from "node:assert/strict";
import test from "node:test";

import { addTodo, parseTodos, removeTodo, toggleTodo } from "./todo.mjs";

test("adds trimmed non-empty todos without mutating the list", () => {
  const original = [];
  const added = addTodo(original, "  Проверить Spec Kit  ");

  assert.deepEqual(original, []);
  assert.equal(added.length, 1);
  assert.equal(added[0].text, "Проверить Spec Kit");
  assert.equal(added[0].completed, false);
  assert.ok(added[0].id);
  assert.strictEqual(addTodo(added, "   "), added);
});

test("toggles and removes duplicate texts independently", () => {
  const first = addTodo([], "Повтор");
  const todos = addTodo(first, "Повтор");
  const toggled = toggleTodo(todos, todos[0].id);

  assert.equal(todos[0].completed, false);
  assert.equal(toggled[0].completed, true);
  assert.equal(toggled[1].completed, false);
  assert.deepEqual(removeTodo(toggled, toggled[0].id), [toggled[1]]);
});

test("parses valid state and rejects the whole invalid state", () => {
  const valid = [{ id: "one", text: "  Задача  ", completed: true }];
  assert.deepEqual(parseTodos(JSON.stringify(valid)), [{ ...valid[0], text: "Задача" }]);

  for (const json of ["broken", "{}", JSON.stringify([{ ...valid[0], completed: 1 }]),
    JSON.stringify([valid[0], valid[0]]), JSON.stringify([{ ...valid[0], extra: true }])]) {
    assert.deepEqual(parseTodos(json), []);
  }
});
