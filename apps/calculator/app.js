(function (root) {
  'use strict';

  const OPERATORS = new Set(['+', '-', '*', '/']);

  function initialState() {
    return {
      display: '0',
      storedValue: null,
      pendingOperator: null,
      waitingForOperand: false,
      error: null,
    };
  }

  function formatNumber(value) {
    if (!Number.isFinite(value)) return 'Error';
    if (Object.is(value, -0)) return '0';
    return Number.parseFloat(value.toPrecision(12)).toString();
  }

  function calculate(left, operator, right) {
    if (operator === '+') return { value: left + right };
    if (operator === '-') return { value: left - right };
    if (operator === '*') return { value: left * right };
    if (operator === '/') {
      if (right === 0) return { error: 'Cannot divide by zero' };
      return { value: left / right };
    }
    return { value: right };
  }

  function inputDigit(state, digit) {
    if (!/^\d$/.test(digit)) return state;
    if (state.error) return { ...initialState(), display: digit };
    if (state.waitingForOperand) {
      return { ...state, display: digit, waitingForOperand: false, error: null };
    }
    return { ...state, display: state.display === '0' ? digit : state.display + digit };
  }

  function inputDecimal(state) {
    if (state.error) return { ...initialState(), display: '0.' };
    if (state.waitingForOperand) {
      return { ...state, display: '0.', waitingForOperand: false, error: null };
    }
    if (state.display.includes('.')) return state;
    return { ...state, display: state.display + '.' };
  }

  function inputOperator(state, operator) {
    if (!OPERATORS.has(operator)) return state;
    if (state.error) return state;

    const currentValue = Number(state.display);
    if (state.pendingOperator && state.waitingForOperand) {
      return { ...state, pendingOperator: operator };
    }
    if (state.storedValue === null) {
      return {
        ...state,
        storedValue: currentValue,
        pendingOperator: operator,
        waitingForOperand: true,
      };
    }

    const result = calculate(state.storedValue, state.pendingOperator, currentValue);
    if (result.error) {
      return { ...initialState(), display: result.error, error: result.error };
    }
    return {
      ...state,
      display: formatNumber(result.value),
      storedValue: result.value,
      pendingOperator: operator,
      waitingForOperand: true,
      error: null,
    };
  }

  function inputEquals(state) {
    if (state.error || !state.pendingOperator || state.storedValue === null || state.waitingForOperand) {
      return state;
    }

    const result = calculate(state.storedValue, state.pendingOperator, Number(state.display));
    if (result.error) {
      return { ...initialState(), display: result.error, error: result.error };
    }
    return {
      ...state,
      display: formatNumber(result.value),
      storedValue: null,
      pendingOperator: null,
      waitingForOperand: true,
      error: null,
    };
  }

  function inputBackspace(state) {
    if (state.error || state.waitingForOperand) return state;
    if (state.display.length <= 1) return { ...state, display: '0' };
    return { ...state, display: state.display.slice(0, -1) };
  }

  function applyAction(state, action) {
    if (!action || !action.type) return state;
    if (action.type === 'clear') return initialState();
    if (action.type === 'digit') return inputDigit(state, String(action.value ?? ''));
    if (action.type === 'decimal') return inputDecimal(state);
    if (action.type === 'operator') return inputOperator(state, action.value);
    if (action.type === 'equals') return inputEquals(state);
    if (action.type === 'backspace') return inputBackspace(state);
    return state;
  }

  function actionFromKey(key) {
    if (/^\d$/.test(key)) return { type: 'digit', value: key };
    if (key === '.') return { type: 'decimal' };
    if (key === '+' || key === '-' || key === '*' || key === '/') {
      return { type: 'operator', value: key };
    }
    if (key === 'Enter' || key === '=') return { type: 'equals' };
    if (key === 'Backspace') return { type: 'backspace' };
    if (key === 'Escape') return { type: 'clear' };
    return null;
  }

  function initCalculator(documentRef) {
    if (!documentRef) return null;
    const display = documentRef.querySelector('[data-display]');
    const keys = documentRef.querySelectorAll('[data-action]');
    if (!display || keys.length === 0) return null;

    let state = initialState();
    const render = () => {
      display.textContent = state.display;
      display.dataset.error = state.error ? 'true' : 'false';
    };
    const dispatch = (action) => {
      state = applyAction(state, action);
      render();
    };

    keys.forEach((key) => {
      key.addEventListener('click', () => {
        dispatch({ type: key.dataset.action, value: key.dataset.value });
      });
    });
    documentRef.addEventListener('keydown', (event) => {
      const action = actionFromKey(event.key);
      if (!action) return;
      event.preventDefault();
      dispatch(action);
    });
    render();
    return { getState: () => state, dispatch };
  }

  const api = {
    initialState,
    applyAction,
    actionFromKey,
    formatNumber,
    calculate,
    initCalculator,
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  }
  root.CalculatorApp = api;

  if (typeof document !== 'undefined') {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => initCalculator(document));
    } else {
      initCalculator(document);
    }
  }
})(typeof globalThis !== 'undefined' ? globalThis : window);
