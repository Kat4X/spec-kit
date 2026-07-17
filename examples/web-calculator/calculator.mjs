const INVALID_EXPRESSION = "Некорректное выражение";

export function evaluateExpression(expression) {
  if (typeof expression !== "string" || !expression.trim()) {
    throw new Error("Введите выражение");
  }

  const source = expression.replaceAll("×", "*").replaceAll("÷", "/").replaceAll(/\s/g, "");
  if (!/^[\d.+*/()-]+$/.test(source)) throw new Error(INVALID_EXPRESSION);

  let position = 0;

  function parseExpression() {
    let value = parseTerm();
    while (source[position] === "+" || source[position] === "-") {
      const operator = source[position++];
      const right = parseTerm();
      value = operator === "+" ? value + right : value - right;
    }
    return value;
  }

  function parseTerm() {
    let value = parseUnary();
    while (source[position] === "*" || source[position] === "/") {
      const operator = source[position++];
      const right = parseUnary();
      if (operator === "/" && right === 0) throw new Error("Деление на ноль недопустимо");
      value = operator === "*" ? value * right : value / right;
    }
    return value;
  }

  function parseUnary() {
    if (source[position] === "-") {
      position++;
      return -parseUnary();
    }
    return parsePrimary();
  }

  function parsePrimary() {
    if (source[position] === "(") {
      position++;
      const value = parseExpression();
      if (source[position++] !== ")") throw new Error(INVALID_EXPRESSION);
      return value;
    }

    const start = position;
    while (/\d|\./.test(source[position] ?? "")) position++;
    const token = source.slice(start, position);
    if (!/^(?:\d+(?:\.\d*)?|\.\d+)$/.test(token)) throw new Error(INVALID_EXPRESSION);
    return Number(token);
  }

  let value;
  try {
    value = parseExpression();
  } catch (error) {
    if (error instanceof RangeError) throw new Error("Выражение слишком длинное");
    throw error;
  }

  if (position !== source.length) throw new Error(INVALID_EXPRESSION);
  if (!Number.isFinite(value)) throw new Error("Результат вне допустимого диапазона");
  return String(Number(value.toPrecision(12)));
}
