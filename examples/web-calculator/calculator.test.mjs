import assert from "node:assert/strict";
import test from "node:test";

import { evaluateExpression } from "./calculator.mjs";

test("вычисляет операции с приоритетом, скобками и unary minus", () => {
  assert.equal(evaluateExpression("2 + 3 * 4"), "14");
  assert.equal(evaluateExpression("2 × (3 + 4)"), "14");
  assert.equal(evaluateExpression("-(2 + 3) ÷ .5"), "-10");
  assert.equal(evaluateExpression("1--2"), "3");
});

test("форматирует обычные десятичные результаты", () => {
  assert.equal(evaluateExpression("0.1 + 0.2"), "0.3");
  assert.equal(evaluateExpression("1 / 3"), "0.333333333333");
});

test("отклоняет пустые, неполные и некорректные выражения", () => {
  for (const expression of ["", " ", "1 +", "(1 + 2", "1..2", "2(3)", "+1"]) {
    assert.throws(() => evaluateExpression(expression));
  }
});

test("отклоняет деление на ноль и произвольный JavaScript", () => {
  assert.throws(() => evaluateExpression("1 / (2 - 2)"), /Деление на ноль/);
  for (const expression of ["globalThis.process.exit()", "fetch('https://example.com')", "2; throw 1"]) {
    assert.throws(() => evaluateExpression(expression), /Некорректное выражение/);
  }
});
