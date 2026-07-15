import assert from 'node:assert/strict';
import calculator from '../apps/calculator/app.js';

const { initialState, applyAction, actionFromKey } = calculator;

function run(actions) {
  return actions.reduce((state, action) => applyAction(state, action), initialState());
}

function digit(value) {
  return { type: 'digit', value: String(value) };
}

function op(value) {
  return { type: 'operator', value };
}

assert.equal(run([digit(2), op('+'), digit(2), { type: 'equals' }]).display, '4');
assert.equal(run([digit(7), op('*'), digit(8), { type: 'equals' }]).display, '56');

const divideByZero = run([digit(9), op('/'), digit(0), { type: 'equals' }]);
assert.equal(divideByZero.error, 'Cannot divide by zero');
assert.equal(divideByZero.display, 'Cannot divide by zero');
assert.equal(applyAction(divideByZero, { type: 'clear' }).display, '0');

const decimal = run([digit(1), { type: 'decimal' }, digit(2), { type: 'decimal' }, digit(3)]);
assert.equal(decimal.display, '1.23');

const cleared = applyAction(run([digit(4), digit(2)]), { type: 'clear' });
assert.deepEqual(cleared, initialState());

const backspaced = applyAction(run([digit(4), digit(2)]), { type: 'backspace' });
assert.equal(backspaced.display, '4');
assert.equal(applyAction(backspaced, { type: 'backspace' }).display, '0');

assert.equal(run([digit(0), { type: 'decimal' }, digit(1), op('+'), digit(0), { type: 'decimal' }, digit(2), { type: 'equals' }]).display, '0.3');

const repeatedOperator = run([digit(5), op('+'), op('*'), digit(3), { type: 'equals' }]);
assert.equal(repeatedOperator.display, '15');

const repeatedEquals = run([digit(5), op('+'), digit(3), { type: 'equals' }, { type: 'equals' }]);
assert.equal(repeatedEquals.display, '8');

assert.deepEqual(actionFromKey('Enter'), { type: 'equals' });
assert.deepEqual(actionFromKey('Backspace'), { type: 'backspace' });
assert.deepEqual(actionFromKey('Escape'), { type: 'clear' });
assert.deepEqual(actionFromKey('/'), { type: 'operator', value: '/' });

console.log('calculator checks passed');
