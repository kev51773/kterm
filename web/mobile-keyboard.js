(function () {
  'use strict';

  var SWIPE_THRESHOLD2 = 26 * 26;

  function charKey(page, sid, char, code, keyCode) {
    return { kind: 'char', id: page + '_' + sid, sid: sid, char: char, code: code, keyCode: keyCode, w: 1, fixed: true };
  }

  function plainKey(page, sid, char, code, keyCode) {
    return { kind: 'char', id: page + '_' + sid, sid: null, char: char, code: code, keyCode: keyCode, w: 1, fixed: true };
  }

  function slashDef(page) {
    var d = plainKey(page, 'slash', '/', 'Slash', 191);
    d.alt = { char: '\\', code: 'Backslash', keyCode: 220 };
    return d;
  }

  function keyKey(page, id, label, code, key, keyCode, w, noRepeat) {
    return { kind: 'key', id: page + '_' + id, label: label, code: code, key: key, keyCode: keyCode, w: w || 1, noRepeat: !!noRepeat };
  }

  function modKey(page, id, mod, label, code, key, keyCode, w) {
    return { kind: 'mod', id: page + '_' + id, mod: mod, label: label, code: code, key: key, keyCode: keyCode, w: w || 1 };
  }

  function switchKey(page, id, mode, w) {
    return { kind: 'switch', id: page + '_' + id, mode: mode, label: '', code: '', keyCode: 0, w: w || 1 };
  }

  function toolSwitch(page, id, mode, label, w) {
    var d = switchKey(page, id, mode, w);
    d.label = label;
    d.cls = 'mk-tool';
    return d;
  }

  function shiftDef(page) {
    var d = modKey(page, 'shift', 'shift', '⇧', 'Shift', 'Shift', 16, 1.5);
    d.cls = 'mk-tool';
    d.svg = 'M12 5l7 8h-4v6H9v-6H5z';
    d.svgFill = true;
    return d;
  }

  function bspDef(page) {
    var d = keyKey(page, 'backspace', '⌫', 'Backspace', 'Backspace', 8, 1.5);
    d.cls = 'mk-tool';
    d.alt = { label: 'DEL', key: 'Delete', code: 'Delete', keyCode: 46 };
    return d;
  }

  function spaceDef(page) {
    var d = plainKey(page, 'space', ' ', 'Space', 32, 6);
    d.fixed = false;
    d.cls = 'mk-space';
    return d;
  }

  function enterDef(page) {
    var d = keyKey(page, 'enter', '↵', 'Enter', 'Enter', 13, 0.75, true);
    d.cls = 'mk-tool';
    d.svg = 'M5 6h14v12h-8M14.5 14.5L11 18l3.5 3.5';
    return d;
  }

  function digitRow(page) {
    var digits = [
      ['1', 'Digit1', 49], ['2', 'Digit2', 50], ['3', 'Digit3', 51], ['4', 'Digit4', 52], ['5', 'Digit5', 53],
      ['6', 'Digit6', 54], ['7', 'Digit7', 55], ['8', 'Digit8', 56], ['9', 'Digit9', 57], ['0', 'Digit0', 48]
    ];
    return digits.map(function (x) {
      return plainKey(page, 'd' + x[0], x[0], x[1], x[2]);
    });
  }

  function bottomRow(page, mode, label) {
    return [
      toolSwitch(page, 'mode', mode, label, 0.75),
      plainKey(page, 'comma', ',', 'Comma', 188),
      spaceDef(page),
      plainKey(page, 'period', '.', 'Period', 190),
      enterDef(page)
    ];
  }

  function buildPageDefs(layout) {
    var uk = layout === 'uk';
    var ck = charKey, pk = plainKey, kk = keyKey;
    return {
      abc: [
        [
          ck('abc', 'q', 'q', 'KeyQ', 81),
          ck('abc', 'w', 'w', 'KeyW', 87),
          ck('abc', 'e', 'e', 'KeyE', 69),
          ck('abc', 'r', 'r', 'KeyR', 82),
          ck('abc', 't', 't', 'KeyT', 84),
          ck('abc', 'y', 'y', 'KeyY', 89),
          ck('abc', 'u', 'u', 'KeyU', 85),
          ck('abc', 'i', 'i', 'KeyI', 73),
          ck('abc', 'o', 'o', 'KeyO', 79),
          ck('abc', 'p', 'p', 'KeyP', 80)
        ],
        [
          ck('abc', 'a', 'a', 'KeyA', 65),
          ck('abc', 's', 's', 'KeyS', 83),
          ck('abc', 'd', 'd', 'KeyD', 68),
          ck('abc', 'f', 'f', 'KeyF', 70),
          ck('abc', 'g', 'g', 'KeyG', 71),
          ck('abc', 'h', 'h', 'KeyH', 72),
          ck('abc', 'j', 'j', 'KeyJ', 74),
          ck('abc', 'k', 'k', 'KeyK', 75),
          ck('abc', 'l', 'l', 'KeyL', 76)
        ],
        [
          shiftDef('abc'),
          ck('abc', 'z', 'z', 'KeyZ', 90),
          ck('abc', 'x', 'x', 'KeyX', 88),
          ck('abc', 'c', 'c', 'KeyC', 67),
          ck('abc', 'v', 'v', 'KeyV', 86),
          ck('abc', 'b', 'b', 'KeyB', 66),
          ck('abc', 'n', 'n', 'KeyN', 78),
          ck('abc', 'm', 'm', 'KeyM', 77),
          bspDef('abc')
        ],
        bottomRow('abc', 'num', '?123')
      ],
      num: [
        digitRow('num'),
        (uk ? [
          pk('num', 'at', '@', 'Quote', 222),
          pk('num', 'hash', '#', 'Digit3', 50),
          pk('num', 'pound', '£', 'Digit3', 50),
          pk('num', 'underscore', '_', 'Minus', 189),
          pk('num', 'amp', '&', 'Digit7', 55),
          pk('num', 'minus', '-', 'Minus', 189),
          pk('num', 'plus', '+', 'Equal', 187),
          pk('num', 'parenleft', '(', 'Digit9', 57),
          pk('num', 'parenright', ')', 'Digit0', 48),
          slashDef('num')
        ] : [
          pk('num', 'at', '@', 'Digit2', 50),
          pk('num', 'hash', '#', 'Digit3', 50),
          pk('num', 'dollar', '$', 'Digit4', 52),
          pk('num', 'underscore', '_', 'Minus', 189),
          pk('num', 'amp', '&', 'Digit7', 55),
          pk('num', 'minus', '-', 'Minus', 189),
          pk('num', 'plus', '+', 'Equal', 187),
          pk('num', 'parenleft', '(', 'Digit9', 57),
          pk('num', 'parenright', ')', 'Digit0', 48),
          slashDef('num')
        ]),
        (uk ? [
          toolSwitch('num', 'sym', 'sym', '=\\<', 1.5),
          pk('num', 'star', '*', 'Digit8', 56),
          pk('num', 'dquote', '"', 'Digit2', 50),
          pk('num', 'apostrophe', "'", 'Quote', 222),
          pk('num', 'colon', ':', 'Semicolon', 186),
          pk('num', 'semicolon', ';', 'Semicolon', 186),
          pk('num', 'exclaim', '!', 'Digit1', 49),
          pk('num', 'question', '?', 'Slash', 191),
          bspDef('num')
        ] : [
          toolSwitch('num', 'sym', 'sym', '=\\<', 1.5),
          pk('num', 'equal', '=', 'Equal', 187),
          pk('num', 'backslash', '\\', 'Backslash', 220),
          pk('num', 'star', '*', 'Digit8', 56),
          pk('num', 'dquote', '"', 'Quote', 222),
          pk('num', 'apostrophe', "'", 'Quote', 222),
          pk('num', 'colon', ':', 'Semicolon', 186),
          pk('num', 'semicolon', ';', 'Semicolon', 186),
          pk('num', 'exclaim', '!', 'Digit1', 49),
          pk('num', 'question', '?', 'Slash', 191)
        ]),
        bottomRow('num', 'abc', 'ABC')
      ],
      sym: (uk ? [
        [
          pk('sym', 'tilde', '~', 'Backquote', 192),
          pk('sym', 'grave', '`', 'Backquote', 192),
          pk('sym', 'pipe', '|', 'Backslash', 220),
          pk('sym', 'bullet', '•', 'IntlBackslash', 226),
          pk('sym', 'root', '√', 'IntlBackslash', 226),
          pk('sym', 'pi', 'π', 'IntlBackslash', 226),
          pk('sym', 'divide', '÷', 'Slash', 191),
          pk('sym', 'times', '×', 'Digit8', 56),
          pk('sym', 'section', '§', 'Digit5', 53),
          pk('sym', 'delta', '∆', 'Digit4', 52)
        ],
        [
          pk('sym', 'euro', '€', 'Digit4', 52),
          pk('sym', 'yen', '¥', 'Digit5', 53),
          pk('sym', 'dollar', '$', 'Digit4', 52),
          pk('sym', 'cent', '¢', 'IntlBackslash', 226),
          pk('sym', 'caret', '^', 'Digit6', 54),
          pk('sym', 'degree', '°', 'IntlBackslash', 226),
          pk('sym', 'equal', '=', 'Equal', 187),
          pk('sym', 'lbrace', '{', 'BracketLeft', 219),
          pk('sym', 'rbrace', '}', 'BracketRight', 221),
          pk('sym', 'backslash', '\\', 'Backslash', 220)
        ],
        [
          toolSwitch('sym', 'num_btn', 'num', '?123', 1.5),
          pk('sym', 'percent', '%', 'Digit5', 53),
          pk('sym', 'copy', '©', 'IntlBackslash', 226),
          pk('sym', 'reg', '®', 'IntlBackslash', 226),
          pk('sym', 'tm', '™', 'IntlBackslash', 226),
          pk('sym', 'check', '✓', 'IntlBackslash', 226),
          pk('sym', 'bracketleft', '[', 'BracketLeft', 219),
          pk('sym', 'bracketright', ']', 'BracketRight', 221),
          bspDef('sym')
        ],
        [
          toolSwitch('sym', 'mode', 'abc', 'ABC', 0.75),
          pk('sym', 'less', '<', 'Comma', 188),
          spaceDef('sym'),
          pk('sym', 'greater', '>', 'Period', 190),
          enterDef('sym')
        ]
      ] : [
        [
          pk('sym', 'bracketleft', '[', 'BracketLeft', 219),
          pk('sym', 'bracketright', ']', 'BracketRight', 221),
          pk('sym', 'lbrace', '{', 'BracketLeft', 219),
          pk('sym', 'rbrace', '}', 'BracketRight', 221),
          pk('sym', 'hash', '#', 'Digit3', 50),
          pk('sym', 'percent', '%', 'Digit5', 53),
          pk('sym', 'caret', '^', 'Digit6', 54),
          pk('sym', 'star', '*', 'Digit8', 56),
          pk('sym', 'plus', '+', 'Equal', 187),
          pk('sym', 'equal', '=', 'Equal', 187)
        ],
        [
          pk('sym', 'underscore', '_', 'Minus', 189),
          pk('sym', 'backslash', '\\', 'Backslash', 220),
          pk('sym', 'pipe', '|', 'Backslash', 220),
          pk('sym', 'tilde', '~', 'Backquote', 192),
          pk('sym', 'less', '<', 'Comma', 188),
          pk('sym', 'greater', '>', 'Period', 190),
          pk('sym', 'euro', '€', 'Digit4', 52),
          pk('sym', 'pound', '£', 'Digit3', 50),
          pk('sym', 'yen', '¥', 'Digit5', 53),
          pk('sym', 'bullet', '•', 'IntlBackslash', 226)
        ],
        digitRow('sym'),
        bottomRow('sym', 'num', '?123')
      ])
    };
  }

  function buildKey(def) {
    var b = document.createElement('button');
    b.type = 'button';
    b.className = 'mk-key';
    if (def.cls) b.className += ' ' + def.cls;
    b.dataset.id = def.id;
    if (def.fixed) {
      b.className += ' mk-fixed';
    } else {
      b.style.flexGrow = def.w || 1;
    }

    if (def.kind === 'char') {
      var main = document.createElement('span');
      main.className = 'mk-main';
      main.textContent = def.char;
      b.appendChild(main);
    } else if (def.svg) {
      var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      svg.setAttribute('viewBox', '0 0 24 24');
      svg.setAttribute('class', 'mk-key-svg' + (def.svgFill ? ' mk-key-svg-fill' : ''));
      svg.setAttribute('aria-hidden', 'true');
      var path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      path.setAttribute('d', def.svg);
      svg.appendChild(path);
      b.appendChild(svg);
    } else {
      var m = document.createElement('span');
      m.className = 'mk-main';
      m.textContent = def.label;
      b.appendChild(m);
    }
    if (def.longHint) {
      var lp = document.createElement('span');
      lp.className = 'mk-lp';
      lp.textContent = def.longHint;
      b.appendChild(lp);
    }
    return b;
  }

  var TEMPLATE = document.createElement('template');
  TEMPLATE.innerHTML =
    '<style>' +
    ':host{' +
    'display:block;width:100%;' +
    '--mk-bg:#161b22;--mk-gap:3px;' +
    '--mk-key-height:clamp(30px,6vh,42px);' +
    '--mk-key-unit:calc((100% - var(--mk-gap)*9)/10);' +
    '--mk-extra-height:clamp(25px,4.6vh,34px);' +
    '--mk-extra-unit:calc((100% - var(--mk-gap)*6)/7);' +
    '--mk-key-bg:#21262d;--mk-key-color:#e6edf3;--mk-key-sub:#8b949e;' +
    '--mk-key-border:#30363d;' +
    '--mk-tool-bg:#30363d;--mk-tool-color:#e6edf3;--mk-tool-border:#363b43;' +
    '--mk-active:#388bfd;--mk-active-color:#ffffff;--mk-locked:#2ea043;' +
    '--mk-extra-bg:#1c3044;--mk-extra-key:#27415c;--mk-extra-border:#33577a;' +
    '--mk-extra-color:#dbe6f0;' +
    '--mk-sep:#0b0e13;' +
    '--mk-radius:8px;' +
    '--mk-font:system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;' +
    '}' +
    '.gb{box-sizing:border-box;width:100%;' +
    'display:flex;flex-direction:column;' +
    'touch-action:none;user-select:none;-webkit-user-select:none;}' +
    '.mk-extra{background:var(--mk-extra-bg);padding:5px 6px 4px;' +
    'display:flex;flex-direction:column;gap:var(--mk-gap);' +
    'border-radius:12px 12px 0 0;}' +
    '.mk-sep{height:1px;background:var(--mk-sep);}' +
    '.mk-area{flex:1;min-height:0;display:flex;flex-direction:column;' +
    'gap:var(--mk-gap);padding:5px 6px 6px;background:var(--mk-bg);' +
    'border-radius:0 0 12px 12px;}' +
    '.mk-collapse{display:none;height:var(--mk-key-height);min-height:0;' +
    'background:var(--mk-extra-bg);border:1px solid var(--mk-extra-border);' +
    'border-radius:12px;align-items:center;justify-content:center;cursor:pointer;' +
    'touch-action:none;user-select:none;-webkit-user-select:none;}' +
    '.mk-collapse-icon{width:26px;height:26px;fill:var(--mk-extra-color);' +
    'opacity:0.7;pointer-events:none;}' +
    '.gb.collapsed .mk-extra,.gb.collapsed .mk-sep,.gb.collapsed .mk-area{display:none;}' +
    '.gb.collapsed .mk-collapse{display:flex;}' +
    '.mk-pages{flex:1;display:flex;flex-direction:column;gap:var(--mk-gap);' +
    'min-height:calc(var(--mk-key-height)*4 + var(--mk-gap)*3);}' +
    '.mk-page{flex:1;display:flex;flex-direction:column;gap:var(--mk-gap);min-height:0;}' +
    '.mk-page[hidden]{display:none;}' +
    '.mk-row{flex:0 0 auto;display:flex;gap:var(--mk-gap);justify-content:center;}' +
    '.mk-page .mk-key{height:var(--mk-key-height);}' +
    '.mk-page .mk-fixed{flex:0 0 auto;width:var(--mk-key-unit);}' +
    '.mk-extra .mk-key{height:var(--mk-extra-height);}' +
    '.mk-extra .mk-fixed{flex:0 0 auto;width:var(--mk-extra-unit);}' +
    '.mk-key{flex:0 0 0;min-width:0;border:1px solid var(--mk-key-border);' +
    'border-radius:var(--mk-radius);background:var(--mk-key-bg);color:var(--mk-key-color);' +
    'font-family:var(--mk-font);font-size:clamp(21px,5.8vw,34px);font-weight:500;padding:0;margin:0;' +
    'display:flex;align-items:center;justify-content:center;position:relative;cursor:pointer;' +
    'touch-action:none;user-select:none;-webkit-user-select:none;-webkit-tap-highlight-color:transparent;' +
    'outline:none;box-shadow:inset 0 -2px 0 rgba(0,0,0,0.35);}' +
    '.mk-key.mk-tool{background:var(--mk-tool-bg);color:var(--mk-tool-color);' +
    'border-color:var(--mk-tool-border);}' +
    '.mk-extra .mk-key{background:var(--mk-extra-key);color:var(--mk-extra-color);' +
    'border-color:var(--mk-extra-border);font-size:clamp(12px,3.2vw,17px);font-weight:600;}' +
    '.mk-key:active,.mk-key.mk-pressed{background:var(--mk-active);color:var(--mk-active-color);' +
    'transform:translateY(1px);box-shadow:none;}' +
    '.mk-key.mk-mod-active{background:var(--mk-active);color:var(--mk-active-color);box-shadow:none;}' +
    '.mk-key.mk-mod-locked{background:var(--mk-locked);color:#fff;box-shadow:none;}' +
    '.mk-key.mk-alt{background:var(--mk-active);color:var(--mk-active-color);box-shadow:none;}' +
    '.mk-page.num .mk-key,.mk-page.sym .mk-key{font-size:clamp(18px,4.8vw,26px);}' +
    '.mk-main{pointer-events:none;}' +
    '.mk-lp{position:absolute;top:1px;right:3px;font-size:9px;line-height:1;' +
    'font-weight:600;opacity:0.55;pointer-events:none;}' +
    '.mk-key-svg{width:26px;height:26px;fill:none;stroke:currentColor;' +
    'stroke-width:2.5;stroke-linecap:round;stroke-linejoin:round;pointer-events:none;}' +
    '.mk-key-svg-fill{fill:currentColor;stroke:none;}' +
    '</style>' +
    '<div class="gb">' +
    '<div class="mk-extra"></div>' +
    '<div class="mk-sep"></div>' +
    '<div class="mk-area">' +
    '<div class="mk-pages"></div>' +
    '</div>' +
    '<div class="mk-collapse">' +
    '<svg class="mk-collapse-icon" viewBox="0 0 24 24" aria-hidden="true">' +
    '<path d="M20 5H4c-1.1 0-1.99.9-1.99 2L2 17c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm-9 3h2v2h-2V8zm0 3h2v2h-2v-2zM8 8h2v2H8V8zm0 3h2v2H8v-2zm-1 2H5v-2h2v2zm0-3H5V8h2v2zm9 7H8v-2h8v2zm0-4h-2v-2h2v2zm0-3h-2V8h2v2zm3 3h-2v-2h2v2zm0-3h-2V8h2v2z"/></svg>' +
    '</div>' +
    '</div>';

  function extSlashDef() {
    var d = plainKey('ext', 'slash', '/', 'Slash', 191);
    d.alt = { char: '\\', code: 'Backslash', keyCode: 220 };
    return d;
  }

  function escDef() {
    var d = keyKey('ext', 'esc', 'ESC', 'Escape', 'Escape', 27, 1, true);
    d.long = 'frow';
    d.longHint = 'Fn';
    return d;
  }

  function tabDef() {
    var d = keyKey('ext', 'tab', 'TAB', 'Tab', 'Tab', 9, 1, true);
    d.long = 'shiftTab';
    d.longHint = 'S⇥';
    return d;
  }

  function fKey(i) {
    var n = '' + i;
    return keyKey('ext', 'f' + n, 'F' + n, 'F' + n, 'F' + n, 111 + i, 1, true);
  }

  function buildFDefs() {
    var rows = [
      [
        escDef(),
        fKey(1), fKey(2), fKey(3), fKey(4), fKey(5),
        fKey(6), fKey(7), fKey(8), fKey(9)
      ],
      [
        fKey(10), fKey(11), fKey(12),
        tabDef(),
        modKey('ext', 'ctrl', 'ctrl', 'CTRL', 'Control', 'Control', 17),
        modKey('ext', 'alt', 'alt', 'ALT', 'Alt', 'Alt', 18),
        keyKey('ext', 'left', '◀', 'ArrowLeft', 'ArrowLeft', 37, 1, true),
        keyKey('ext', 'down', '▼', 'ArrowDown', 'ArrowDown', 40, 1, true),
        keyKey('ext', 'right', '▶', 'ArrowRight', 'ArrowRight', 39, 1, true),
        keyKey('ext', 'pgdn', 'PGDN', 'PageDown', 'PageDown', 34, 1, true)
      ]
    ];
    rows.forEach(function (row) {
      row.forEach(function (d) {
        d.fixed = false;
        d.w = 1;
      });
    });
    return rows;
  }

  function buildExtraDefs() {
    var rows = [
      [
        escDef(),
        extSlashDef(),
        plainKey('ext', 'minus', '-', 'Minus', 189),
        keyKey('ext', 'home', 'HOME', 'Home', 'Home', 36, 1, true),
        keyKey('ext', 'up', '▲', 'ArrowUp', 'ArrowUp', 38, 1, true),
        keyKey('ext', 'end', 'END', 'End', 'End', 35, 1, true),
        keyKey('ext', 'pgup', 'PGUP', 'PageUp', 'PageUp', 33, 1, true)
      ],
      [
        tabDef(),
        modKey('ext', 'ctrl', 'ctrl', 'CTRL', 'Control', 'Control', 17),
        modKey('ext', 'alt', 'alt', 'ALT', 'Alt', 'Alt', 18),
        keyKey('ext', 'left', '◀', 'ArrowLeft', 'ArrowLeft', 37, 1, true),
        keyKey('ext', 'down', '▼', 'ArrowDown', 'ArrowDown', 40, 1, true),
        keyKey('ext', 'right', '▶', 'ArrowRight', 'ArrowRight', 39, 1, true),
        keyKey('ext', 'pgdn', 'PGDN', 'PageDown', 'PageDown', 34, 1, true)
      ]
    ];
    rows.forEach(function (row) {
      row.forEach(function (d) { d.fixed = true; });
    });
    return rows;
  }

  class MobileKeyboard extends HTMLElement {
    constructor() {
      super();
      this.attachShadow({ mode: 'open' });
      this._defs = {};
      this._els = {};
      this._keyPointers = new Map();
      this._downState = new Map();
      this._mods = { shift: false, ctrl: false, alt: false };
      this._sticky = { shift: 'none', ctrl: 'none', alt: 'none' };
      this._heldMods = { shift: 0, ctrl: 0, alt: 0 };
      this._modChord = { shift: false, ctrl: false, alt: false };
      this._modDownAt = { shift: 0, ctrl: 0, alt: 0 };
      this._modActiveAtDown = { shift: false, ctrl: false, alt: false };
      this._lastTap = { shift: 0, ctrl: 0, alt: 0 };
      this._modDefs = {};
      this._frow = false;
      this._repeatTimer = null;
      this._repeatInt = null;
      this._repeatDef = null;
      this._repeatEnabled = true;
      this._layout = 'uk';
      this._mode = 'abc';
      this._pages = {};
      this._wrap = null;
      this._touchPointers = new Map();
      this._twoFinger = false;
      this._twoStart = null;
      this._twoTriggered = false;
      this._onWindowUp = this._onWindowUp.bind(this);
      this._onWindowMove = this._onWindowMove.bind(this);
      this._onBlur = this._releaseAll.bind(this);
    }

    static get observedAttributes() { return ['layout', 'repeat', 'mode']; }

    attributeChangedCallback(name, oldV, newV) {
      if (name === 'layout') {
        this._layout = this._normLayout(newV);
        if (this._wrap) {
          this._build();
          var m = this._normMode(this.getAttribute('mode'));
          if (m && m !== this._mode) this._setMode(m);
        }
      } else if (name === 'repeat') {
        this._repeatEnabled = newV !== 'off' && newV !== 'false';
      } else if (name === 'mode') {
        if (this._wrap) {
          var m2 = this._normMode(newV);
          if (m2) this._setMode(m2);
        }
      }
    }

    _normLayout(v) {
      v = String(v || 'uk').toLowerCase();
      return v === 'uk' || v === 'gb' ? 'uk' : 'us';
    }

    _normMode(v) {
      v = String(v || '').toLowerCase();
      return v === 'num' || v === 'sym' ? v : (v === 'abc' ? 'abc' : null);
    }

    connectedCallback() {
      var rp = this.getAttribute('repeat');
      this._layout = this._normLayout(this.getAttribute('layout'));
      this._repeatEnabled = rp !== 'off' && rp !== 'false';
      this._build();
      var m = this._normMode(this.getAttribute('mode'));
      if (m && m !== this._mode) this._setMode(m);
      window.addEventListener('pointerup', this._onWindowUp);
      window.addEventListener('pointercancel', this._onWindowUp);
      window.addEventListener('pointermove', this._onWindowMove);
      window.addEventListener('blur', this._onBlur);
      window.addEventListener('visibilitychange', this._onBlur);
    }

    disconnectedCallback() {
      this._releaseAll();
      window.removeEventListener('pointerup', this._onWindowUp);
      window.removeEventListener('pointercancel', this._onWindowUp);
      window.removeEventListener('pointermove', this._onWindowMove);
      window.removeEventListener('blur', this._onBlur);
      window.removeEventListener('visibilitychange', this._onBlur);
    }

    reset() {
      this._releaseAll();
    }

    get mode() {
      return this._mode;
    }

    get modifiers() {
      return { shift: this._mods.shift, ctrl: this._mods.ctrl, alt: this._mods.alt, caps: this._sticky.shift === 'locked' };
    }

    _build() {
      var root = this.shadowRoot;
      root.innerHTML = '';
      root.appendChild(TEMPLATE.content.cloneNode(true));

      var wrap = root.querySelector('.gb');
      this._wrap = wrap;
      this._defs = {};
      this._els = {};
      this._pages = {};

      var self = this;
      var extraRoot = root.querySelector('.mk-extra');
      var extraRows = buildExtraDefs();
      for (var er = 0; er < extraRows.length; er++) {
        var erow = document.createElement('div');
        erow.className = 'mk-row';
        this._appendKeys(erow, extraRows[er]);
        extraRoot.appendChild(erow);
      }

      var pagesRoot = root.querySelector('.mk-pages');
      var defs = buildPageDefs(this._layout);
      var pageName;
      for (pageName in defs) {
        var pageDiv = document.createElement('div');
        pageDiv.className = 'mk-page ' + pageName;
        pageDiv.dataset.page = pageName;
        var rows = defs[pageName];
        for (var r = 0; r < rows.length; r++) {
          var row = document.createElement('div');
          row.className = 'mk-row';
          var ds = rows[r];
          for (var i = 0; i < ds.length; i++) {
            var el = buildKey(ds[i]);
            row.appendChild(el);
            this._defs[ds[i].id] = ds[i];
            this._els[ds[i].id] = el;
          }
          pageDiv.appendChild(row);
        }
        pagesRoot.appendChild(pageDiv);
        this._pages[pageName] = pageDiv;
      }

      this._modDefs = {};
      for (var id in this._defs) {
        var d = this._defs[id];
        if (d.kind === 'mod' && !this._modDefs[d.mod]) this._modDefs[d.mod] = d;
      }

      wrap.addEventListener('pointerdown', function (e) { self._onDown(e); });
      wrap.addEventListener('contextmenu', function (e) { e.preventDefault(); });
      var collapseBar = root.querySelector('.mk-collapse');
      collapseBar.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        e.stopPropagation();
        self._expand();
      });

      this._mode = 'abc';
      for (var p in this._pages) this._pages[p].hidden = (p !== 'abc');
      this._sync();
    }

    _appendKeys(container, defs) {
      for (var i = 0; i < defs.length; i++) {
        var el = buildKey(defs[i]);
        container.appendChild(el);
        this._defs[defs[i].id] = defs[i];
        this._els[defs[i].id] = el;
      }
    }

    _setMode(m) {
      if (!this._pages[m] || m === this._mode) return;
      this._mode = m;
      for (var p in this._pages) this._pages[p].hidden = (p !== m);
      this.dispatchEvent(new CustomEvent('modechange', { detail: { mode: m }, bubbles: true, composed: true }));
    }

    _onDown(e) {
      this._touchPointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (!this._twoFinger && this._touchPointers.size >= 2) this._beginTwoFinger();
      var el = e.target.closest('.mk-key');
      if (!el) return;
      e.preventDefault();
      try { el.setPointerCapture(e.pointerId); } catch (err) {}
      var def = this._defs[el.dataset.id];
      if (!def) return;
      var set = this._keyPointers.get(def.id);
      if (!set) {
        set = new Set();
        this._keyPointers.set(def.id, set);
      }
      if (set.size === 0) {
        if (def.kind === 'mod') {
          this._modDown(def);
        } else {
          this._downKey(def, el, e);
        }
      }
      set.add(e.pointerId);
      el.classList.add('mk-pressed');
    }

    _downKey(def, el, e) {
      this._markChord();
      var self = this;
      var entry = {
        def: def,
        el: el,
        x: e.clientX,
        y: e.clientY,
        swiped: false,
        repeating: false,
        longpressed: false,
        timer: null
      };
      this._downState.set(e.pointerId, entry);
      if (def.alt) {
        entry.timer = setTimeout(function () {
          entry.timer = null;
          if (!self._downState.get(e.pointerId)) return;
          entry.longpressed = true;
          self._toggleChar(def);
        }, 450);
      } else if (def.long) {
        entry.timer = setTimeout(function () {
          entry.timer = null;
          if (!self._downState.get(e.pointerId)) return;
          entry.longpressed = true;
          self._longPress(def);
        }, 450);
      } else if (this._repeatEnabled && !def.noRepeat && this._isRepeatable(def)) {
        entry.timer = setTimeout(function () {
          entry.timer = null;
          entry.repeating = true;
          self._repeatInt = setInterval(function () {
            var set = self._keyPointers.get(def.id);
            if (set && set.size > 0) {
              self._emit(def, 'repeat');
            } else {
              self._stopRepeat();
            }
          }, 80);
        }, 450);
      }
    }

    _isRepeatable(def) {
      return def.kind === 'key' &&
        ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown',
         'Home', 'End', 'PageUp', 'PageDown'].indexOf(def.key) >= 0;
    }

    _onWindowMove(e) {
      var tp = this._touchPointers.get(e.pointerId);
      if (tp) { tp.x = e.clientX; tp.y = e.clientY; }
      if (this._twoFinger) {
        if (!this._twoTriggered && this._touchPointers.size >= 2) {
          var sx = 0, sy = 0;
          this._touchPointers.forEach(function (p) { sx += p.x; sy += p.y; });
          var n = this._touchPointers.size;
          var dx = sx / n - this._twoStart.x;
          var dy = sy / n - this._twoStart.y;
          if (dy > 0 && dy >= Math.abs(dx) && dx * dx + dy * dy >= SWIPE_THRESHOLD2) {
            this._twoTriggered = true;
            this._collapse();
          }
        }
        return;
      }
      if (!this._downState) return;
      var entry = this._downState.get(e.pointerId);
      if (!entry || entry.swiped) return;
      var dx2 = e.clientX - entry.x;
      var dy2 = e.clientY - entry.y;
      if (dx2 * dx2 + dy2 * dy2 >= SWIPE_THRESHOLD2) {
        entry.swiped = true;
        entry.sx = e.clientX;
        entry.sy = e.clientY;
        if (entry.el) entry.el.classList.remove('mk-pressed');
        if (entry.timer) { clearTimeout(entry.timer); entry.timer = null; }
        if (entry.repeating) { this._stopRepeat(); entry.repeating = false; }
      }
    }

    _onWindowUp(e) {
      this._touchPointers.delete(e.pointerId);
      if (this._touchPointers.size < 2) {
        this._twoFinger = false;
        this._twoStart = null;
        this._twoTriggered = false;
      }
      this._upPointer(e.pointerId);
    }

    _beginTwoFinger() {
      var self = this;
      this._twoFinger = true;
      this._twoTriggered = false;
      var sx = 0, sy = 0;
      this._touchPointers.forEach(function (p) { sx += p.x; sy += p.y; });
      var n = this._touchPointers.size;
      this._twoStart = { x: sx / n, y: sy / n };
      this._downState.forEach(function (entry) {
        entry.swiped = true;
        entry.suppressSwipe = true;
        if (entry.timer) { clearTimeout(entry.timer); entry.timer = null; }
        if (entry.repeating) { self._stopRepeat(); entry.repeating = false; }
        if (entry.el) entry.el.classList.remove('mk-pressed');
      });
    }

    _collapse() {
      this._vibrate(25);
      this._releaseAll();
      if (this._wrap) this._wrap.classList.add('collapsed');
    }

    _expand() {
      this._vibrate(15);
      if (this._wrap) this._wrap.classList.remove('collapsed');
    }

    _upPointer(pid) {
      var self = this;
      var entry = this._downState.get(pid);
      if (entry) {
        this._downState.delete(pid);
        if (entry.swiped) {
          if (!entry.suppressSwipe) this._emitSwipe(entry);
        } else {
          if (entry.timer) { clearTimeout(entry.timer); entry.timer = null; }
          if (entry.repeating) {
            this._stopRepeat();
          } else if (!entry.longpressed) {
            this._tapKey(entry.def);
          }
          this._sync();
        }
      }
      this._keyPointers.forEach(function (set, id) {
        if (set.has(pid)) {
          set.delete(pid);
          var el = self._els[id];
          if (el) el.classList.remove('mk-pressed');
          if (set.size === 0) {
            self._keyPointers.delete(id);
            var def = self._defs[id];
            if (def && def.kind === 'mod') self._modUp(def);
            else self._stopRepeat();
          }
        }
      });
    }

    _tapKey(def) {
      this._vibrate(15);
      if (def.kind === 'switch') {
        this._setMode(def.mode);
      } else {
        this._emit(def, 'press');
      }
      this._consumeOneshots();
      this._sync();
    }

    _vibrate(ms) {
      try {
        if (navigator.vibrate) navigator.vibrate(ms);
      } catch (err) {}
    }

    _toggleChar(def) {
      this._vibrate(12);
      if (def.kind === 'char') {
        var old = { char: def.char, code: def.code, keyCode: def.keyCode };
        def.char = def.alt.char;
        def.code = def.alt.code;
        def.keyCode = def.alt.keyCode;
        def.alt = old;
      } else if (def.kind === 'key') {
        var old = { label: def.label, key: def.key, code: def.code, keyCode: def.keyCode };
        def.label = def.alt.label;
        def.key = def.alt.key;
        def.code = def.alt.code;
        def.keyCode = def.alt.keyCode;
        def.alt = old;
      }
      def.alternate = !def.alternate;
      var el = this._els[def.id];
      if (el) {
        var main = el.querySelector('.mk-main');
        if (main) main.textContent = def.kind === 'char' ? def.char : def.label;
        el.classList.toggle('mk-alt', !!def.alternate);
      }
    }

    _longPress(def) {
      if (def.long === 'shiftTab') {
        this._emit(def, 'press', { shiftKey: true });
      } else if (def.long === 'frow') {
        this._toggleFrow();
      }
    }

    _toggleFrow() {
      this._frow = !this._frow;
      var root = this.shadowRoot;
      var extraRoot = root.querySelector('.mk-extra');
      extraRoot.innerHTML = '';
      var rows = this._frow ? buildFDefs() : buildExtraDefs();
      for (var i = 0; i < rows.length; i++) {
        var row = document.createElement('div');
        row.className = 'mk-row';
        this._appendKeys(row, rows[i]);
        extraRoot.appendChild(row);
      }
      var esc = this._els['ext_esc'];
      if (esc) esc.classList.toggle('mk-alt', !!this._frow);
    }

    _emitSwipe(entry) {
      var dx = entry.sx - entry.x;
      var dy = entry.sy - entry.y;
      var key, code, keyCode;
      if (Math.abs(dx) >= Math.abs(dy)) {
        if (dx >= 0) { key = 'ArrowRight'; code = 'ArrowRight'; keyCode = 39; }
        else { key = 'ArrowLeft'; code = 'ArrowLeft'; keyCode = 37; }
      } else {
        if (dy >= 0) { key = 'ArrowDown'; code = 'ArrowDown'; keyCode = 40; }
        else { key = 'ArrowUp'; code = 'ArrowUp'; keyCode = 38; }
      }
      var m = this._mods;
      var d = {
        type: 'key', action: 'swipe', key: key, code: code, keyCode: keyCode, char: null,
        shiftKey: m.shift, ctrlKey: m.ctrl, altKey: m.alt, metaKey: false, fnKey: false,
        capsLock: this._sticky.shift === 'locked', repeat: false
      };
      this.dispatchEvent(new CustomEvent('keyinput', { detail: d, bubbles: true, composed: true }));
    }

    _modDown(def) {
      var mod = def.mod;
      this._heldMods[mod]++;
      this._modChord[mod] = false;
      this._modDownAt[mod] = performance.now();
      this._modActiveAtDown[mod] = this._sticky[mod] !== 'none';
      this._mods[mod] = true;
      this._emit(def, 'down');
      this._sync();
    }

    _modUp(def) {
      var mod = def.mod;
      var held = Math.max(0, this._heldMods[mod] - 1);
      this._heldMods[mod] = held;
      if (held > 0) return;
      var now = performance.now();
      var duration = now - this._modDownAt[mod];

      if (this._modChord[mod]) {
        this._mods[mod] = false;
        this._sticky[mod] = 'none';
        this._emit(def, 'up');
        this._sync();
        return;
      }
      if (duration < 300 && now - this._lastTap[mod] < 350) {
        this._sticky[mod] = 'locked';
        this._mods[mod] = true;
        this._emit(def, 'lock');
        this._lastTap[mod] = now;
      } else if (this._modActiveAtDown[mod]) {
        this._mods[mod] = false;
        this._sticky[mod] = 'none';
        this._emit(def, 'up');
        this._lastTap[mod] = 0;
      } else {
        this._sticky[mod] = 'oneshot';
        this._mods[mod] = true;
        this._emit(def, 'latch');
        if (duration < 300) this._lastTap[mod] = now;
      }
      this._sync();
    }

    _markChord() {
      for (var mod in this._heldMods) {
        if (this._heldMods[mod] > 0) this._modChord[mod] = true;
      }
    }

    _consumeOneshots() {
      for (var mod in this._sticky) {
        if (this._sticky[mod] === 'oneshot' && this._mods[mod] && this._heldMods[mod] === 0) {
          this._mods[mod] = false;
          this._sticky[mod] = 'none';
          this._lastTap[mod] = 0;
          var d = this._modDefs[mod];
          if (d) this._emit(d, 'up');
        }
      }
    }

    _charOf(def) {
      if (/^[a-z]$/.test(def.char)) {
        return this._mods.shift ? def.char.toUpperCase() : def.char;
      }
      return def.char;
    }

    _emit(def, action, extra) {
      var m = this._mods;
      var d = {
        type: def.kind,
        action: action,
        key: def.key || (def.kind === 'char' ? def.char : def.label),
        code: def.code,
        keyCode: def.keyCode,
        char: null,
        shiftKey: m.shift,
        ctrlKey: m.ctrl,
        altKey: m.alt,
        metaKey: false,
        fnKey: false,
        capsLock: this._sticky.shift === 'locked',
        repeat: action === 'repeat'
      };
      if (def.kind === 'mod') {
        d.mod = def.mod;
        d.value = m[def.mod];
      } else if (def.kind === 'char') {
        var c = this._charOf(def);
        d.char = c;
        d.key = c;
      }
      if (extra) {
        for (var k in extra) d[k] = extra[k];
      }
      this.dispatchEvent(new CustomEvent('keyinput', { detail: d, bubbles: true, composed: true }));
    }

    _stopRepeat() {
      if (this._repeatTimer) { clearTimeout(this._repeatTimer); this._repeatTimer = null; }
      if (this._repeatInt) { clearInterval(this._repeatInt); this._repeatInt = null; }
      this._repeatDef = null;
    }

    _releaseAll() {
      this._stopRepeat();
      this._touchPointers = new Map();
      this._twoFinger = false;
      this._twoStart = null;
      this._twoTriggered = false;
      if (this._downState) {
        this._downState.forEach(function (entry) { if (entry.timer) clearTimeout(entry.timer); });
        this._downState = new Map();
      }
      this._keyPointers = new Map();
      this._mods = { shift: false, ctrl: false, alt: false };
      this._sticky = { shift: 'none', ctrl: 'none', alt: 'none' };
      this._heldMods = { shift: 0, ctrl: 0, alt: 0 };
      this._modChord = { shift: false, ctrl: false, alt: false };
      this._modActiveAtDown = { shift: false, ctrl: false, alt: false };
      this._lastTap = { shift: 0, ctrl: 0, alt: 0 };
      var els = this._els;
      for (var id in els) {
        if (els[id]) els[id].classList.remove('mk-pressed');
      }
      this._sync();
    }

    _sync() {
      if (!this._wrap) return;
      var shift = this._mods.shift;
      for (var id in this._els) {
        var el = this._els[id];
        if (!el) continue;
        var def = this._defs[id];
        if (!def) continue;
        if (def.kind === 'char') {
          var main = el.querySelector('.mk-main');
          if (!main) continue;
          if (/^[a-z]$/.test(def.char)) {
            main.textContent = shift ? def.char.toUpperCase() : def.char;
          }
        } else if (def.kind === 'mod') {
          el.classList.toggle('mk-mod-active', this._mods[def.mod]);
          el.classList.toggle('mk-mod-locked', this._sticky[def.mod] === 'locked');
        }
      }
    }
  }

  if (!customElements.get('mobile-keyboard')) {
    customElements.define('mobile-keyboard', MobileKeyboard);
  }
})();
